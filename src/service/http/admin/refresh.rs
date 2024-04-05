use crate::prelude::Query as RefreshQuery;
use crate::prelude::{AppState, ResponseType, ServerResponse};
use actix_web::{get, web::Data, HttpResponse};
use integrationos_domain::error::IntegrationOSError as Error;
use integrationos_domain::InternalError;

#[tracing::instrument(skip(state))]
#[get("/get_state")]
pub async fn get_state(state: Data<AppState>) -> Result<HttpResponse, Error> {
    let response = state
        .refresh_actor
        .send(RefreshQuery)
        .await
        .map_err(|e| InternalError::io_err(e.to_string().as_str(), None))?;

    Ok(HttpResponse::Ok().json(ServerResponse::new(ResponseType::Query, response)))
}
