mod configuration;

pub use configuration::*;

use integrationos_domain::{
    algebra::MongoStore, client::secrets_client::SecretsClient,
    connection_oauth_definition::ConnectionOAuthDefinition, error::IntegrationOSError as Error,
    event_access::EventAccess, Connection, InternalError, Store,
};
use mongodb::options::FindOptions;
use reqwest::Client;
use serde_json::Value;
use std::{sync::Arc, time::Duration};
use tokio::time::timeout;

use crate::Metrics;

#[derive(Clone, Debug)]
pub struct AppState {
    client: Client,
    secrets: Arc<SecretsClient>,
    connections: Arc<MongoStore<Connection>>,
    oauths: Arc<MongoStore<ConnectionOAuthDefinition>>,
    event_access: Arc<MongoStore<EventAccess>>,
    metrics: Arc<Metrics>,
}

impl AppState {
    pub async fn try_from(config: RefreshConfig) -> Result<Self, Error> {
        let client = Client::builder()
            .timeout(Duration::from_millis(config.timeout()))
            .build()
            .map_err(|e| InternalError::io_err(e.to_string().as_str(), None))?;
        let mongo_client = mongodb::Client::with_uri_str(&config.database().control_db_url)
            .await
            .map_err(|e| InternalError::io_err(e.to_string().as_str(), None))?;

        timeout(Duration::from_secs(config.timeout()), async {
            mongo_client
                .database(&config.database().event_db_name)
                .collection::<Value>("system-stats")
                .find(
                    None,
                    FindOptions::builder()
                        .limit(1)
                        .max_time(Duration::from_secs(config.timeout()))
                        .max_await_time(Duration::from_secs(config.timeout()))
                        .build(),
                )
                .await
                .map_err(|e| {
                    tracing::warn!("Failed to connect to MongoDB within {} seconds. Please check your connection string. {:?}", config.timeout(), e);
                    e
                })
        })
        .await
        .unwrap_or_else(|_| panic!("Failed to connect to MongoDB within {} seconds. Please check your connection string.", config.timeout()))
        .ok();

        let database = mongo_client.database(config.database().control_db_name.as_ref());
        let secrets = SecretsClient::new(config.secrets_config())?;
        let oauths = MongoStore::<ConnectionOAuthDefinition>::new(
            &database,
            &Store::ConnectionOAuthDefinitions,
        )
        .await?;
        let connections = MongoStore::<Connection>::new(&database, &Store::Connections).await?;
        let event_access = MongoStore::<EventAccess>::new(&database, &Store::EventAccess).await?;

        let oauths = Arc::new(oauths);
        let connections = Arc::new(connections);
        let secrets = Arc::new(secrets);
        let event_access = Arc::new(event_access);
        let metrics = Arc::new(Metrics::new()?);

        Ok(AppState {
            event_access,
            connections,
            metrics,
            client,
            oauths,
            secrets,
        })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn connections(&self) -> &Arc<MongoStore<Connection>> {
        &self.connections
    }

    pub fn oauths(&self) -> &Arc<MongoStore<ConnectionOAuthDefinition>> {
        &self.oauths
    }

    pub fn event_access(&self) -> &Arc<MongoStore<EventAccess>> {
        &self.event_access
    }

    pub fn secrets(&self) -> &Arc<SecretsClient> {
        &self.secrets
    }

    pub fn metrics(&self) -> &Arc<Metrics> {
        &self.metrics
    }
}
