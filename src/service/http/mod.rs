mod private;
mod public;

pub use private::*;
pub use public::*;

use super::AppState;
use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    error::ErrorUnauthorized,
    web::Data,
    Error as ActixWebError, HttpMessage,
};
use actix_web::{HttpResponse, HttpResponseBuilder};
use actix_web_lab::middleware::Next;
use integrationos_domain::{algebra::StoreExt, event_access::EventAccess, Claims};
use jsonwebtoken::{decode, DecodingKey, Validation};
use mongodb::bson::doc;
use reqwest::StatusCode;
use std::sync::Arc;

#[derive(serde::Serialize, serde::Deserialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ResponseType {
    Health,
    Trigger,
    Query,
    Error,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ServerResponse<T>
where
    T: serde::Serialize,
{
    #[serde(rename = "type")]
    pub r#type: ResponseType,
    pub args: T,
    pub code: u16,
}

impl<T> ServerResponse<T>
where
    T: serde::Serialize,
{
    pub fn from(r#type: ResponseType, args: T, code: u16) -> HttpResponse {
        HttpResponseBuilder::new(StatusCode::from_u16(code).unwrap_or(StatusCode::OK))
            .json(ServerResponse { r#type, args, code })
    }
}

pub async fn auth_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, ActixWebError> {
    let state = req.app_data::<Data<AppState>>();
    let state = match state {
        None => return Err(ErrorUnauthorized("No state found")),
        Some(state) => state,
    };

    let event_access = extract_event_info(&req, state).await;
    let claims = claims(&req, state);

    match (event_access, claims) {
        (Ok(event_access), Ok(claims)) => {
            req.extensions_mut().insert(claims.to_owned());
            req.extensions_mut().insert(event_access.to_owned());
            next.call(req).await
        }
        (Err(err), _) | (_, Err(err)) => Err(ErrorUnauthorized(err)),
    }
}

fn claims(req: &ServiceRequest, state: &Data<AppState>) -> Result<Claims, ActixWebError> {
    let token = req
        .headers()
        .get(state.configuration().server().admin_header())
        .and_then(|header| header.to_str().ok())
        .map(|h| h.to_string().split_at(7).1.to_string());

    let token = match token {
        Some(token) => token,
        None => return Err(ErrorUnauthorized("No token found")),
    };

    let mut validator = Validation::default();
    validator.set_audience(&["integration-team", "oauth-integrationos"]);

    let claims = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(state.configuration().server().admin_secret().as_ref()),
        &validator,
    )
    .map_err(|_| ErrorUnauthorized("Invalid token"))?;

    Ok(claims.claims)
}

async fn extract_event_info(
    req: &ServiceRequest,
    state: &Data<AppState>,
) -> Result<Arc<EventAccess>, ActixWebError> {
    let Some(auth_header) = req
        .headers()
        .get(state.configuration().server().auth_header())
    else {
        Err(ErrorUnauthorized("No auth header found"))?
    };

    let event_access = state
        .cache()
        .try_get_with_by_ref(auth_header, async {
            let key = auth_header
                .to_str()
                .map_err(|e| format!("Invalid auth header: {}", e))?;

            if let Some(event_access) = state
                .event_access()
                .get_one(doc! {
                    "accessKey": key,
                    "deleted": false
                })
                .await
                .map_err(|e| {
                    tracing::warn!("{}", e);
                    format!("{}", e)
                })?
            {
                Ok(Arc::new(event_access))
            } else {
                Err(format!("No event access found for key: {}", key))
            }
        })
        .await;

    let event_access: Arc<EventAccess> = match event_access {
        Ok(event_access) => event_access,
        Err(err) => Err(ErrorUnauthorized(err))?,
    };

    Ok(event_access)
}
