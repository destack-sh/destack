use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::replay::LogSequence;
use crate::runtime::time::WorldInstant;

use super::{BranchId, SnapshotId, World};

/// Checkpoint readiness state for world-owned checkpoint operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum CheckpointState {
    /// The world is ready for checkpoint operations.
    Ready,
    /// The world is inside one active execution or mutation region.
    Blocked,
}

/// One temporary guard that restores checkpoint readiness on drop.
struct BusyGuard<'a> {
    /// The world whose checkpoint state is currently blocked.
    world: &'a World,
}

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.world.set_checkpoint_state(CheckpointState::Ready);
    }
}

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
    /// The branch that owns this checkpoint.
    pub branch_id: BranchId,
    /// The replay sequence captured by this checkpoint.
    pub sequence: LogSequence,
    /// The durable snapshot backing this checkpoint.
    pub snapshot_id: SnapshotId,
    /// The wall-clock instant captured by this checkpoint.
    pub wall: WorldInstant,
    /// The monotonic instant captured by this checkpoint.
    pub mono: WorldInstant,
    /// The checkpoint name.
    pub name: String,
    /// The checkpoint labels.
    pub labels: BTreeMap<String, String>,
}

impl World {
    /// Require the world to be ready for checkpoint operations.
    pub(crate) fn require_checkpoint_ready(&self) -> RuntimeResult<()> {
        if *self.checkpoint_state.read() == CheckpointState::Ready {
            return Ok(());
        }

        Err(RuntimeError::Internal {
            message: "world is not ready for checkpoint operations".to_string(),
        }
        .boxed())
    }

    /// Set the current world checkpoint state.
    pub(crate) fn set_checkpoint_state(&self, state: CheckpointState) {
        *self.checkpoint_state.write() = state;
    }

    /// Execute one closure while checkpoint operations are blocked.
    pub(crate) fn with_checkpoint_blocked<T>(
        &self,
        callback: impl FnOnce() -> RuntimeResult<T>,
    ) -> RuntimeResult<T> {
        self.set_checkpoint_state(CheckpointState::Blocked);
        let _guard = BusyGuard { world: self };
        callback()
    }

    /// Create one checkpoint for the active branch.
    pub fn checkpoint(&self, name: impl Into<String>) -> RuntimeResult<CheckpointId> {
        self.checkpoint_inner(name.into())
    }

    /// Restore the world to one stored checkpoint.
    pub fn rewind(&self, checkpoint_id: CheckpointId) -> RuntimeResult<()> {
        self.require_checkpoint_ready()?;

        let snapshot = {
            let lineage = self.lineage.read();
            let checkpoint = lineage.checkpoints.get(&checkpoint_id).ok_or_else(|| {
                RuntimeError::Internal {
                    message: format!("checkpoint {} does not exist", checkpoint_id.get()),
                }
                .boxed()
            })?;

            lineage
                .snapshots
                .get(&checkpoint.snapshot_id)
                .cloned()
                .ok_or_else(|| {
                    RuntimeError::Internal {
                        message: format!(
                            "checkpoint {} is missing snapshot {}",
                            checkpoint_id.get(),
                            checkpoint.snapshot_id.get()
                        ),
                    }
                    .boxed()
                })?
        };

        self.restore_snapshot(&snapshot)
    }

    /// Fork one child world from one stored checkpoint.
    pub fn fork(
        &self,
        checkpoint_id: CheckpointId,
        name: impl Into<String>,
    ) -> RuntimeResult<Arc<World>> {
        self.fork_inner(checkpoint_id, name.into())
    }

    /// Return metadata for one stored checkpoint.
    pub fn checkpoint_info(&self, checkpoint_id: CheckpointId) -> RuntimeResult<Checkpoint> {
        let lineage = self.lineage.read();
        let checkpoint = lineage.checkpoints.get(&checkpoint_id).ok_or_else(|| {
            RuntimeError::Internal {
                message: format!("checkpoint {} does not exist", checkpoint_id.get()),
            }
            .boxed()
        })?;

        Ok(checkpoint.clone())
    }

    /// Return identifiers for all stored checkpoints in stable order.
    pub fn checkpoint_ids(&self) -> Vec<CheckpointId> {
        self.lineage.read().checkpoints.keys().copied().collect()
    }

    /// Return one stored checkpoint error until snapshot capture lands.
    fn checkpoint_inner(&self, name: String) -> RuntimeResult<CheckpointId> {
        let _ = name;
        let (checkpoint_id, snapshot_id) = {
            let lineage = self.lineage.read();

            (lineage.next_checkpoint_id, lineage.next_snapshot_id)
        };

        let error = self.capture_snapshot().unwrap_err();
        let error = format!(
            "world checkpointing is not implemented yet on branch {}: next checkpoint {checkpoint_id}, next snapshot {snapshot_id}, capture path failed with {error}",
            self.branch_id().get(),
        );

        Err(RuntimeError::Internal { message: error }.boxed())
    }

    /// Return one fork error until snapshot restore lands.
    fn fork_inner(&self, checkpoint_id: CheckpointId, name: String) -> RuntimeResult<Arc<World>> {
        let _ = checkpoint_id;
        let _ = name;
        self.require_checkpoint_ready()?;
        let branch_id = self.lineage.read().next_branch_id;

        Err(RuntimeError::Internal {
            message: format!(
                "world fork is not implemented yet from branch {}: next branch {branch_id}",
                self.branch_id().get()
            ),
        }
        .boxed())
    }
}
