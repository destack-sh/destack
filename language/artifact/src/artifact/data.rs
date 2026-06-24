use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// One parsed non-code module payload.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub enum Data {
    /// One parsed JSON-like module value.
    Json(serde_json::Value),
}
