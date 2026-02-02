use serde::{Deserialize, Serialize};

use crate::replay::{BranchId, CheckpointId, LogSequence, ReplayCheckpointIndex};

/// Snapshot metadata stored alongside checkpoint payloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Checkpoint identifier.
    pub checkpoint_id: CheckpointId,
    /// Branch identifier for the snapshot.
    pub branch_id: BranchId,
    /// Sequence number associated with the snapshot.
    pub sequence: LogSequence,
    /// Path to the snapshot file.
    pub path: String,
    /// Hash of the snapshot payload.
    pub hash: u128,
    /// Size of the snapshot payload in bytes.
    pub size_bytes: u64,
}

impl SnapshotMetadata {
    /// Convert snapshot metadata into a replay log checkpoint index.
    pub fn into_checkpoint_index(self) -> ReplayCheckpointIndex {
        ReplayCheckpointIndex {
            checkpoint_id: self.checkpoint_id,
            branch_id: self.branch_id,
            sequence: self.sequence,
            path: self.path,
            hash: self.hash,
            size_bytes: self.size_bytes,
        }
    }
}
