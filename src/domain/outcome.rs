use integrationos_domain::IntegrationOSError;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum Outcome {
    Success {
        message: String,
        metadata: Value,
    },
    Failure {
        error: IntegrationOSError,
        metadata: Value,
    },
}

impl Outcome {
    pub fn success(message: &str, metadata: Value) -> Self {
        Self::Success {
            message: message.to_string(),
            metadata,
        }
    }

    pub fn failure(error: IntegrationOSError, metadata: Value) -> Self {
        Self::Failure { error, metadata }
    }
}
