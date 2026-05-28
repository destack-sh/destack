use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::World;
use crate::world::trace::TraceCheckpointIndex;
use destack_core::CaptureMode;

use super::RevisionId;

/// Checkpoint identifier for one durable world restore point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

/// Checkpoint metadata for one durable world restore point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    /// The checkpoint identifier.
    pub id: CheckpointId,
    /// The revision anchored by this checkpoint.
    pub revision_id: RevisionId,
    /// The checkpoint name.
    pub name: String,
    /// The checkpoint labels.
    pub labels: BTreeMap<String, String>,
}

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
        let sequence = self.state.trace.log().next_sequence();

        let (size_bytes, hash) = World::image_size_and_hash(&image)?;
        self.state
            .trace
            .log()
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

    /// Restore the world to one stored checkpoint.
    pub fn rewind(&mut self, checkpoint_id: CheckpointId) -> RuntimeResult<()> {
        let revision = self.checkpoint_revision(checkpoint_id)?;

        self.rewind_revision(revision)
    }

    /// Fork one child world from one stored checkpoint.
    pub fn fork(
        &mut self,
        checkpoint_id: CheckpointId,
        name: impl Into<String>,
    ) -> RuntimeResult<World> {
        let revision = self.checkpoint_revision(checkpoint_id)?;

        self.fork_from_revision(revision, name.into())
    }

    /// Return metadata for one stored checkpoint.
    pub fn checkpoint_info(&self, checkpoint_id: CheckpointId) -> RuntimeResult<Checkpoint> {
        let history = self.history.read();
        let checkpoint = history
            .checkpoints
            .get(&checkpoint_id)
            .ok_or_else(|| RuntimeError::checkpoint_not_found(checkpoint_id.get()).boxed())?;

        Ok(checkpoint.clone())
    }

    /// Return identifiers for all stored checkpoints in stable order.
    pub fn checkpoint_ids(&self) -> Vec<CheckpointId> {
        self.history.read().checkpoints.keys().copied().collect()
    }

    /// Set one label on one specific checkpoint.
    pub fn label_checkpoint(
        &self,
        checkpoint_id: CheckpointId,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> RuntimeResult<()> {
        let mut history = self.history.write();
        let checkpoint = history
            .checkpoints
            .get_mut(&checkpoint_id)
            .ok_or_else(|| RuntimeError::checkpoint_not_found(checkpoint_id.get()).boxed())?;
        checkpoint.labels.insert(key.into(), value.into());

        Ok(())
    }

    /// Return the revision anchored by one checkpoint.
    pub(super) fn checkpoint_revision(
        &self,
        checkpoint_id: CheckpointId,
    ) -> RuntimeResult<RevisionId> {
        let history = self.history.read();
        let checkpoint = history
            .checkpoints
            .get(&checkpoint_id)
            .ok_or_else(|| RuntimeError::checkpoint_not_found(checkpoint_id.get()).boxed())?;

        Ok(checkpoint.revision_id)
    }
}
