mod admin;
mod application;
mod middleware;
mod public;

pub use admin::*;
pub use application::*;
pub use middleware::*;
pub use public::*;

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
    pub response_type: ResponseType,
    pub args: T,
}

impl<T> ServerResponse<T>
where
    T: serde::Serialize,
{
    pub fn new(response_type: ResponseType, args: T) -> Self {
        Self {
            response_type,
            args,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ServerError {
    pub message: Vec<String>,
}

impl ServerError {
    pub fn new(message: Vec<String>) -> Self {
        Self { message }
    }
}

impl From<Vec<String>> for ServerResponse<ServerError> {
    fn from(message: Vec<String>) -> Self {
        Self {
            response_type: ResponseType::Error,
            args: ServerError::new(message.iter().map(|s| s.to_string()).collect()),
        }
    }
}

impl<'a> From<Vec<&'a str>> for ServerResponse<ServerError> {
    fn from(message: Vec<&'a str>) -> Self {
        Self {
            response_type: ResponseType::Error,
            args: ServerError::new(message.iter().map(|s| s.to_string()).collect()),
        }
    }
}
