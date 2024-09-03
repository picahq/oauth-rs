use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Refresh {
    refresh_before_in_minutes: i64,
}

impl Refresh {
    pub fn new(refresh_before_in_minutes: i64) -> Self {
        Self {
            refresh_before_in_minutes,
        }
    }

    pub fn refresh_before_in_minutes(&self) -> i64 {
        self.refresh_before_in_minutes
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Refreshed {
    message: String,
    metadata: Value,
}

impl Refreshed {
    pub fn new(message: &str, metadata: Value) -> Self {
        Self {
            message: message.to_string(),
            metadata,
        }
    }
}
