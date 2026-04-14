use std::collections::BTreeMap;
use std::sync::Arc;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::ResourceRebinders;
use crate::runtime::bindings::BindingReplayPayload;
use crate::runtime::random::Random;
use crate::runtime::time::WorldInstant;
use crate::runtime::trace::{
    Outcome, Trace, TraceCheckpointIndex, TraceHeader, TraceImage, TraceRecord, TraceSequence,
};
use destack_core::CaptureMode;
use destack_heap as heap;
use destack_workspace::{
    ExecutionMode, RandomMode, RandomOptions, ReplayOptions, ReplayPayloadMode, RuntimeOptions,
    TimeMode, TimeOptions,
};

use serde::{Deserialize, Serialize};

use crate::runtime::world::{Observations, World};

use super::{BranchId, Image, Moment, Revision, RevisionId, Snapshot};

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
        let checkpoint = committed.2.ok_or_else(|| {
            RuntimeError::Internal {
                message: "checkpoint commit did not produce checkpoint metadata".to_string(),
            }
            .boxed()
        })?;
        let checkpoint_id = checkpoint.id;
        let revision_id = committed.0.id;
        let image = committed.1;
        let sequence = self.trace.log().next_sequence();

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
    pub fn rewind(&mut self, checkpoint_id: CheckpointId) -> RuntimeResult<()> {
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
        &mut self,
        checkpoint_id: CheckpointId,
        name: impl Into<String>,
    ) -> RuntimeResult<World> {
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
    pub fn rewind_revision(&mut self, revision_id: RevisionId) -> RuntimeResult<()> {
        let (target_revision, base_revision, image, trace_image) = {
            let lineage = self.lineage.read();
            let target_revision = lineage.revision(revision_id)?;
            let base_revision_id = self.nearest_image_revision_id(revision_id)?;
            let (base_revision, image, _) = self.revision_data(base_revision_id)?;
            let trace_image = self.trace_image(revision_id)?;

            (target_revision, base_revision, image, trace_image)
        };

        let image =
            self.revision_image(&target_revision, &base_revision, &image, &trace_image, None)?;

        self.restore_revision_image(target_revision.id, &image, &trace_image, None)
    }

    /// Restore this branch to one specific moment.
    pub fn restore_moment(&mut self, moment: Moment) -> RuntimeResult<()> {
        if moment.branch_id != self.branch_id {
            return Err(RuntimeError::MomentBranchMismatch {
                moment_branch_id: moment.branch_id.get(),
                world_branch_id: self.branch_id.get(),
            }
            .boxed());
        }

        let (anchor_revision, base_revision, image) = {
            let lineage = self.lineage.read();
            let head_revision = lineage.head_revision_for_branch(moment.branch_id)?;
            if head_revision.sequence.get() < moment.sequence.get() {
                return Err(RuntimeError::MomentNotFound {
                    branch_id: moment.branch_id.get(),
                    sequence: moment.sequence.get(),
                }
                .boxed());
            }
            let anchor_revision =
                lineage.latest_revision_at_or_before(moment.branch_id, moment.sequence)?;
            let base_revision_id = self.nearest_image_revision_id(anchor_revision.id)?;
            let (base_revision, image, _) = self.revision_data(base_revision_id)?;

            (anchor_revision, base_revision, image)
        };

        let image = self.moment_image(moment, &base_revision, &image, None)?;
        let trace_image = self.trace.capture_image_through(moment.sequence)?;

        self.restore_revision_image(anchor_revision.id, &image, &trace_image, None)
    }

    /// Fork one child world from one specific revision.
    pub fn fork_revision(
        &mut self,
        revision_id: RevisionId,
        name: impl Into<String>,
    ) -> RuntimeResult<World> {
        self.fork_revision_inner(revision_id, name.into())
    }

    /// Commit one new revision for the active branch.
    fn commit(
        &mut self,
        mode: CaptureMode,
        checkpoint_name: Option<String>,
    ) -> RuntimeResult<(Revision, Arc<Image>, Option<Checkpoint>)> {
        // capture one exact world image first
        let image = self.capture_image(mode)?;
        let trace_image = self.trace.capture_image();

        // commit the revision in one authoritative lineage update
        let retain_image = self.trace.mode() != ExecutionMode::Record
            || checkpoint_name.is_some()
            || !matches!(mode, CaptureMode::Suspend);
        let mut image = image;
        let image_id = self.images.write().allocate_image_id();
        image.id = image_id;

        let image = Arc::new(image);
        let trace_image = Arc::new(trace_image);
        let (revision, checkpoint) = {
            let mut lineage = self.lineage.write();
            lineage.commit_revision(
                self.branch_id,
                image_id,
                trace_image.next_sequence,
                WorldInstant::from_nanos(self.wall()),
                WorldInstant::from_nanos(self.mono()),
                checkpoint_name,
            )?
        };

        {
            let mut images = self.images.write();
            if retain_image {
                images.insert_image(image.clone());
            }

            images.insert_trace_image(revision.id, trace_image);
        }

        let observations = self
            .observations
            .drain_through(self.branch_id, revision.sequence);

        if !observations.is_empty() {
            let mut lineage = self.lineage.write();
            lineage.record_observations(self.branch_id, observations);
        }

        Ok((revision, image, checkpoint))
    }

    /// Capture one suspendable live revision for later local resume.
    pub fn suspend(&mut self) -> RuntimeResult<RevisionId> {
        let committed = self.commit(CaptureMode::Suspend, None)?;

        Ok(committed.0.id)
    }

    /// Capture one hibernation snapshot for durable restore.
    pub fn hibernate_snapshot(&mut self) -> RuntimeResult<Snapshot> {
        let revision_id = {
            let committed = self.commit(CaptureMode::Hibernate, None)?;

            committed.0.id
        };

        self.snapshot_revision(revision_id)
    }

    /// Fork one child world from one stored checkpoint.
    fn fork_inner(&mut self, checkpoint_id: CheckpointId, name: String) -> RuntimeResult<World> {
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
        &mut self,
        revision_id: RevisionId,
        image: &Image,
        trace_image: &TraceImage,
        rebind_context: Option<&ResourceRebinders>,
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
        &mut self,
        revision_id: RevisionId,
        name: String,
    ) -> RuntimeResult<World> {
        // resolve the retained fork point before mutating lineage
        let target_revision = {
            let lineage = self.lineage.read();
            lineage.revision(revision_id)?
        };
        let base_revision_id = self.nearest_image_revision_id(revision_id)?;
        let (base_revision, image, _) = self.revision_data(base_revision_id)?;
        let trace_image = self.trace_image(revision_id)?;

        // allocate the child branch after retained resolution is complete
        let child_branch = {
            let mut lineage = self.lineage.write();
            lineage.fork_branch(target_revision.id, name)?
        };

        // child trace: clone header but switch to the child branch
        let trace_header = self.fork_trace_header(child_branch.id);

        // child world: fresh mutable state over shared lineage data
        let mut child = self.fork_child_world(child_branch.id, trace_header);

        // restore the child to the fork checkpoint
        let image =
            self.revision_image(&target_revision, &base_revision, &image, &trace_image, None)?;
        child.restore_image(&image, None)?;
        child.trace.restore_image(&trace_image)?;
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

    /// Restore one exact image for one committed revision.
    pub(super) fn revision_image(
        &self,
        target_revision: &Revision,
        base_revision: &Revision,
        image: &Image,
        trace_image: &TraceImage,
        rebinders: Option<&ResourceRebinders>,
    ) -> RuntimeResult<Image> {
        if base_revision.id == target_revision.id {
            return Ok(image.clone());
        }

        let options = self.replay_runtime_options();
        let mut replay_world = World::for_branch(target_revision.branch_id, &options, None)?;
        replay_world.restore_image(image, rebinders)?;
        replay_world.trace.restore_replay_image(trace_image)?;
        replay_world.trace.set_branch_id(target_revision.branch_id);
        replay_world.trace.seek_sequence(base_revision.sequence)?;
        let replay_trace =
            Trace::from_log(ExecutionMode::Replay, replay_world.trace().log().clone());
        replay_trace.set_branch_id(target_revision.branch_id);
        replay_trace.seek_sequence(base_revision.sequence)?;
        replay_world.replay_to(&replay_trace, target_revision.sequence, rebinders)?;

        replay_world.capture_image(CaptureMode::Suspend)
    }

    /// Restore one exact image for one moment.
    fn moment_image(
        &self,
        moment: Moment,
        base_revision: &Revision,
        image: &Image,
        rebinders: Option<&ResourceRebinders>,
    ) -> RuntimeResult<Image> {
        if base_revision.sequence == moment.sequence {
            return Ok(image.clone());
        }

        let options = self.replay_runtime_options();
        let mut replay_world = World::for_branch(moment.branch_id, &options, None)?;
        replay_world.restore_image(image, rebinders)?;
        let replay_trace = Trace::from_log(ExecutionMode::Replay, self.trace.log().clone());
        replay_trace.set_branch_id(moment.branch_id);
        replay_trace.seek_sequence(base_revision.sequence)?;
        replay_world.replay_to(&replay_trace, moment.sequence, rebinders)?;

        replay_world.capture_image(CaptureMode::Suspend)
    }

    /// Materialize one exact image for one committed moment.
    pub(crate) fn image_at_moment(&self, moment: Moment) -> RuntimeResult<Image> {
        let (_anchor_revision, base_revision, image) = {
            let lineage = self.lineage.read();
            let head_revision = lineage.head_revision_for_branch(moment.branch_id)?;
            if head_revision.sequence.get() < moment.sequence.get() {
                return Err(RuntimeError::MomentNotFound {
                    branch_id: moment.branch_id.get(),
                    sequence: moment.sequence.get(),
                }
                .boxed());
            }
            let anchor_revision =
                lineage.latest_revision_at_or_before(moment.branch_id, moment.sequence)?;
            let base_revision_id = self.nearest_image_revision_id(anchor_revision.id)?;
            let (base_revision, image, _) = self.revision_data(base_revision_id)?;

            (anchor_revision, base_revision, image)
        };

        self.moment_image(moment, &base_revision, &image, None)
    }

    /// Replay one trace through one requested sequence boundary.
    fn replay_to(
        &mut self,
        trace: &Trace,
        target_sequence: TraceSequence,
        rebinders: Option<&ResourceRebinders>,
    ) -> RuntimeResult<()> {
        while trace.sequence()? != target_sequence {
            let event = trace.next_event()?.ok_or_else(|| {
                RuntimeError::TraceExhausted {
                    sequence: target_sequence.get(),
                }
                .boxed()
            })?;

            match event {
                TraceRecord::Input(input) => {
                    self.apply_input(input)?;
                }
                TraceRecord::Anchor(_) => {}
                TraceRecord::Outcome(
                    outcome @ (Outcome::RuntimeSpawned { .. } | Outcome::AgentSpawned { .. }),
                ) => {
                    match outcome {
                        // runtime restore
                        Outcome::RuntimeSpawned { runtime, agents } => {
                            self.install_runtime_image(&runtime, &agents, rebinders)?;
                        }

                        // agent restore
                        Outcome::AgentSpawned { agent } => {
                            self.install_agent_image(&agent, rebinders)?;
                        }

                        // replay only world outcomes are handled above
                        Outcome::TimeAdvance(_) | Outcome::Entropy(_) | Outcome::BindingCall(_) => {
                            return Err(RuntimeError::Internal {
                                message: "unexpected low-level trace event escaped world replay"
                                    .to_string(),
                            }
                            .boxed());
                        }
                    }
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

    /// Clone the current trace header for one child branch.
    fn fork_trace_header(&self, branch_id: BranchId) -> TraceHeader {
        let mut header = self.trace.log().header();
        header.branch_id = branch_id;
        header
    }

    /// Build one fresh child-world shell for one forked branch.
    fn fork_child_world(&self, branch_id: BranchId, trace_header: TraceHeader) -> World {
        // parent execution mode
        let trace_mode = self.trace.mode();

        // parent clock and randomness policy
        let clock = self.clock.clone();
        let random = Random::new(self.random.root_seed());

        // fresh child shell: restore_image will install policy, topology, resources, simulation, ids, and runtimes
        World {
            branch_id,
            runtimes: Default::default(),
            simulation: Default::default(),
            policy: self.policy.clone(),
            next_runtime_id: 0,
            next_agent_id: 0,
            topology: Default::default(),
            resources: Default::default(),
            time_mode: self.time_mode,
            random_mode: self.random_mode,
            clock,
            random,
            trace: Trace::new(trace_mode, trace_header),
            observations: Observations::default(),
            arena: self.arena.clone(),
            lineage: self.lineage.clone(),
            images: self.images.clone(),
            shared: heap::SharedSpace::with_arena(self.arena.clone()),
            shared_limits: self.shared_limits,
        }
    }
}
