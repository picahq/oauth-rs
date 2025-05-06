use async_trait::async_trait;
use chrono::{DateTime, Utc};
use mongodb::bson::doc;
use osentities::{Connection, Id, MongoStore, PicaError};

#[async_trait]
pub trait StorageExt {
    async fn get_by(
        &self,
        refresh_before: &DateTime<Utc>,
        refresh_after: &DateTime<Utc>,
    ) -> Result<Vec<Connection>, PicaError>;

    async fn get(&self, id: Id) -> Result<Option<Connection>, PicaError>;
}

#[async_trait]
impl StorageExt for MongoStore<Connection> {
    async fn get_by(
        &self,
        refresh_before: &DateTime<Utc>,
        refresh_after: &DateTime<Utc>,
    ) -> Result<Vec<Connection>, PicaError> {
        self.get_many(
            Some(doc! {
                "oauth.enabled.expires_at": doc! {
                    "$gte": refresh_before.timestamp(),
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

    async fn get(&self, id: Id) -> Result<Option<Connection>, PicaError> {
        self.get_one(doc! {
            "_id": id.to_string(),
        })
        .await
    }
}
