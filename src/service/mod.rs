mod configuration;
mod http;

pub use configuration::*;
pub use http::*;

use crate::prelude::RefreshActor;
use actix::{Addr, Supervisor};
use integrationos_domain::{
    connection_oauth_definition::ConnectionOAuthDefinition, error::IntegrationOSError as Error,
    event_access::EventAccess, mongo::MongoDbStore, service::secrets_client::SecretsClient,
    Connection, InternalError, Store,
};
use moka::future::Cache;
use mongodb::options::FindOptions;
use reqwest::{header::HeaderValue, Client};
use std::{sync::Arc, time::Duration};
use tokio::time::timeout;

#[derive(Clone, Debug)]
pub struct AppState {
    configuration: Config,
    cache: Cache<HeaderValue, Arc<EventAccess>>,
    client: Client,
    secrets: Arc<SecretsClient>,
    connections: Arc<MongoDbStore<Connection>>,
    oauths: Arc<MongoDbStore<ConnectionOAuthDefinition>>,
    event_access: Arc<MongoDbStore<EventAccess>>,
    refresh_actor: Addr<RefreshActor>,
}

impl AppState {
    pub async fn try_from(config: Config) -> Result<Self, Error> {
        let client = Client::builder()
            .timeout(Duration::from_millis(config.server().timeout()))
            .build()
            .map_err(|e| InternalError::io_err(e.to_string().as_str(), None))?;
        let mongo_client = mongodb::Client::with_uri_str(&config.oauth().database().control_db_url)
            .await
            .map_err(|e| InternalError::io_err(e.to_string().as_str(), None))?;

        timeout(Duration::from_millis(config.server().timeout()), async {
            mongo_client
                .database("admin")
                .collection::<String>("system.users")
                .find(
                    None,
                    FindOptions::builder()
                        .limit(1)
                        .max_time(Duration::from_secs(1))
                        .max_await_time(Duration::from_secs(1))
                        .build(),
                )
                .await
        })
        .await
        .expect(
            "Failed to connect to MongoDB within 5 seconds. Please check your connection string.",
        )
        .ok();

        let database = mongo_client.database(config.oauth().database().control_db_name.as_ref());
        let secrets = SecretsClient::new(config.oauth().secrets_config())?;
        let oauths = MongoDbStore::<ConnectionOAuthDefinition>::new_with_db(
            database.clone(),
            Store::ConnectionOAuthDefinitions,
        )
        .await?;
        let connections =
            MongoDbStore::<Connection>::new_with_db(database.clone(), Store::Connections).await?;
        let cache = Cache::new(config.server().cache_size());
        let event_access =
            MongoDbStore::<EventAccess>::new_with_db(database.clone(), Store::EventAccess).await?;

        let oauths = Arc::new(oauths);
        let connections = Arc::new(connections);
        let secrets = Arc::new(secrets);
        let event_access = Arc::new(event_access);

        let actor = RefreshActor::new(
            oauths.clone(),
            connections.clone(),
            secrets.clone(),
            client.clone(),
        );
        let refresh_actor = Supervisor::start(move |_| actor);

        Ok(AppState {
            configuration: config,
            cache,
            event_access,
            connections,
            client,
            oauths,
            secrets,
            refresh_actor,
        })
    }

    pub fn configuration(&self) -> &Config {
        &self.configuration
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn connections(&self) -> &Arc<MongoDbStore<Connection>> {
        &self.connections
    }

    pub fn cache(&self) -> &Cache<HeaderValue, Arc<EventAccess>> {
        &self.cache
    }

    pub fn oauths(&self) -> &Arc<MongoDbStore<ConnectionOAuthDefinition>> {
        &self.oauths
    }

    pub fn event_access(&self) -> &Arc<MongoDbStore<EventAccess>> {
        &self.event_access
    }

    pub fn secrets(&self) -> &Arc<SecretsClient> {
        &self.secrets
    }

    pub fn refresh_actor(&self) -> &Addr<RefreshActor> {
        &self.refresh_actor
    }
}
