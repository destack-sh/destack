/// Serialized snapshot payload for VM isolates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// Snapshot payload bytes.
    pub payload: Vec<u8>,
}
