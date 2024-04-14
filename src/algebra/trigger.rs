use super::Parameter;
use crate::prelude::{Outcome, Trigger};
use actix::prelude::*;
use chrono::Duration;
use integrationos_domain::{
    algebra::{MongoStore, StoreExt},
    api_model_config::ContentType,
    connection_oauth_definition::{Computation, ConnectionOAuthDefinition, OAuthResponse},
    error::IntegrationOSError as Error,
    get_secret_request::GetSecretRequest,
    oauth_secret::OAuthSecret,
    service::secrets_client::SecretsClient,
    ApplicationError, Connection, Id, InternalError, OAuth,
};
use mongodb::bson::{self, doc};
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use tracing::warn;
use tracing_actix_web::RequestId;

pub struct TriggerActor {
    connections: Arc<MongoStore<Connection>>,
    oauths: Arc<MongoStore<ConnectionOAuthDefinition>>,
    secrets_client: Arc<SecretsClient>,
    request_id: Option<RequestId>,
    client: Client,
}

impl TriggerActor {
    pub fn new(
        connections: Arc<MongoStore<Connection>>,
        oauths: Arc<MongoStore<ConnectionOAuthDefinition>>,
        secrets_client: Arc<SecretsClient>,
        client: Client,
        request_id: Option<RequestId>,
    ) -> Self {
        Self {
            connections,
            oauths,
            secrets_client,
            request_id,
            client,
        }
    }
}

impl Actor for TriggerActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let request_id = self.request_id.map(|id| id.to_string());

        tracing::info!(
            request_id = request_id.as_deref(),
            "TriggerActor started with address: {:?}",
            ctx.address()
        );
    }
}

impl Supervised for TriggerActor {}

impl Handler<Trigger> for TriggerActor {
    type Result = ResponseFuture<Outcome>;

    #[tracing::instrument(
        name = "TriggerActor handle", skip(self, msg), fields(request_id = self.request_id.map(|id| id.to_string()).as_deref())
    )]
    fn handle(&mut self, msg: Trigger, _: &mut Self::Context) -> Self::Result {
        let oauths = self.oauths.clone();
        let secrets_client = self.secrets_client.clone();
        let connections = self.connections.clone();
        let client = self.client.clone();
        let request_id = self.request_id.map(|id| id.to_string());

        let future = async move {
            let ask = || async {
                let id = match &msg.connection().oauth {
                    Some(OAuth::Enabled {
                        connection_oauth_definition_id,
                        ..
                    }) => Ok(connection_oauth_definition_id),
                    _ => Err(ApplicationError::not_found(
                        format!("Connection {} has no oauth", msg.connection().id).as_str(),
                        None,
                    )),
                }?;

                let definition = oauths
                    .get_one(doc! {
                        "_id": id.to_string(),
                    })
                    .await
                    .map_err(|e| {
                        warn!("Failed to get connection oauth definition: {}", e);
                        ApplicationError::not_found(
                            format!("Connection oauth definition not found: {}", e).as_str(),
                            None,
                        )
                    })?
                    .ok_or(ApplicationError::not_found(
                        format!("Connection oauth definition not found: {}", id).as_str(),
                        None,
                    ))?;

                let secret: OAuthSecret = secrets_client
                    .get_secret::<OAuthSecret>(&GetSecretRequest {
                        id: msg.connection().secrets_service_id.clone(),
                        buildable_id: msg.connection().ownership.client_id.clone(),
                    })
                    .await
                    .map_err(|e| {
                        warn!("Failed to get secret: {}", e);
                        ApplicationError::not_found(
                            format!("Failed to get secret: {}", e).as_str(),
                            None,
                        )
                    })?;

                let compute_payload = serde_json::to_value(&secret).map_err(|e| {
                    warn!("Failed to serialize secret: {}", e);
                    InternalError::encryption_error("Failed to serialize secret", None)
                })?;

                let computation = definition
                    .compute
                    .refresh
                    .computation
                    .clone()
                    .map(|computation| computation.compute::<Computation>(&compute_payload))
                    .transpose()
                    .map_err(|e| {
                        warn!("Failed to compute oauth payload: {}", e);
                        InternalError::encryption_error("Failed to parse computation payload", None)
                    })?;

                let body = definition.body(&secret)?;
                let query = definition.query(computation.as_ref())?;
                let headers = definition.headers(computation.as_ref())?;

                let request = client
                    .post(definition.configuration.refresh.uri())
                    .headers(headers.unwrap_or_default());
                let request = match definition.configuration.refresh.content {
                    Some(ContentType::Json) => request.json(&body).query(&query),
                    Some(ContentType::Form) => request.form(&body).query(&query),
                    _ => request.query(&query),
                }
                .build()
                .map_err(|e| {
                    warn!("Failed to build request: {}", e);
                    InternalError::io_err("Failed to build request", None)
                })?;

                let response = client.execute(request).await.map_err(|e| {
                    warn!("Failed to execute request: {}", e);
                    InternalError::io_err("Failed to execute request", None)
                })?;

                let json = response.json::<serde_json::Value>().await.map_err(|e| {
                    warn!("Failed to parse response: {}", e);
                    InternalError::decryption_error("Failed to parse response", None)
                })?;

                let decoded: OAuthResponse = definition
                    .compute
                    .refresh
                    .response
                    .compute(&json)
                    .map_err(|e| {
                        warn!("Failed to decode oauth response from {}: {}", json, e);
                        InternalError::decryption_error("Failed to decode oauth response", None)
                    })?;

                let oauth_secret = secret.from_refresh(decoded, None, None, json);

                let secret = secrets_client
                    .create_secret(
                        msg.connection().clone().ownership.client_id,
                        &oauth_secret.as_json(),
                    )
                    .await
                    .map_err(|e| {
                        warn!("Failed to create oauth secret: {}", e);
                        InternalError::io_err("Failed to create oauth secret", None)
                    })?;

                let set = OAuth::Enabled {
                    connection_oauth_definition_id: *id,
                    expires_at: Some(
                        (chrono::Utc::now() + Duration::seconds(oauth_secret.expires_in as i64))
                            .timestamp(),
                    ),
                    expires_in: Some(oauth_secret.expires_in),
                };

                let data = doc! {
                    "$set": {
                        "oauth": bson::to_bson(&set).map_err(|e| {
                            warn!("Failed to serialize oauth: {}", e);
                            InternalError::encryption_error("Failed to serialize oauth", None)
                        })?,
                        "secretsServiceId": secret.id,
                    }
                };

                connections
                    .update_one(&msg.connection().id.to_string(), data)
                    .await
                    .map_err(|e| {
                        warn!("Failed to update connection: {}", e);
                        InternalError::io_err("Failed to update connection", None)
                    })?;

                tracing::info!(
                    request_id = request_id.as_deref(),
                    "Connection {} updated",
                    msg.connection().id
                );

                Ok::<Id, Error>(*id)
            };

            match ask().await {
                Ok(id) => {
                    Outcome::success(id.to_string().as_str(), json!({ "id": id.to_string() }))
                }
                Err(e) => Outcome::failure(
                    msg.connection().id.to_string().as_str(),
                    json!({ "error": e.to_string() }),
                ),
            }
        };

        Box::pin(future)
    }
}
