use chrono::{DateTime, Utc};
use futures::lock::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StatefulActor {
    /// The state of the actor
    state: Value,
    /// The last time the actor was updated
    last_updated: DateTime<Utc>,
}

impl StatefulActor {
    /// Create a new empty stateful actor
    ///
    /// An `Arc` is used to share the state between actors,
    /// when `clone` is called on the `Arc` the reference count
    /// is increased by one rather than creating a new copy of
    /// the state.
    ///
    /// Whilst a `Mutex` is used to ensure that only one actor
    /// can access the state for mutation at a time.
    ///
    /// Hence, the combination of `Arc` and `Mutex` allows
    /// multiple actors to share the same state and mutate it
    /// in a thread-safe manner.
    pub fn empty() -> Arc<Mutex<StatefulActor>> {
        Arc::new(Mutex::new(StatefulActor {
            state: Value::Null,
            last_updated: Utc::now(),
        }))
    }

    /// Returns the state of the actor
    pub fn state(&self) -> &Value {
        &self.state
    }

    /// Returns the last time the actor was updated
    pub fn last_updated(&self) -> &DateTime<Utc> {
        &self.last_updated
    }

    /// Updates the state of the actor
    pub async fn update(value: Value, state: Arc<Mutex<StatefulActor>>) {
        let mut actor = state.lock().await;
        actor.state = value;
        actor.last_updated = Utc::now();
    }
}
