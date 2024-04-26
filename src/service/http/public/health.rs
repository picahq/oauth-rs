use actix_web::{get, HttpResponse};

use crate::service::{ResponseType, ServerResponse};

#[get("/health_check")]
pub async fn health_check() -> HttpResponse {
    ServerResponse::from(ResponseType::Health, "I'm alive!".to_string(), 200)
}
