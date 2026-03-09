use std::collections::BTreeMap;
use std::sync::Arc;

use destack_base::CaptureMode;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::random::Random;
use crate::runtime::replay::{Trace, TraceCheckpointIndex, TraceHeader, TraceImage};
use crate::runtime::time::WorldInstant;

use super::{BranchId, Image, Observe, RevisionId, World};

/// Shared and exclusive access state for world-owned execution and capture operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum AccessState {
    /// The world allows shared activity with the given number of active operations.
    Shared { active_operations: usize },
    /// The world is under exclusive access for capture or restore.
    Exclusive,
}

/// One temporary guard that releases one active world operation on drop.
pub(super) struct ActivityGuard<'a> {
    /// The world whose active operation count is currently borrowed.
    world: &'a World,
}

impl Drop for ActivityGuard<'_> {
    fn drop(&mut self) {
        self.world.finish_activity();
    }
}

/// One temporary lease that releases exclusive access when dropped.
pub(super) struct ExclusiveAccessLease<'a> {
    /// The world under exclusive access.
    world: &'a World,
}

impl Drop for ExclusiveAccessLease<'_> {
    fn drop(&mut self) {
        self.world.release_exclusive_access();
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
    /// The revision anchored by this checkpoint.
    pub revision_id: RevisionId,
    /// The checkpoint name.
    pub name: String,
    /// The checkpoint labels.
    pub labels: BTreeMap<String, String>,
}

impl World {
    /// Enter one world activity that must not overlap with exclusive world access.
    pub(super) fn enter_activity(&self) -> RuntimeResult<ActivityGuard<'_>> {
        let mut access_state = self.access_state.write();
        match *access_state {
            AccessState::Shared { active_operations } => {
                *access_state = AccessState::Shared {
                    active_operations: active_operations.saturating_add(1),
                };
                Ok(ActivityGuard { world: self })
            }
            AccessState::Exclusive => Err(RuntimeError::Internal {
                message: "world is under exclusive access for capture or restore".to_string(),
            }
            .boxed()),
        }
    }

    /// Finish one previously started world activity.
    fn finish_activity(&self) {
        let mut access_state = self.access_state.write();
        match *access_state {
            AccessState::Shared { active_operations } => {
                let active_operations = active_operations.saturating_sub(1);
                *access_state = AccessState::Shared { active_operations };
            }
            AccessState::Exclusive => {
                panic!("world activity finished under exclusive access");
            }
        }
    }

    /// Acquire one exclusive-access lease for capture or restore operations.
    pub(super) fn acquire_exclusive_access(&self) -> RuntimeResult<ExclusiveAccessLease<'_>> {
        let mut access_state = self.access_state.write();
        match *access_state {
            AccessState::Shared {
                active_operations: 0,
            } => {
                *access_state = AccessState::Exclusive;
                Ok(ExclusiveAccessLease { world: self })
            }
            AccessState::Shared { active_operations } => Err(RuntimeError::Internal {
                message: format!(
                    "world cannot acquire exclusive access while {active_operations} operation(s) are active"
                ),
            }
            .boxed()),
            AccessState::Exclusive => Err(RuntimeError::Internal {
                message: "world is already under exclusive access".to_string(),
            }
            .boxed()),
        }
    }

    /// Release one previously acquired exclusive-access lease.
    fn release_exclusive_access(&self) {
        *self.access_state.write() = AccessState::Shared {
            active_operations: 0,
        };
    }

    /// Create one checkpoint for the active branch.
    pub fn checkpoint(&self, name: impl Into<String>) -> RuntimeResult<CheckpointId> {
        let _exclusive_access = self.acquire_exclusive_access()?;
        self.checkpoint_inner(name.into())
    }

    /// Restore the world to one stored checkpoint.
    pub fn rewind(&self, checkpoint_id: CheckpointId) -> RuntimeResult<()> {
        let revision_id = {
            let lineage = self.lineage.read();
            let checkpoint = lineage.checkpoints.get(&checkpoint_id).ok_or_else(|| {
                RuntimeError::Internal {
                    message: format!("checkpoint {} does not exist", checkpoint_id.get()),
                }
                .boxed()
            })?;
            checkpoint.revision_id
        };

        self.rewind_revision(revision_id)
    }

    /// Fork one child world from one stored checkpoint.
    pub fn fork(
        &self,
        checkpoint_id: CheckpointId,
        name: impl Into<String>,
    ) -> RuntimeResult<Arc<World>> {
        let _exclusive_access = self.acquire_exclusive_access()?;
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

    /// Restore this branch to one specific revision.
    pub fn rewind_revision(&self, revision_id: RevisionId) -> RuntimeResult<()> {
        let backing = {
            let lineage = self.lineage.read();
            lineage.resolve_revision_backing(revision_id)?
        };

        let _exclusive_access = self.acquire_exclusive_access()?;
        self.restore_revision_image(revision_id, &backing.image, &backing.trace_image, None)
    }

    /// Fork one child world from one specific revision.
    pub fn fork_revision(
        &self,
        revision_id: RevisionId,
        name: impl Into<String>,
    ) -> RuntimeResult<Arc<World>> {
        let _exclusive_access = self.acquire_exclusive_access()?;
        self.fork_revision_inner(revision_id, name.into())
    }

    /// Capture one checkpoint for the active branch and advance branch lineage.
    fn checkpoint_inner(&self, name: String) -> RuntimeResult<CheckpointId> {
        // materialize one fork-safe revision
        let committed = self.materialize_revision(CaptureMode::Fork, Some(name))?;
        let checkpoint = committed
            .checkpoint
            .expect("checkpoint commit must create one checkpoint");

        // index the checkpoint in the trace trailer
        let (size_bytes, hash) = World::image_size_and_hash(&committed.image)?;
        self.trace.log().record_checkpoint(TraceCheckpointIndex {
            checkpoint_id: checkpoint.id,
            revision_id: committed.revision.id,
            sequence: self.trace.log().next_sequence(),
            path: format!("memory://checkpoint/{}", checkpoint.id.get()),
            hash,
            size_bytes,
        })?;

        Ok(checkpoint.id)
    }

    /// Materialize one new revision for the active branch.
    fn materialize_revision(
        &self,
        mode: CaptureMode,
        checkpoint_name: Option<String>,
    ) -> RuntimeResult<super::CommittedRevision> {
        // capture one exact world image first
        let image = self.capture_image(mode)?;
        let trace_image = self.trace.capture_image();

        // commit the revision in one authoritative lineage update
        let mut lineage = self.lineage.write();
        lineage.commit_revision(
            self.branch_id,
            image,
            trace_image,
            WorldInstant::from_nanos(self.wall()),
            WorldInstant::from_nanos(self.mono()),
            checkpoint_name,
        )
    }

    /// Capture one suspendable live revision for later local resume.
    pub fn suspend(&self) -> RuntimeResult<RevisionId> {
        let _exclusive_access = self.acquire_exclusive_access()?;
        let committed = self.materialize_revision(CaptureMode::Suspend, None)?;

        Ok(committed.revision.id)
    }

    /// Capture one hibernation snapshot for durable restore.
    pub fn hibernate_snapshot(&self) -> RuntimeResult<super::Snapshot> {
        let _exclusive_access = self.acquire_exclusive_access()?;
        let committed = self.materialize_revision(CaptureMode::Hibernate, None)?;

        self.snapshot_revision(committed.revision.id)
    }

    /// Fork one child world from one stored checkpoint.
    fn fork_inner(&self, checkpoint_id: CheckpointId, name: String) -> RuntimeResult<Arc<World>> {
        let revision_id = {
            let lineage = self.lineage.read();
            let checkpoint = lineage.checkpoints.get(&checkpoint_id).ok_or_else(|| {
                RuntimeError::Internal {
                    message: format!("checkpoint {} does not exist", checkpoint_id.get()),
                }
                .boxed()
            })?;

            checkpoint.revision_id
        };

        self.fork_revision_inner(revision_id, name)
    }

    /// Restore one specific revision image and update branch lineage.
    pub(crate) fn restore_revision_image(
        &self,
        revision_id: RevisionId,
        image: &Image,
        trace_image: &TraceImage,
        rebind_context: Option<&super::RebindContext>,
    ) -> RuntimeResult<()> {
        self.restore_image(image, rebind_context)?;
        self.trace.restore_image(trace_image)?;
        self.trace.set_branch_id(self.branch_id);

        let mut lineage = self.lineage.write();
        lineage.set_branch_head_revision_id(self.branch_id, revision_id);

        Ok(())
    }

    /// Fork one child world from one stored revision.
    fn fork_revision_inner(
        &self,
        revision_id: RevisionId,
        name: String,
    ) -> RuntimeResult<Arc<World>> {
        // load the revision backing and allocate the child branch
        let (child_branch, image, trace_image) = {
            let mut lineage = self.lineage.write();
            let backing = lineage.resolve_revision_backing(revision_id)?;
            let child_branch = lineage.fork_branch(backing.revision.id, name)?;

            (child_branch, backing.image, backing.trace_image)
        };

        // child trace: clone header but switch to the child branch
        let trace_header = self.forked_trace_header(child_branch.id);

        // child world: fresh mutable state over shared lineage backing
        let child = Arc::new(World {
            branch_id: child_branch.id,
            runtimes: parking_lot::RwLock::new(Default::default()),
            simulation: parking_lot::RwLock::new(Default::default()),
            time_mode: self.time_mode,
            random_mode: self.random_mode,
            clock: self.clock.clone(),
            random: Random::new(self.random.root_seed()),
            trace: Trace::new(self.trace.mode(), trace_header),
            observe: Observe::default(),
            policy: parking_lot::RwLock::new(self.policy.read().clone()),
            mutation_lock: parking_lot::Mutex::new(()),
            next_runtime_id: std::sync::atomic::AtomicU64::new(
                self.next_runtime_id
                    .load(std::sync::atomic::Ordering::SeqCst),
            ),
            next_agent_id: std::sync::atomic::AtomicU64::new(
                self.next_agent_id.load(std::sync::atomic::Ordering::SeqCst),
            ),
            lineage: self.lineage.clone(),
            access_state: parking_lot::RwLock::new(AccessState::Shared {
                active_operations: 0,
            }),
            topology: parking_lot::RwLock::new(self.topology.read().clone()),
            resources: parking_lot::RwLock::new(self.resources.read().clone()),
        });

        // restore the child to the fork checkpoint
        child.restore_image(&image, None)?;
        child.trace.restore_image(&trace_image)?;
        child.trace.set_branch_id(child.branch_id);

        Ok(child)
    }

    /// Clone the current trace header for one child branch.
    fn forked_trace_header(&self, branch_id: BranchId) -> TraceHeader {
        let mut header = self.trace.log().header();
        header.branch_id = branch_id;

        header
    }
}
