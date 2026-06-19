use serde::{Deserialize, Serialize};

/// Constant value stored in lowered instructions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConstValue {
    /// Constant payload for an aggregate value.
    Aggregate(Box<[u8]>),
}
