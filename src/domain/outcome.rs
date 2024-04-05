use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    Success { message: String, metadata: Value },
    Failure { message: String, metadata: Value },
}

impl Outcome {
    pub fn success(message: &str, metadata: Value) -> Self {
        Self::Success {
            message: message.to_string(),
            metadata,
        }
    }

    pub fn failure(message: &str, metadata: Value) -> Self {
        Self::Failure {
            message: message.to_string(),
            metadata,
        }
    }
}
