use crate::{
    algebra::StorageExt,
    domain::Trigger,
    service::{AppState, ResponseType, ServerResponse},
    trigger,
};
use actix_web::{
    post,
    web::{Data, Path},
    HttpResponse,
};
use integrationos_domain::{
    error::IntegrationOSError as Error, ApplicationError, Id, InternalError,
};
use reqwest::StatusCode;
use serde_json::json;

#[tracing::instrument(name = "Trigger refresh", skip(state))]
#[post("/trigger/{id}")]
pub async fn trigger_refresh(state: Data<AppState>, id: Path<Id>) -> Result<HttpResponse, Error> {
    let id = id.into_inner();
    let connection = state
        .connections()
        .get(id)
        .await?
        .ok_or(ApplicationError::not_found(
            format!("Connection with id {} not found", id).as_str(),
            None,
        ))?;

    tracing::info!("Triggering refresh for connection {}", connection.id);

    let outcome = trigger(
        Trigger::new(connection),
        state.secrets().clone(),
        state.connections().clone(),
        state.oauths().clone(),
        state.client().clone(),
    )
    .await
    .map_err(|e| InternalError::io_err(e.to_string().as_str(), None))?;

    let json = json!({
        "id": id,
        "outcome": outcome,
    });

    Ok(ServerResponse::from(
        ResponseType::Trigger,
        json,
        StatusCode::OK.as_u16(),
    ))
}
