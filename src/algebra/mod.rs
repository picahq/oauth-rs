mod parameter;
mod refresh;
mod token;
mod trigger;

pub use parameter::*;
pub use refresh::*;
pub use token::*;
pub use trigger::*;

use chrono::{DateTime, Utc};
use integrationos_domain::{
    algebra::{MongoStore, StoreExt},
    Connection, Id, IntegrationOSError,
};
use mongodb::bson::doc;

pub async fn get_connections_to_refresh(
    collection: &MongoStore<Connection>,
    refresh_before: &DateTime<Utc>,
    refresh_after: &DateTime<Utc>,
) -> Result<Vec<Connection>, IntegrationOSError> {
    collection
        .get_many(
            Some(doc! {
                "oauth.enabled.expires_at": doc! {
                    "$gt": refresh_before.timestamp(),
                    "$lte": refresh_after.timestamp(),
                },
            }),
            None,
            None,
            None,
            None,
        )
        .await
}

pub async fn get_connection_to_trigger(
    collection: &MongoStore<Connection>,
    id: Id,
) -> Result<Option<Connection>, IntegrationOSError> {
    collection
        .get_one(doc! {
            "_id": id.to_string(),
        })
        .await
}
