use async_trait::async_trait;
use chrono::{DateTime, Utc};
use integrationos_domain::{Connection, Id, IntegrationOSError, MongoStore, StoreExt};
use mongodb::bson::doc;

#[async_trait]
pub trait StorageExt {
    async fn get_by(
        &self,
        refresh_before: &DateTime<Utc>,
        refresh_after: &DateTime<Utc>,
    ) -> Result<Vec<Connection>, IntegrationOSError>;

    async fn get(&self, id: Id) -> Result<Option<Connection>, IntegrationOSError>;
}

#[async_trait]
impl StorageExt for MongoStore<Connection> {
    async fn get_by(
        &self,
        refresh_before: &DateTime<Utc>,
        refresh_after: &DateTime<Utc>,
    ) -> Result<Vec<Connection>, IntegrationOSError> {
        self.get_many(
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

    async fn get(&self, id: Id) -> Result<Option<Connection>, IntegrationOSError> {
        self.get_one(doc! {
            "_id": id.to_string(),
        })
        .await
    }
}
