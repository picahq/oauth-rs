mod admin;
mod application;
mod middleware;
mod public;

use actix_web::{HttpResponse, HttpResponseBuilder};
pub use admin::*;
pub use application::*;
pub use middleware::*;
pub use public::*;
use reqwest::StatusCode;

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
