use serde::{Deserialize, Serialize};

/// Serialized snapshot payload for VM isolates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    /// Snapshot payload bytes.
    pub payload: Vec<u8>,
}
