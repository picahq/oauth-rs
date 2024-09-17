mod configuration;

pub use configuration::*;

use crate::{Metrics, SecretsClient};
use integrationos_domain::{
    algebra::MongoStore, connection_oauth_definition::ConnectionOAuthDefinition,
    error::IntegrationOSError as Error, event_access::EventAccess, Connection, InternalError,
    Store,
};
use mongodb::options::FindOptions;
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use reqwest_tracing::TracingMiddleware;
use serde_json::Value;
use std::{sync::Arc, time::Duration};
use tokio::time::timeout;

#[derive(Clone, Debug)]
pub struct AppState {
    client: ClientWithMiddleware,
    secrets: Arc<SecretsClient>,
    connections: Arc<MongoStore<Connection>>,
    oauths: Arc<MongoStore<ConnectionOAuthDefinition>>,
    event_access: Arc<MongoStore<EventAccess>>,
    metrics: Arc<Metrics>,
}

impl AppState {
    pub async fn try_from(config: RefreshConfig) -> Result<Self, Error> {
        let retry_policy =
            ExponentialBackoff::builder().build_with_max_retries(config.max_retries());
        let client = Client::builder()
            .timeout(Duration::from_millis(config.timeout()))
            .build()
            .map_err(|e| InternalError::io_err(e.to_string().as_str(), None))?;
        let client = ClientBuilder::new(client)
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .with(TracingMiddleware::default())
            .build();
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
        let oauths = MongoStore::<ConnectionOAuthDefinition>::new(
            &database,
            &Store::ConnectionOAuthDefinitions,
        )
        .await?;
        let connections = MongoStore::<Connection>::new(&database, &Store::Connections).await?;
        let event_access = MongoStore::<EventAccess>::new(&database, &Store::EventAccess).await?;

        let oauths = Arc::new(oauths);
        let connections = Arc::new(connections);
        let event_access = Arc::new(event_access);
        let metrics = Arc::new(Metrics::new()?);
        let secrets = SecretsClient::new(&config, &event_access, client.clone());
        let secrets = Arc::new(secrets);

        Ok(AppState {
            event_access,
            connections,
            metrics,
            client,
            oauths,
            secrets,
        })
    }

    pub fn client(&self) -> &ClientWithMiddleware {
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
