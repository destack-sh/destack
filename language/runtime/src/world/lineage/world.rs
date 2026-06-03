use std::collections::BTreeMap;
use std::sync::Arc;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::binding::BindingReplayPayload;
use crate::host::core::HostQueue;
use crate::host::poller::create_host_poller;
use crate::host::resource::ResourceRebinders;
use crate::runtime::random::Random;
use crate::runtime::time::Instant;
use crate::world::trace::{
    Observations, Outcome, Trace, TraceHeader, TraceImage, TraceRecord, TraceSequence,
};
use destack_core::CaptureMode;
use destack_workspace::{ExecutionMode, ReplayPayloadMode, RuntimeOptions};

use crate::world::{World, WorldImage, WorldSnapshot, WorldState};

use super::{BranchId, Checkpoint, Moment, Revision, RevisionId};

impl World {
    /// Restore this branch to one specific revision.
    pub fn rewind_revision(&mut self, revision_id: RevisionId) -> RuntimeResult<()> {
        let (target_revision, base_revision, image, trace_image) = {
            let lineage = self.lineage.read();
            let target_revision = lineage.revision(revision_id)?;
            let base_revision = self.nearest_image_revision(revision_id)?;
            let (base_revision, image, _) = self.revision_data(base_revision)?;
            let trace_image = self.trace_image(revision_id)?;

            (target_revision, base_revision, image, trace_image)
        };

        let image =
            self.revision_image(&target_revision, &base_revision, &image, &trace_image, None)?;

        self.restore_revision_image(revision_id, &image, &trace_image, None)
    }

    /// Restore this branch to one specific moment.
    pub fn restore_moment(&mut self, moment: Moment) -> RuntimeResult<()> {
        if moment.branch_id != self.state.branch_id {
            return Err(RuntimeError::moment_branch_mismatch(
                moment.branch_id.get(),
                self.state.branch_id.get(),
            )
            .boxed());
        }

        let (anchor_revision, base_revision, image) = {
            let lineage = self.lineage.read();
            let head_revision = lineage.head_revision(moment.branch_id)?;
            if head_revision.sequence.get() < moment.sequence.get() {
                return Err(RuntimeError::moment_not_found(
                    moment.branch_id.get(),
                    moment.sequence.get(),
                )
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
        revision_id: RevisionId,
        name: impl Into<String>,
    ) -> RuntimeResult<World> {
        self.fork_from_revision(revision_id, name.into())
    }

    /// Commit one new revision for the active branch.
    pub(super) fn commit(
        &mut self,
        mode: CaptureMode,
        checkpoint_name: Option<String>,
    ) -> RuntimeResult<(RevisionId, Revision, Arc<WorldImage>, Option<Checkpoint>)> {
        // capture one exact world image first
        let image = self.capture_image(mode)?;
        let trace_image = self.state.trace.capture_image();

        // commit the revision in one authoritative lineage update
        let retain_image = self.state.trace.mode() != ExecutionMode::Record
            || checkpoint_name.is_some()
            || !matches!(mode, CaptureMode::Suspend);
        let image_id = self.lineage.write().allocate_image_id()?;
        let mut image = image;

        if retain_image {
            self.lineage
                .write()
                .retain_image_payloads(self.state.branch_id, &mut image)?;
        }

        let image = Arc::new(image);
        let trace_image = Arc::new(trace_image);
        let (revision_id, revision, checkpoint) = {
            let mut lineage = self.lineage.write();
            lineage.commit_revision(
                self.state.branch_id,
                image_id,
                trace_image.next_sequence()?,
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

            lineage.insert_trace_image(revision_id, trace_image);
        }

        let observations = self
            .state
            .observations
            .drain_through(self.state.branch_id, revision.sequence);

        if !observations.is_empty() {
            let mut lineage = self.lineage.write();
            lineage.record_observations(self.state.branch_id, observations);
        }

        Ok((revision_id, revision, image, checkpoint))
    }

    /// Capture one suspendable live revision for later local resume.
    pub fn suspend(&mut self) -> RuntimeResult<RevisionId> {
        let committed = self.commit(CaptureMode::Suspend, None)?;

        Ok(committed.0)
    }

    /// Capture one hibernation snapshot for durable restore.
    pub fn hibernate_snapshot(&mut self) -> RuntimeResult<WorldSnapshot> {
        let revision = self.commit(CaptureMode::Hibernate, None)?.0;

        self.snapshot_revision(revision)
    }

    /// Restore one specific revision image and update branch lineage.
    pub(crate) fn restore_revision_image(
        &mut self,
        revision_id: RevisionId,
        image: &WorldImage,
        trace_image: &TraceImage,
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<()> {
        self.restore_image(image, rebind_context)?;
        self.state.trace.restore_image(trace_image)?;
        self.state.trace.set_branch_id(self.state.branch_id);

        let mut lineage = self.lineage.write();
        lineage.set_branch_head(self.state.branch_id, revision_id)?;

        Ok(())
    }

    /// Fork one child world from one stored revision.
    pub(super) fn fork_from_revision(
        &mut self,
        revision_id: RevisionId,
        name: String,
    ) -> RuntimeResult<World> {
        // resolve the retained fork point before mutating lineage
        let (target_revision, head_revision) = {
            let lineage = self.lineage.read();
            (
                lineage.revision(revision_id)?,
                lineage.head_revision(self.state.branch_id)?,
            )
        };
        let base_revision = self.nearest_image_revision(revision_id)?;
        let (base_revision, image, _) = self.revision_data(base_revision)?;
        let trace_image = self.trace_image(revision_id)?;

        // allocate the child branch after retained resolution is complete
        let child_branch = {
            let mut lineage = self.lineage.write();
            lineage.fork_branch(revision_id, name)?
        };

        // child trace: clone header but switch to the child branch
        let trace_header = self.fork_trace_header(child_branch.id);

        // direct live fork: current committed head with no uncommitted tail
        if revision_id == self.revision_id()?
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
        let replay_chunk_size_mb = if header.max_chunk_size_bytes == 0 {
            None
        } else {
            Some(header.max_chunk_size_bytes / (1024 * 1024))
        };

        let mut options = RuntimeOptions::default();
        options.execution.mode = ExecutionMode::Replay;
        options.trace.chunk_size_mb = replay_chunk_size_mb;
        options.trace.payload = replay_payload;

        options
    }

    /// Restore one exact image for one committed revision.
    pub(crate) fn revision_image(
        &self,
        target_revision: &Revision,
        base_revision: &Revision,
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
        let environment = self.state.trace.log().header().environment.clone();
        let mut replay_world =
            World::empty(target_revision.branch_id, &options, environment, None)?;
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
        base_revision: &Revision,
        image: &WorldImage,
        rebinders: Option<&ResourceRebinders>,
    ) -> RuntimeResult<WorldImage> {
        if base_revision.sequence == moment.sequence {
            return Ok(image.clone());
        }

        let options = self.replay_runtime_options();
        let environment = self.state.trace.log().header().environment.clone();
        let mut replay_world = World::empty(moment.branch_id, &options, environment, None)?;
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
            let head_revision = lineage.head_revision(moment.branch_id)?;
            if head_revision.sequence.get() < moment.sequence.get() {
                return Err(RuntimeError::moment_not_found(
                    moment.branch_id.get(),
                    moment.sequence.get(),
                )
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
            let event = trace
                .next_event()?
                .ok_or_else(|| RuntimeError::trace_exhausted(target_sequence.get()).boxed())?;

            match event {
                TraceRecord::Mutation(mutation) => {
                    self.apply_mutation(mutation)?;
                }
                TraceRecord::Entrypoint(invocation) => {
                    let _ = self.execute_entrypoint(
                        invocation.runtime_id,
                        &invocation.entry,
                        &invocation.args,
                    )?;
                }
                TraceRecord::Label(_) => {}
                TraceRecord::Outcome(
                    outcome @ (Outcome::RuntimeSpawned { .. } | Outcome::WorkerSpawned { .. }),
                ) => {
                    match outcome {
                        // runtime restore
                        Outcome::RuntimeSpawned {
                            runtime_id,
                            runtime_entity,
                            runtime,
                            workers,
                        } => {
                            self.restore_runtime_image(
                                runtime_id,
                                runtime_entity,
                                &runtime,
                                &workers,
                                rebinders,
                            )?;
                        }

                        // worker restore
                        Outcome::WorkerSpawned {
                            runtime_id,
                            worker_id,
                            worker_entity,
                            worker,
                        } => {
                            self.restore_worker_image(
                                runtime_id,
                                worker_id,
                                worker_entity,
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
        let random = Random::with_source(self.state.random.source(), self.state.random.root_seed());
        // state restored by the image replay
        let state = WorldState {
            branch_id,
            policy: self.state.policy.clone(),
            next_runtime_id: 0,
            next_worker_id: 0,
            topology: Default::default(),
            clock,
            random,
            trace: Trace::new(trace_mode, trace_header),
            observations: Observations::default(),
        };

        let poller_backend = self.poller.backend();
        let poller = create_host_poller(poller_backend)?;

        Ok(World {
            host: self.host.clone(),
            host_queue: HostQueue::new(),
            poller,
            runtimes: Default::default(),
            memory: self.memory.clone(),
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
            let collector = self.memory.shared_collector.clone();
            let mut runtimes = BTreeMap::new();
            for (runtime_id, runtime) in &mut self.runtimes {
                let Some(runtime) = runtime.try_fork(execution_mode, collector.clone())? else {
                    return Ok(None);
                };
                runtimes.insert(*runtime_id, runtime);
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
                policy: self.state.policy.clone(),
                next_runtime_id: self.state.next_runtime_id,
                next_worker_id: self.state.next_worker_id,
                topology: self.state.topology.clone(),
                clock,
                random,
                trace,
                observations: Observations::default(),
            };

            let poller_backend = self.poller.backend();
            let poller = create_host_poller(poller_backend)?;

            Ok(Some(World {
                host: self.host.clone(),
                host_queue: HostQueue::new(),
                poller,
                runtimes,
                memory: self.memory.clone(),
                state,
                lineage: self.lineage.clone(),
            }))
        })();

        self.resume_shared_gc();

        result
    }
}
