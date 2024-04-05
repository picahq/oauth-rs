use crate::prelude::{
    get_connections_to_refresh, Query, Refresh, StatefulActor, Trigger, TriggerActor, Unit,
};
use actix::prelude::*;
use chrono::{Duration, Utc};
use futures::lock::Mutex;
use integrationos_domain::{
    connection_oauth_definition::ConnectionOAuthDefinition, error::IntegrationOSError as Error,
    mongo::MongoDbStore, service::secrets_client::SecretsClient, Connection, InternalError,
};
use reqwest::Client;
use std::sync::Arc;

pub struct RefreshActor {
    connections: Arc<MongoDbStore<Connection>>,
    oauths: Arc<MongoDbStore<ConnectionOAuthDefinition>>,
    secrets: Arc<SecretsClient>,
    client: Client,
    state: Arc<Mutex<StatefulActor>>,
}

impl RefreshActor {
    pub fn new(
        oauths: Arc<MongoDbStore<ConnectionOAuthDefinition>>,
        connections: Arc<MongoDbStore<Connection>>,
        secrets: Arc<SecretsClient>,
        client: Client,
    ) -> Self {
        Self {
            connections,
            oauths,
            secrets,
            client,
            state: StatefulActor::empty(),
        }
    }
}

impl Actor for RefreshActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        tracing::info!("RefreshActor started with id {:?}", ctx.address());
    }
}

impl Supervised for RefreshActor {}

impl Handler<Refresh> for RefreshActor {
    type Result = ResponseFuture<Result<Unit, Error>>;

    fn handle(&mut self, msg: Refresh, _: &mut Self::Context) -> Self::Result {
        let refresh_before = Utc::now();
        let refresh_after = refresh_before + Duration::minutes(msg.refresh_before_in_minutes());
        tracing::info!(
            "Searching for connections to refresh between {} and {}",
            refresh_before,
            refresh_after
        );

        let secrets = self.secrets.clone();
        let client = self.client.clone();
        let connections_store = self.connections.clone();
        let oauths_store = self.oauths.clone();
        let state = self.state.clone();

        Box::pin(async move {
            let connections =
                get_connections_to_refresh(&connections_store, &refresh_before, &refresh_after)
                    .await?;

            tracing::info!("Found {} connections to refresh", connections.len());

            let mut requests = vec![];
            for connection in &connections {
                let trigger_message = Trigger::new(connection.clone());
                let actor = TriggerActor::new(
                    connections_store.clone(),
                    oauths_store.clone(),
                    secrets.clone(),
                    client.clone(),
                    None,
                )
                .start();
                let future = actor.send(trigger_message);
                requests.push(future);
            }

            match futures::future::join_all(requests)
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()
            {
                Ok(vec) => {
                    let vec_as_json = serde_json::to_value(&vec).map_err(|e| {
                        InternalError::encryption_error(
                            "Failed to serialize outcome",
                            Some(e.to_string().as_str()),
                        )
                    })?;
                    StatefulActor::update(vec_as_json, state).await;

                    tracing::info!(
                        "Refreshed {} connections with outcome: {:?}",
                        vec.len(),
                        vec
                    );

                    Ok(())
                }
                Err(err) => Err(InternalError::io_err(err.to_string().as_str(), None)),
            }
        })
    }
}

impl Handler<Query> for RefreshActor {
    type Result = ResponseFuture<StatefulActor>;

    fn handle(&mut self, _: Query, _: &mut Self::Context) -> Self::Result {
        let state = self.state.clone();

        Box::pin(async move { state.lock().await.clone() })
    }
}
