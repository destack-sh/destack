use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::bindings::BindingReplayPayload;
use crate::runtime::policy::PolicyState;
use crate::runtime::random::Random;
use crate::runtime::time::WorldInstant;
use crate::runtime::trace::{
    Outcome, Trace, TraceCheckpointIndex, TraceHeader, TraceImage, TraceRecord, TraceSequence,
};
use destack_core::CaptureMode;
use destack_workspace::{
    ExecutionMode, RandomMode, RandomOptions, ReplayOptions, ReplayPayloadMode, RuntimeOptions,
    TimeMode, TimeOptions,
};
use parking_lot::{Mutex, RwLock};

use serde::{Deserialize, Serialize};

use crate::runtime::world::{AccessState, Input, ObservationLog, World};

use super::{BranchId, CommittedRevision, Image, Moment, RebindContext, RevisionId, Snapshot};

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
    pub fn checkpoint(&self, name: impl Into<String>) -> RuntimeResult<CheckpointId> {
        let (checkpoint_id, revision_id, image, sequence) = {
            let _exclusive_access = self.acquire_exclusive_access()?;
            let committed = self.materialize_revision(CaptureMode::Fork, Some(name.into()))?;
            let checkpoint = committed.checkpoint.ok_or_else(|| {
                RuntimeError::Internal {
                    message: "checkpoint commit did not produce checkpoint metadata".to_string(),
                }
                .boxed()
            })?;

            (
                checkpoint.id,
                committed.revision.id,
                committed.image,
                self.trace.log().next_sequence(),
            )
        };

        let (size_bytes, hash) = World::image_size_and_hash(&image)?;
        self.trace
            .log()
            .record_checkpoint_exact(TraceCheckpointIndex {
                checkpoint_id,
                revision_id,
                sequence,
                path: TraceCheckpointIndex::memory_path(checkpoint_id),
                hash,
                size_bytes,
            })?;

        Ok(checkpoint_id)
    }

    /// Restore the world to one stored checkpoint.
    pub fn rewind(&self, checkpoint_id: CheckpointId) -> RuntimeResult<()> {
        let revision_id = {
            let lineage = self.lineage.read();
            let checkpoint = lineage.checkpoints.get(&checkpoint_id).ok_or_else(|| {
                RuntimeError::CheckpointNotFound {
                    checkpoint_id: checkpoint_id.get(),
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
            RuntimeError::CheckpointNotFound {
                checkpoint_id: checkpoint_id.get(),
            }
            .boxed()
        })?;

        Ok(checkpoint.clone())
    }

    /// Return identifiers for all stored checkpoints in stable order.
    pub fn checkpoint_ids(&self) -> Vec<CheckpointId> {
        self.lineage.read().checkpoints.keys().copied().collect()
    }

    /// Replace labels for one specific checkpoint.
    pub(crate) fn set_checkpoint_labels(
        &self,
        checkpoint_id: CheckpointId,
        labels: BTreeMap<String, String>,
    ) -> RuntimeResult<()> {
        let mut lineage = self.lineage.write();
        let checkpoint = lineage.checkpoints.get_mut(&checkpoint_id).ok_or_else(|| {
            RuntimeError::CheckpointNotFound {
                checkpoint_id: checkpoint_id.get(),
            }
            .boxed()
        })?;
        checkpoint.labels = labels;

        Ok(())
    }

    /// Restore this branch to one specific revision.
    pub fn rewind_revision(&self, revision_id: RevisionId) -> RuntimeResult<()> {
        let plan = {
            let lineage = self.lineage.read();
            lineage.resolve_revision_restore_plan(revision_id)?
        };

        let _exclusive_access = self.acquire_exclusive_access()?;
        let image = self.materialize_revision_image_from_plan(&plan, None)?;
        self.restore_revision_image(plan.target_revision.id, &image, &plan.trace_image, None)
    }

    /// Restore this branch to one specific moment.
    pub fn restore_moment(&self, moment: Moment) -> RuntimeResult<()> {
        if moment.branch_id != self.branch_id {
            return Err(RuntimeError::MomentBranchMismatch {
                moment_branch_id: moment.branch_id.get(),
                world_branch_id: self.branch_id.get(),
            }
            .boxed());
        }

        let plan = {
            let lineage = self.lineage.read();
            lineage.resolve_moment_restore_plan(moment)?
        };

        let _exclusive_access = self.acquire_exclusive_access()?;
        let image = self.materialize_moment_image_from_plan(&plan, None)?;
        let trace_image = self
            .trace
            .capture_image_through(plan.target_moment.sequence)?;
        self.restore_revision_image(plan.anchor_revision.id, &image, &trace_image, None)
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

    /// Materialize one new revision for the active branch.
    fn materialize_revision(
        &self,
        mode: CaptureMode,
        checkpoint_name: Option<String>,
    ) -> RuntimeResult<CommittedRevision> {
        // capture one exact world image first
        let image = self.capture_image(mode)?;
        let trace_image = self.trace.capture_image();

        // commit the revision in one authoritative lineage update
        let retain_image = if self.trace.mode() != ExecutionMode::Record {
            true
        } else if checkpoint_name.is_some() {
            true
        } else {
            !matches!(mode, CaptureMode::Suspend)
        };
        let committed = {
            let mut lineage = self.lineage.write();
            lineage.commit_revision(
                self.branch_id,
                image,
                trace_image,
                retain_image,
                WorldInstant::from_nanos(self.wall()),
                WorldInstant::from_nanos(self.mono()),
                checkpoint_name,
            )?
        };

        // move committed observations into lineage history
        let observations = self
            .observations
            .drain_through(self.branch_id, committed.revision.sequence);

        if !observations.is_empty() {
            let mut lineage = self.lineage.write();
            lineage.record_observations(self.branch_id, observations);
        }

        Ok(committed)
    }

    /// Capture one suspendable live revision for later local resume.
    pub fn suspend(&self) -> RuntimeResult<RevisionId> {
        let _exclusive_access = self.acquire_exclusive_access()?;
        let committed = self.materialize_revision(CaptureMode::Suspend, None)?;

        Ok(committed.revision.id)
    }

    /// Capture one hibernation snapshot for durable restore.
    pub fn hibernate_snapshot(&self) -> RuntimeResult<Snapshot> {
        let revision_id = {
            let _exclusive_access = self.acquire_exclusive_access()?;
            let committed = self.materialize_revision(CaptureMode::Hibernate, None)?;
            committed.revision.id
        };

        self.snapshot_revision(revision_id)
    }

    /// Fork one child world from one stored checkpoint.
    fn fork_inner(&self, checkpoint_id: CheckpointId, name: String) -> RuntimeResult<Arc<World>> {
        let revision_id = {
            let lineage = self.lineage.read();
            let checkpoint = lineage.checkpoints.get(&checkpoint_id).ok_or_else(|| {
                RuntimeError::CheckpointNotFound {
                    checkpoint_id: checkpoint_id.get(),
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
        rebind_context: Option<&RebindContext>,
    ) -> RuntimeResult<()> {
        self.restore_image(image, rebind_context)?;
        self.trace.restore_image(trace_image)?;
        self.trace.set_branch_id(self.branch_id);

        let mut lineage = self.lineage.write();
        lineage.set_branch_head_revision_id(self.branch_id, revision_id);

        Ok(())
    }

    /// Fork one child world from one stored revision.
    #[allow(clippy::arc_with_non_send_sync)]
    fn fork_revision_inner(
        &self,
        revision_id: RevisionId,
        name: String,
    ) -> RuntimeResult<Arc<World>> {
        // load the revision backing and allocate the child branch
        let (child_branch, plan) = {
            let mut lineage = self.lineage.write();
            let plan = lineage.resolve_revision_restore_plan(revision_id)?;
            let child_branch = lineage.fork_branch(plan.target_revision.id, name)?;

            (child_branch, plan)
        };

        // child trace: clone header but switch to the child branch
        let trace_header = self.fork_trace_header(child_branch.id);

        // child world: fresh mutable state over shared lineage backing
        let child = self.fork_child_world(child_branch.id, trace_header);

        // restore the child to the fork checkpoint
        let image = self.materialize_revision_image_from_plan(&plan, None)?;
        child.restore_image(&image, None)?;
        child.trace.restore_image(&plan.trace_image)?;
        child.trace.set_branch_id(child.branch_id);

        Ok(child)
    }

    /// Build replay options for one temporary suffix-replay world.
    fn replay_runtime_options(&self) -> RuntimeOptions {
        let header = self.trace.log().header();
        let replay_payload = match header.replay_payload {
            BindingReplayPayload::Results => ReplayPayloadMode::ResultsOnly,
            BindingReplayPayload::ArgumentsAndResults => ReplayPayloadMode::ArgumentsAndResults,
        };
        let replay_chunk_size_mb = if header.max_chunk_bytes == 0 {
            None
        } else {
            Some(header.max_chunk_bytes / (1024 * 1024))
        };

        RuntimeOptions {
            execution: ExecutionMode::Replay,
            time: TimeOptions {
                mode: TimeMode::Virtual,
                ..TimeOptions::default()
            },
            random: RandomOptions {
                mode: RandomMode::Deterministic,
                ..RandomOptions::default()
            },
            replay: ReplayOptions {
                chunk_size_mb: replay_chunk_size_mb,
                payload: replay_payload,
                ..ReplayOptions::default()
            },
            ..RuntimeOptions::default()
        }
    }

    /// Materialize one exact image for one restore plan.
    pub(super) fn materialize_revision_image_from_plan(
        &self,
        plan: &super::RevisionRestorePlan,
        rebind_context: Option<&RebindContext>,
    ) -> RuntimeResult<Image> {
        if !plan.requires_replay() {
            return Ok(plan.image.as_ref().clone());
        }

        let options = self.replay_runtime_options();
        let replay_world =
            World::build_with_branch(plan.target_revision.branch_id, &options, None)?;
        replay_world.restore_image(&plan.image, rebind_context)?;
        replay_world.trace.restore_replay_image(&plan.trace_image)?;
        replay_world
            .trace
            .set_branch_id(plan.target_revision.branch_id);
        replay_world
            .trace
            .seek_sequence(plan.base_revision.sequence)?;
        replay_world.replay_revision_suffix(plan.target_revision.sequence, rebind_context)?;

        replay_world.capture_image(CaptureMode::Suspend)
    }

    /// Materialize one exact image for one moment restore plan.
    fn materialize_moment_image_from_plan(
        &self,
        plan: &super::MomentRestorePlan,
        rebind_context: Option<&RebindContext>,
    ) -> RuntimeResult<Image> {
        if !plan.requires_replay() {
            return Ok(plan.image.as_ref().clone());
        }

        let options = self.replay_runtime_options();
        let replay_world = World::build_with_branch(plan.target_moment.branch_id, &options, None)?;
        let replay_world = Arc::try_unwrap(replay_world).map_err(|_| {
            RuntimeError::Internal {
                message: "fresh replay world should have one strong reference".to_string(),
            }
            .boxed()
        })?;
        replay_world.restore_image(&plan.image, rebind_context)?;
        let replay_trace = Trace::from_log(ExecutionMode::Replay, self.trace.log().clone());
        replay_trace.set_branch_id(plan.target_moment.branch_id);
        replay_trace.seek_sequence(plan.base_revision.sequence)?;

        while replay_trace.sequence()? != plan.target_moment.sequence {
            let event = replay_trace.next_event()?.ok_or_else(|| {
                RuntimeError::TraceExhausted {
                    sequence: plan.target_moment.sequence.get(),
                }
                .boxed()
            })?;

            match event {
                TraceRecord::Input(input) => {
                    replay_world.replay_input(input, rebind_context)?;
                }
                TraceRecord::Anchor(_) => {}
                TraceRecord::Outcome(
                    outcome @ (Outcome::RuntimeSpawned { .. } | Outcome::AgentSpawned { .. }),
                ) => {
                    replay_world.replay_outcome(outcome, rebind_context)?;
                }
                TraceRecord::Outcome(Outcome::TimeAdvance(_))
                | TraceRecord::Outcome(Outcome::Entropy(_))
                | TraceRecord::Outcome(Outcome::BindingCall(_)) => {
                    return Err(RuntimeError::Internal {
                        message: "unexpected low-level trace event escaped world replay"
                            .to_string(),
                    }
                    .boxed());
                }
            }
        }

        replay_world.capture_image(CaptureMode::Suspend)
    }

    /// Materialize one exact image for one committed moment.
    pub(crate) fn image_at_committed_moment(&self, moment: Moment) -> RuntimeResult<Image> {
        let plan = {
            let lineage = self.lineage.read();
            lineage.resolve_moment_restore_plan(moment)?
        };

        self.materialize_moment_image_from_plan(&plan, None)
    }

    /// Replay one revision suffix until the requested sequence boundary.
    fn replay_revision_suffix(
        &self,
        target_sequence: TraceSequence,
        rebind_context: Option<&RebindContext>,
    ) -> RuntimeResult<()> {
        while self.trace.sequence()? != target_sequence {
            let event = self.trace.next_event()?.ok_or_else(|| {
                RuntimeError::TraceExhausted {
                    sequence: target_sequence.get(),
                }
                .boxed()
            })?;

            match event {
                TraceRecord::Input(input) => {
                    self.replay_input(input, rebind_context)?;
                }
                TraceRecord::Anchor(_) => {}
                TraceRecord::Outcome(
                    outcome @ (Outcome::RuntimeSpawned { .. } | Outcome::AgentSpawned { .. }),
                ) => {
                    self.replay_outcome(outcome, rebind_context)?;
                }
                TraceRecord::Outcome(Outcome::TimeAdvance(_))
                | TraceRecord::Outcome(Outcome::Entropy(_))
                | TraceRecord::Outcome(Outcome::BindingCall(_)) => {
                    return Err(RuntimeError::Internal {
                        message: "unexpected low-level trace event escaped world replay"
                            .to_string(),
                    }
                    .boxed());
                }
            }
        }

        Ok(())
    }

    /// Apply one replayed world input.
    fn replay_input(
        &self,
        input: Input,
        _rebind_context: Option<&RebindContext>,
    ) -> RuntimeResult<()> {
        self.apply_input(input)
    }

    /// Apply one replayed world outcome.
    fn replay_outcome(
        &self,
        outcome: Outcome,
        rebind_context: Option<&RebindContext>,
    ) -> RuntimeResult<()> {
        match outcome {
            // runtime restore
            Outcome::RuntimeSpawned { runtime, agents } => {
                self.install_runtime_image(&runtime, &agents, rebind_context)?;
            }

            // agent restore
            Outcome::AgentSpawned { agent } => {
                self.install_agent_image(&agent, rebind_context)?;
            }

            // replay only world outcomes are handled above
            Outcome::TimeAdvance(_) | Outcome::Entropy(_) | Outcome::BindingCall(_) => {
                return Err(RuntimeError::Internal {
                    message: "unexpected low-level trace event escaped world replay".to_string(),
                }
                .boxed());
            }
        }

        Ok(())
    }

    /// Clone the current trace header for one child branch.
    fn fork_trace_header(&self, branch_id: BranchId) -> TraceHeader {
        let mut header = self.trace.log().header();
        header.branch_id = branch_id;
        header
    }

    /// Build one fresh child-world shell for one forked branch.
    #[allow(clippy::arc_with_non_send_sync)] // NOTE #Architecture: worlds are intentionally thread-affine and still reference counted
    fn fork_child_world(&self, branch_id: BranchId, trace_header: TraceHeader) -> Arc<World> {
        // parent execution mode
        let trace_mode = self.trace.mode();

        // parent clock and randomness policy
        let clock = self.clock.clone();
        let random = Random::new(self.random.root_seed());

        // fresh child shell: restore_image will install policy, topology, resources, simulation, ids, and runtimes
        Arc::new(World {
            branch_id,
            runtimes: RwLock::new(Default::default()),
            simulation: RwLock::new(Default::default()),
            time_mode: self.time_mode,
            random_mode: self.random_mode,
            clock,
            random,
            trace: Trace::new(trace_mode, trace_header),
            observations: ObservationLog::default(),
            policy: RwLock::new(PolicyState::new(self.policy())),
            mutation_lock: Mutex::new(()),
            next_runtime_id: AtomicU64::new(0),
            next_agent_id: AtomicU64::new(0),
            lineage: self.lineage.clone(),
            access_state: RwLock::new(AccessState::Shared {
                active_operations: 0,
            }),
            topology: RwLock::new(Default::default()),
            resources: RwLock::new(Default::default()),
        })
    }
}
