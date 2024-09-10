use crate::{
    algebra::StorageExt,
    domain::{Refresh, Trigger, Unit},
    Metrics, ParameterExt, Refreshed,
};
use chrono::{Duration, Utc};
use integrationos_domain::{
    algebra::MongoStore,
    api_model_config::ContentType,
    client::secrets_client::SecretsClient,
    connection_oauth_definition::{Computation, ConnectionOAuthDefinition, OAuthResponse},
    error::IntegrationOSError as Error,
    get_secret_request::GetSecretRequest,
    oauth_secret::OAuthSecret,
    ApplicationError, Connection, DefaultTemplate, InternalError, OAuth, TemplateExt,
};
use mongodb::bson::{self, doc};
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use tracing::warn;

#[derive(Debug, Clone, serde::Serialize)]
pub struct OAuthJson {
    #[serde(flatten)]
    pub json: serde_json::Value,
    pub metadata: OAuthSecret,
}

impl OAuthJson {
    pub fn as_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

pub async fn refresh(
    msg: Refresh,
    connections_store: Arc<MongoStore<Connection>>,
    secrets: Arc<SecretsClient>,
    oauths: Arc<MongoStore<ConnectionOAuthDefinition>>,
    client: Client,
    metrics: Arc<Metrics>,
) -> Result<Unit, Error> {
    let refresh_before = Utc::now();
    let refresh_after = refresh_before + Duration::minutes(msg.refresh_before_in_minutes());
    tracing::info!(
        "Searching for connections to refresh between {} and {}",
        refresh_before.timestamp(),
        refresh_after.timestamp()
    );

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
        let result = trigger(
            trigger_message,
            secrets.clone(),
            connections_store.clone(),
            oauths.clone(),
            client.clone(),
        );

        requests.push(result);
    }

    let results = futures::future::join_all(requests).await;

    let (successes, failures): (Vec<_>, Vec<_>) =
        results.into_iter().partition(|result| result.is_ok());

    tracing::info!("Refreshed {} connections: {:?}", successes.len(), successes);
    tracing::info!(
        "Failed to refresh {} connections: {:?}",
        failures.len(),
        failures
    );

    metrics.add_refreshed(successes.len() as u64);
    metrics.add_failed_to_refresh(failures.len() as u64);

    Ok(())
}

pub async fn trigger(
    msg: Trigger,
    secrets: Arc<SecretsClient>,
    connections: Arc<MongoStore<Connection>>,
    oauths: Arc<MongoStore<ConnectionOAuthDefinition>>,
    client: Client,
) -> Result<Refreshed, Error> {
    let template = DefaultTemplate::default();

    let conn_oauth_id = match &msg.connection().oauth {
        Some(OAuth::Enabled {
            connection_oauth_definition_id: conn_oauth_definition_id,
            ..
        }) => Ok(conn_oauth_definition_id),
        _ => Err(ApplicationError::not_found(
            format!("Connection {} has no oauth", msg.connection().id).as_str(),
            None,
        )),
    }?;

    let conn_oauth_definition = oauths
        .get_one(doc! {
            "_id": conn_oauth_id.to_string(),
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
            format!("Connection oauth definition not found: {}", conn_oauth_id).as_str(),
            None,
        ))?;

    let secret: OAuthSecret = secrets
        .get_secret::<OAuthSecret>(&GetSecretRequest {
            id: msg.connection().secrets_service_id.clone(),
            buildable_id: msg.connection().ownership.client_id.clone(),
        })
        .await
        .map_err(|e| {
            warn!("Failed to get secret: {}", e);
            ApplicationError::not_found(format!("Failed to get secret: {}", e).as_str(), None)
        })?;

    let compute_payload = serde_json::to_value(&secret).map_err(|e| {
        warn!("Failed to serialize secret: {}", e);
        InternalError::serialize_error("Failed to serialize secret", None)
    })?;

    let conn_oauth_definition = if conn_oauth_definition.is_full_template_enabled {
        template.render_as(&conn_oauth_definition, Some(&compute_payload))?
    } else {
        conn_oauth_definition
    };

    let computation = conn_oauth_definition
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

    let body = conn_oauth_definition.body(&secret)?;
    let query = conn_oauth_definition.query(computation.as_ref())?;
    let headers = conn_oauth_definition.headers(computation.as_ref())?;

    let request = client
        .post(conn_oauth_definition.configuration.refresh.uri())
        .headers(headers.unwrap_or_default());
    let request = match conn_oauth_definition.configuration.refresh.content {
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

    // This is done because some platforms do not return a refresh token in the response
    // (i.e. Salesforce). In these cases, we hold on to the original refresh token as a backup.
    let json_oauth = OAuthJson {
        json: json.clone(),
        metadata: secret.clone(),
    }
    .as_json();

    let decoded: OAuthResponse = conn_oauth_definition
        .compute
        .refresh
        .response
        .compute(&json_oauth)
        .map_err(|e| {
            warn!("Failed to decode oauth response from {}: {}", json_oauth, e);
            InternalError::decryption_error("Failed to decode oauth response", None)
        })?;

    let oauth_secret = secret.from_refresh(decoded, None, None, json);
    let secret = secrets
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
        connection_oauth_definition_id: *conn_oauth_id,
        expires_at: Some(
            (chrono::Utc::now() + Duration::seconds(oauth_secret.expires_in as i64)).timestamp(),
        ),
        expires_in: Some(oauth_secret.expires_in),
    };

    let data = doc! {
        "$set": {
            "oauth": bson::to_bson(&set).map_err(|e| {
                warn!("Failed to serialize oauth: {}", e);
                InternalError::serialize_error("Failed to serialize oauth", None)
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

    tracing::info!("Connection {} updated", msg.connection().id);

    Ok(Refreshed::new(
        msg.connection().id.to_string().as_str(),
        json!({ "id": msg.connection().id.to_string() }),
    ))
}
