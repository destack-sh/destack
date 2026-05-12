use std::collections::BTreeMap;
use std::sync::Arc;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::ResourceRebinders;
use crate::runtime::binding::BindingReplayPayload;
use crate::runtime::random::Random;
use crate::runtime::time::Instant;
use crate::runtime::trace::{
    Observations, Outcome, Trace, TraceCheckpointIndex, TraceHeader, TraceImage, TraceRecord,
    TraceSequence,
};
use destack_core::CaptureMode;
use destack_workspace::{ExecutionMode, ReplayPayloadMode, RuntimeOptions};

use serde::{Deserialize, Serialize};

use crate::runtime::world::{World, WorldState};

use super::{BranchId, Moment, Revision, RevisionState, WorldImage, WorldSnapshot};

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
    pub revision: Revision,
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
                revision,
                sequence,
                path: TraceCheckpointIndex::memory_path(checkpoint_id),
                hash,
                size_bytes,
            })?;

        Ok(checkpoint_id)
    }

    /// Restore the world to one stored checkpoint.
    pub fn rewind(&mut self, checkpoint_id: CheckpointId) -> RuntimeResult<()> {
        let revision = {
            let lineage = self.lineage.read();
            let checkpoint = lineage.checkpoints.get(&checkpoint_id).ok_or_else(|| {
                RuntimeError::CheckpointNotFound {
                    checkpoint_id: checkpoint_id.get(),
                }
                .boxed()
            })?;
            checkpoint.revision
        };

        self.rewind_revision(revision)
    }

    /// Fork one child world from one stored checkpoint.
    pub fn fork(
        &mut self,
        checkpoint_id: CheckpointId,
        name: impl Into<String>,
    ) -> RuntimeResult<World> {
        self.fork_from_checkpoint(checkpoint_id, name.into())
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
    pub fn set_checkpoint_labels(
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
    pub fn rewind_revision(&mut self, revision: Revision) -> RuntimeResult<()> {
        let (target_revision, base_revision, image, trace_image) = {
            let lineage = self.lineage.read();
            let target_revision = lineage.revision_state(revision)?;
            let base_revision = self.nearest_image_revision(revision)?;
            let (base_revision, image, _) = self.revision_data(base_revision)?;
            let trace_image = self.trace_image(revision)?;

            (target_revision, base_revision, image, trace_image)
        };

        let image =
            self.revision_image(&target_revision, &base_revision, &image, &trace_image, None)?;

        self.restore_revision_image(revision, &image, &trace_image, None)
    }

    /// Restore this branch to one specific moment.
    pub fn restore_moment(&mut self, moment: Moment) -> RuntimeResult<()> {
        if moment.branch_id != self.state.branch_id {
            return Err(RuntimeError::MomentBranchMismatch {
                moment_branch_id: moment.branch_id.get(),
                world_branch_id: self.state.branch_id.get(),
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
            let anchor_revision_id =
                lineage.latest_revision_at_or_before(moment.branch_id, moment.sequence)?;
            let base_revision = self.nearest_image_revision(anchor_revision_id)?;
            let (base_revision, image, _) = self.revision_data(base_revision)?;

            (anchor_revision_id, base_revision, image)
        };

        let image = self.moment_image(moment, &base_revision, &image, None)?;
        let trace_image = self.state.trace.capture_image_through(moment.sequence)?;

        self.restore_revision_image(anchor_revision, &image, &trace_image, None)
    }

    /// Fork one child world from one specific revision.
    pub fn fork_revision(
        &mut self,
        revision: Revision,
        name: impl Into<String>,
    ) -> RuntimeResult<World> {
        self.fork_from_revision(revision, name.into())
    }

    /// Commit one new revision for the active branch.
    fn commit(
        &mut self,
        mode: CaptureMode,
        checkpoint_name: Option<String>,
    ) -> RuntimeResult<(Revision, RevisionState, Arc<WorldImage>, Option<Checkpoint>)> {
        // capture one exact world image first
        let image = self.capture_image(mode)?;
        let trace_image = self.state.trace.capture_image();

        // commit the revision in one authoritative lineage update
        let retain_image = self.state.trace.mode() != ExecutionMode::Record
            || checkpoint_name.is_some()
            || !matches!(mode, CaptureMode::Suspend);
        let image_id = self.lineage.write().allocate_image_id();
        let mut image = image;

        if retain_image {
            self.lineage
                .write()
                .retain_image_payloads(self.state.branch_id, &mut image)?;
        }

        let image = Arc::new(image);
        let trace_image = Arc::new(trace_image);
        let (revision, revision_state, checkpoint) = {
            let mut lineage = self.lineage.write();
            lineage.commit_revision(
                self.state.branch_id,
                image_id,
                trace_image.next_sequence,
                Instant::from_nanos(self.wall()),
                Instant::from_nanos(self.mono()),
                checkpoint_name,
            )?
        };

        {
            let mut lineage = self.lineage.write();
            if retain_image {
                lineage.insert_image(image_id, image.clone());
            }

            lineage.insert_trace_image(revision_state.trace_image_id, trace_image);
        }

        let observations = self
            .state
            .observations
            .drain_through(self.state.branch_id, revision_state.sequence);

        if !observations.is_empty() {
            let mut lineage = self.lineage.write();
            lineage.record_observations(self.state.branch_id, observations);
        }

        Ok((revision, revision_state, image, checkpoint))
    }

    /// Capture one suspendable live revision for later local resume.
    pub fn suspend(&mut self) -> RuntimeResult<Revision> {
        let committed = self.commit(CaptureMode::Suspend, None)?;

        Ok(committed.0)
    }

    /// Capture one hibernation snapshot for durable restore.
    pub fn hibernate_snapshot(&mut self) -> RuntimeResult<WorldSnapshot> {
        let revision = self.commit(CaptureMode::Hibernate, None)?.0;

        self.snapshot_revision(revision)
    }

    /// Fork one child world from one stored checkpoint.
    fn fork_from_checkpoint(
        &mut self,
        checkpoint_id: CheckpointId,
        name: String,
    ) -> RuntimeResult<World> {
        let revision = {
            let lineage = self.lineage.read();
            let checkpoint = lineage.checkpoints.get(&checkpoint_id).ok_or_else(|| {
                RuntimeError::CheckpointNotFound {
                    checkpoint_id: checkpoint_id.get(),
                }
                .boxed()
            })?;

            checkpoint.revision
        };

        self.fork_from_revision(revision, name)
    }

    /// Restore one specific revision image and update branch lineage.
    pub(crate) fn restore_revision_image(
        &mut self,
        revision: Revision,
        image: &WorldImage,
        trace_image: &TraceImage,
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<()> {
        self.restore_image(image, rebind_context)?;
        self.state.trace.restore_image(trace_image)?;
        self.state.trace.set_branch_id(self.state.branch_id);

        let mut lineage = self.lineage.write();
        lineage.set_branch_head(self.state.branch_id, revision);

        Ok(())
    }

    /// Fork one child world from one stored revision.
    fn fork_from_revision(&mut self, revision: Revision, name: String) -> RuntimeResult<World> {
        // resolve the retained fork point before mutating lineage
        let (target_revision, head_revision) = {
            let lineage = self.lineage.read();
            (
                lineage.revision_state(revision)?,
                lineage.head_revision_for_branch(self.state.branch_id)?,
            )
        };
        let base_revision = self.nearest_image_revision(revision)?;
        let (base_revision, image, _) = self.revision_data(base_revision)?;
        let trace_image = self.trace_image(revision)?;

        // allocate the child branch after retained resolution is complete
        let child_branch = {
            let mut lineage = self.lineage.write();
            lineage.fork_branch(revision, name)?
        };

        // child trace: clone header but switch to the child branch
        let trace_header = self.fork_trace_header(child_branch.id);

        // direct live fork: current committed head with no uncommitted tail
        if revision == self.revision()
            && self.state.trace.log().next_sequence() == head_revision.sequence
            && let Some(child) =
                self.try_fork_live_child(child_branch.id, trace_header.clone(), &trace_image)?
        {
            return Ok(child);
        }

        // child world: fresh mutable state over shared lineage data
        let mut child = self.fork_child_world(child_branch.id, trace_header)?;

        // restore the child to the fork checkpoint
        let image =
            self.revision_image(&target_revision, &base_revision, &image, &trace_image, None)?;
        child.restore_image(&image, None)?;
        child.state.trace.restore_image(&trace_image)?;
        child.state.trace.set_branch_id(child.state.branch_id);

        Ok(child)
    }

    /// Build replay options for one suffix-replay world.
    fn replay_runtime_options(&self) -> RuntimeOptions {
        let header = self.state.trace.log().header();
        let replay_payload = match header.replay_payload {
            BindingReplayPayload::Results => ReplayPayloadMode::ResultsOnly,
            BindingReplayPayload::ArgumentsAndResults => ReplayPayloadMode::ArgumentsAndResults,
        };
        let replay_chunk_size_mb = if header.max_chunk_bytes == 0 {
            None
        } else {
            Some(header.max_chunk_bytes / (1024 * 1024))
        };

        let mut options = RuntimeOptions::default();
        options.set_execution_mode(ExecutionMode::Replay);
        options.trace.chunk_size_mb = replay_chunk_size_mb;
        options.trace.payload = replay_payload;

        options
    }

    /// Restore one exact image for one committed revision.
    pub(super) fn revision_image(
        &self,
        target_revision: &RevisionState,
        base_revision: &RevisionState,
        image: &WorldImage,
        trace_image: &TraceImage,
        rebinders: Option<&ResourceRebinders>,
    ) -> RuntimeResult<WorldImage> {
        if base_revision.branch_id == target_revision.branch_id
            && base_revision.sequence == target_revision.sequence
        {
            return Ok(image.clone());
        }

        let options = self.replay_runtime_options();
        let mut replay_world = World::new_at_branch(target_revision.branch_id, &options, None)?;
        replay_world.restore_image(image, rebinders)?;
        replay_world.state.trace.restore_replay_image(trace_image)?;
        replay_world
            .state
            .trace
            .set_branch_id(target_revision.branch_id);
        replay_world
            .state
            .trace
            .seek_sequence(base_revision.sequence)?;
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
        base_revision: &RevisionState,
        image: &WorldImage,
        rebinders: Option<&ResourceRebinders>,
    ) -> RuntimeResult<WorldImage> {
        if base_revision.sequence == moment.sequence {
            return Ok(image.clone());
        }

        let options = self.replay_runtime_options();
        let mut replay_world = World::new_at_branch(moment.branch_id, &options, None)?;
        replay_world.restore_image(image, rebinders)?;
        let replay_trace = Trace::from_log(ExecutionMode::Replay, self.state.trace.log().clone());
        replay_trace.set_branch_id(moment.branch_id);
        replay_trace.seek_sequence(base_revision.sequence)?;
        replay_world.replay_to(&replay_trace, moment.sequence, rebinders)?;

        replay_world.capture_image(CaptureMode::Suspend)
    }

    /// Materialize one exact image for one committed moment.
    pub(crate) fn image_at_moment(&self, moment: Moment) -> RuntimeResult<WorldImage> {
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
            let base_revision = self.nearest_image_revision(anchor_revision)?;
            let (base_revision, image, _) = self.revision_data(base_revision)?;

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
                TraceRecord::Command(command) => {
                    self.apply_command(command)?;
                }
                TraceRecord::Anchor(_) => {}
                TraceRecord::Outcome(
                    outcome @ (Outcome::RuntimeSpawned { .. } | Outcome::WorkerSpawned { .. }),
                ) => {
                    match outcome {
                        // runtime restore
                        Outcome::RuntimeSpawned {
                            runtime_id,
                            runtime_name,
                            runtime_labels,
                            runtime,
                            workers,
                        } => {
                            self.install_runtime_image(
                                runtime_id,
                                runtime_name,
                                runtime_labels,
                                &runtime,
                                &workers,
                                rebinders,
                            )?;
                        }

                        // worker restore
                        Outcome::WorkerSpawned {
                            runtime_id,
                            worker_id,
                            worker_name,
                            worker_labels,
                            worker,
                        } => {
                            self.install_worker_image(
                                runtime_id,
                                worker_id,
                                worker_name,
                                worker_labels,
                                &worker,
                                rebinders,
                            )?;
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
        let mut header = self.state.trace.log().header();
        header.branch_id = branch_id;
        header
    }

    /// Build one fresh child-world shell for one forked branch.
    fn fork_child_world(
        &self,
        branch_id: BranchId,
        trace_header: TraceHeader,
    ) -> RuntimeResult<World> {
        // parent execution mode
        let trace_mode = self.state.trace.mode();

        // parent clock and randomness policy
        let clock = self.state.clock.clone();
        let random = Random::new(self.state.random.root_seed());
        // state restored by the image replay
        let state = WorldState {
            branch_id,
            time_mode: self.state.time_mode,
            random_mode: self.state.random_mode,
            simulation: Default::default(),
            policy: self.state.policy.clone(),
            next_runtime_id: 0,
            next_worker_id: 0,
            topology: Default::default(),
            resources: Default::default(),
            clock,
            random,
            trace: Trace::new(trace_mode, trace_header),
            observations: Observations::default(),
        };

        Ok(World {
            runtimes: Default::default(),
            state,
            lineage: self.lineage.clone(),
        })
    }

    /// Try to fork one live child world from the current committed branch head.
    fn try_fork_live_child(
        &mut self,
        branch_id: BranchId,
        trace_header: TraceHeader,
        trace_image: &TraceImage,
    ) -> RuntimeResult<Option<World>> {
        self.quiesce_shared_gc();

        let result = (|| {
            // direct live fork still requires all runtimes to be quiescent
            let execution_mode = self.state.trace.mode();
            let collector = self.lineage.read().collector();
            let mut runtimes = BTreeMap::new();
            for (runtime_id, runtime) in &mut self.runtimes {
                let Some(runtime) = runtime.try_fork(execution_mode, collector.clone())? else {
                    return Ok(None);
                };
                runtimes.insert(*runtime_id, Box::new(runtime));
            }

            // fork branch-local time and random state
            let clock = self.state.clock.clone();
            let random = self.state.random.fork()?;

            // rebuild one live child trace over the retained head image
            let trace = Trace::new(execution_mode, trace_header);
            trace.restore_image(trace_image)?;
            trace.set_branch_id(branch_id);

            let state = WorldState {
                branch_id,
                time_mode: self.state.time_mode,
                random_mode: self.state.random_mode,
                simulation: self.state.simulation.clone(),
                policy: self.state.policy.clone(),
                next_runtime_id: self.state.next_runtime_id,
                next_worker_id: self.state.next_worker_id,
                topology: self.state.topology.clone(),
                resources: self.state.resources.clone(),
                clock,
                random,
                trace,
                observations: Observations::default(),
            };

            Ok(Some(World {
                runtimes,
                state,
                lineage: self.lineage.clone(),
            }))
        })();

        self.resume_shared_gc();

        result
    }
}
