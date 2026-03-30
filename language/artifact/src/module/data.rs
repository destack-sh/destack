use serde::{Deserialize, Serialize};

/// One parsed non-code module payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Data {
    /// One parsed JSON-like module value.
    Json(serde_json::Value),
}
