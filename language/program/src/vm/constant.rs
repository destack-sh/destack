use serde::{Deserialize, Serialize};

/// Constant value stored in lowered instructions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConstValue {
    /// Constant bytes for a frame-backed value.
    Bytes(Box<[u8]>),
}
