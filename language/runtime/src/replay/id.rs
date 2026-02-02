use serde::{Deserialize, Serialize};

/// Sequence number for events within a replay log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LogSequence(u64);

impl LogSequence {
    /// Create a new log sequence number.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw sequence number.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Return the next sequence number.
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// Identifier for a replay branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchId(u128);

impl BranchId {
    /// Create a new branch identifier.
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    /// Return the raw branch identifier value.
    pub const fn get(self) -> u128 {
        self.0
    }
}

/// Identifier for a replay checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CheckpointId(u128);

impl CheckpointId {
    /// Create a new checkpoint identifier.
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    /// Return the raw checkpoint identifier value.
    pub const fn get(self) -> u128 {
        self.0
    }
}
