use crate::{
    algebra::{StorageExt, TriggerActor},
    domain::{Refresh, Trigger, Unit},
};
use actix::prelude::*;
use chrono::{Duration, Utc};
use integrationos_domain::{
    algebra::MongoStore, client::secrets_client::SecretsClient,
    connection_oauth_definition::ConnectionOAuthDefinition, error::IntegrationOSError as Error,
    Connection, InternalError,
};
use reqwest::Client;
use std::sync::Arc;

pub struct RefreshActor {
    connections: Arc<MongoStore<Connection>>,
    oauths: Arc<MongoStore<ConnectionOAuthDefinition>>,
    secrets: Arc<SecretsClient>,
    client: Client,
}

impl RefreshActor {
    pub fn new(
        oauths: Arc<MongoStore<ConnectionOAuthDefinition>>,
        connections: Arc<MongoStore<Connection>>,
        secrets: Arc<SecretsClient>,
        client: Client,
    ) -> Self {
        Self {
            connections,
            oauths,
            secrets,
            client,
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
            refresh_before.timestamp(),
            refresh_after.timestamp()
        );

        let secrets = self.secrets.clone();
        let client = self.client.clone();
        let connections_store = self.connections.clone();
        let oauths_store = self.oauths.clone();

        Box::pin(async move {
            tracing::info!("Searching for connections to refresh");
            let connections = connections_store
                .get_by(&refresh_before, &refresh_after)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to get connections to refresh: {:?}", e);
                    e
                })?;

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
