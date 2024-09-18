use crate::RefreshConfig;
use integrationos_domain::{
    environment::Environment, event_access::EventAccess, IntegrationOSError, InternalError,
    MongoStore, Secret,
};
use mongodb::bson::doc;
use reqwest_middleware::ClientWithMiddleware;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::warn;

const PRODUCTION_KEY: &str = "event_access::custom::live::default::event-inc::internal-ui";
const TEST_KEY: &str = "event_access::custom::test::default::event-inc::internal-ui";
const INTEGRATIONOS_SECRET_HEADER: &str = "X-INTEGRATIONOS-SECRET";

#[derive(Debug, Clone)]
pub struct SecretsClient {
    get: String,
    create: String,
    client: ClientWithMiddleware,
    event: Arc<MongoStore<EventAccess>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateSecretRequest {
    secret: Value,
}

impl SecretsClient {
    pub fn new(
        config: &RefreshConfig,
        event: &Arc<MongoStore<EventAccess>>,
        client: ClientWithMiddleware,
    ) -> Self {
        Self {
            get: config.get_secret().to_string(),
            create: config.create_secret().to_string(),
            client,
            event: Arc::clone(event),
        }
    }

    pub async fn get_secret<T: for<'a> Deserialize<'a>>(
        &self,
        id: &str,
        buildable_id: &str,
        environment: &Environment,
    ) -> Result<T, IntegrationOSError> {
        let key = match environment {
            Environment::Test | Environment::Development => TEST_KEY,
            Environment::Live | Environment::Production => PRODUCTION_KEY,
        };

        let event = self
            .event
            .get_one(doc! {
                "ownership.buildableId": buildable_id,
                "key": key,
                "deleted": false
            })
            .await?
            .ok_or(InternalError::key_not_found("Event access not found", None))?;

        let access_key = event.access_key.clone();

        let uri = format!("{}/{}", self.get, id);
        let response = self
            .client
            .get(&uri)
            .header(INTEGRATIONOS_SECRET_HEADER, access_key)
            .send()
            .await
            .map_err(|err| {
                InternalError::io_err(&format!("Failed to send request: {err}"), None)
            })?;

        let secret = response.json().await;

        let secret: Secret = secret.map_err(|err| {
            warn!("Failed to deserialize response: {err}");
            InternalError::serialize_error(&format!("Failed to deserialize response: {err}"), None)
        })?;

        secret.decode()
    }

    pub async fn create_secret<T: Serialize + for<'a> Deserialize<'a>>(
        &self,
        buildable_id: String,
        secret: T,
        environment: Environment,
    ) -> Result<Secret, IntegrationOSError> {
        let payload = CreateSecretRequest {
            secret: serde_json::to_value(&secret).map_err(|e| {
                warn!("Failed to serialize secret: {}", e);
                InternalError::serialize_error("Failed to serialize secret", None)
            })?,
        };

        let key = match environment {
            Environment::Test | Environment::Development => TEST_KEY,
            Environment::Live | Environment::Production => PRODUCTION_KEY,
        };

        let event = self
            .event
            .get_one(doc! {
                "ownership.buildableId": buildable_id,
                "key": key
            })
            .await?
            .ok_or(InternalError::key_not_found("Event access not found", None))?;

        let access_key = event.access_key.clone();

        let response = self
            .client
            .post(&self.create)
            .json(&payload)
            .header(INTEGRATIONOS_SECRET_HEADER, access_key)
            .send()
            .await
            .map_err(|err| {
                InternalError::io_err(&format!("Failed to send request: {err}"), None)
            })?;

        response.json().await.map_err(|err| {
            warn!("Failed to deserialize response: {err}");
            InternalError::serialize_error(&format!("Failed to deserialize response: {err}"), None)
        })
    }
}
