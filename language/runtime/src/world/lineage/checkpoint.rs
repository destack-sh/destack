use destack_core::CaptureMode;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::World;
use crate::world::topology::LabelSet;
use crate::world::trace::TraceCheckpointIndex;

use super::RevisionId;

/// Checkpoint metadata for one durable world restore point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Checkpoint {
    /// The checkpoint identifier.
    pub id: CheckpointId,
    /// The revision anchored by this checkpoint.
    pub revision_id: RevisionId,
    /// The checkpoint name.
    pub name: String,
    /// The checkpoint labels.
    pub labels: LabelSet,
}

/// Checkpoint identifier for one durable world restore point.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct CheckpointId(u64);

impl World {
    /// Create one checkpoint for the active branch.
    pub fn checkpoint(&mut self, name: impl Into<String>) -> RuntimeResult<CheckpointId> {
        let checkpoint_name = name.into();
        let committed = self.commit(CaptureMode::Fork, Some(checkpoint_name))?;
        let checkpoint = committed.3.ok_or_else(|| {
            RuntimeError::Internal {
                message: "checkpoint commit did not produce checkpoint metadata".to_string(),
            }
            .boxed()
        })?;
        let checkpoint_id = checkpoint.id;
        let revision = committed.0;
        let image = committed.2;
        let sequence = self.state.trace.store().next_sequence();

        let (size_bytes, hash) = World::image_size_and_hash(&image)?;
        self.state
            .trace
            .store()
            .record_checkpoint_exact(TraceCheckpointIndex {
                checkpoint_id,
                revision_id: revision,
                sequence,
                path: TraceCheckpointIndex::memory_path(checkpoint_id),
                hash,
                size_bytes,
            })?;

        Ok(checkpoint_id)
    }

    /// Set one label on one specific checkpoint.
    pub fn label_checkpoint(
        &self,
        checkpoint_id: CheckpointId,
        key: impl Into<Box<str>>,
        value: impl Into<Box<str>>,
    ) -> RuntimeResult<()> {
        let mut lineage = self.lineage.write();
        let checkpoint = lineage
            .checkpoints
            .get_mut(&checkpoint_id)
            .ok_or_else(|| RuntimeError::checkpoint_not_found(checkpoint_id.get()).boxed())?;
        checkpoint.labels.insert(key, value);

        Ok(())
    }
}

impl CheckpointId {
    /// Create a new checkpoint identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw checkpoint identifier value.
    pub const fn get(self) -> u64 {
        self.0
    }
}
