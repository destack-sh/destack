use serde::{Deserialize, Serialize};
use tspp_serde::{Reflect, Value};

/// One parsed non-code module payload.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub enum Data {
    /// One parsed JSON-like module value.
    Json(Value),
}
