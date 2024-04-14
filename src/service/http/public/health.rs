use crate::prelude::{ResponseType, ServerResponse};
use actix_web::{get, HttpResponse};

#[get("/health_check")]
pub async fn health_check() -> HttpResponse {
    ServerResponse::from(ResponseType::Health, "I'm alive!".to_string(), 200)
}
