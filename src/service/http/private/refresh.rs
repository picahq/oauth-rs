use crate::domain::Query;
use crate::service::{AppState, ResponseType, ServerResponse};
use actix_web::{get, web::Data, HttpResponse};
use integrationos_domain::error::IntegrationOSError as Error;
use integrationos_domain::InternalError;

#[tracing::instrument(skip(state))]
#[get("/get_state")]
pub async fn get_state(state: Data<AppState>) -> Result<HttpResponse, Error> {
    let response = state
        .refresh_actor
        .send(Query)
        .await
        .map_err(|e| InternalError::io_err(e.to_string().as_str(), None))?;

    Ok(ServerResponse::from(ResponseType::Query, response, 200))
}
