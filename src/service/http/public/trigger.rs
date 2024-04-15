use crate::prelude::{
    get_connection_to_trigger, AppState, ResponseType, ServerResponse, Trigger, TriggerActor,
};
use actix::Actor;
use actix_web::{
    post,
    web::{Data, Path},
    HttpResponse,
};
use integrationos_domain::{
    error::IntegrationOSError as Error, ApplicationError, Id, InternalError,
};
use serde_json::json;
use tracing_actix_web::RequestId;

#[tracing::instrument(name = "Trigger refresh", skip(state, request_id))]
#[post("/trigger/{id}")]
pub async fn trigger_refresh(
    request_id: RequestId,
    state: Data<AppState>,
    id: Path<Id>,
) -> Result<HttpResponse, Error> {
    let connection = get_connection_to_trigger(state.connections(), *id)
        .await?
        .ok_or(ApplicationError::not_found(
            format!("Connection with id {} not found", id).as_str(),
            None,
        ))?;

    tracing::info!("Triggering refresh for connection {}", connection.id);

    let actor = TriggerActor::new(
        state.connections().clone(),
        state.oauths().clone(),
        state.secrets().clone(),
        state.client().clone(),
        Some(request_id),
    )
    .start();

    let id = connection.id;
    let trigger = Trigger::new(connection);

    let outcome = actor
        .send(trigger)
        .await
        .map_err(|e| InternalError::io_err(e.to_string().as_str(), None))?;

    let json = json!({
        "id": id,
        "outcome": outcome,
    });

    Ok(ServerResponse::from(ResponseType::Trigger, json, 200))
}
