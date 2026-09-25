use std::collections::BTreeMap;
use std::sync::Arc;

use tspp_core::CaptureMode;
use tspp_repository::{ExecutionMode, WorldOptions};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::HostQueue;
use crate::host::poller::HostPoller;
use crate::world::observation::ObservationLog;
use crate::world::random::Random;
use crate::world::time::Instant;
use crate::world::topology::LabelSet;
use crate::world::trace::{Trace, TraceHeader, TraceImage, TraceLog, TraceSequence};
use crate::world::{RestoreContext, World, WorldImage, WorldSnapshot, WorldState};

use super::{BranchId, Image, Moment, MomentSequence, Revision, RevisionId};

/// One committed World revision and its captured state.
pub(super) struct Commit {
    /// Committed revision identifier.
    pub revision_id: RevisionId,
    /// Retained Image when the captured state remains materialized.
    pub image: Option<Image>,
    /// Captured World state.
    pub world_image: Arc<WorldImage>,
}

impl World {
    /// Restore this branch to one specific moment.
    pub fn rewind(&mut self, moment: Moment, restore: RestoreContext<'_>) -> RuntimeResult<()> {
        if moment.branch_id != self.state.branch_id {
            return Err(RuntimeError::moment_branch_mismatch(
                moment.branch_id.get(),
                self.state.branch_id.get(),
            )
            .boxed());
        }

        let (anchor_revision, target_revision, base_revision, image) = {
            let lineage = self.lineage.read();
            let head_revision = lineage.head_revision(moment.branch_id)?;
            if head_revision.sequence.get() < moment.sequence.get() {
                return Err(RuntimeError::moment_not_found(
                    moment.branch_id.get(),
                    moment.sequence.get(),
                )
                .boxed());
            }
            let anchor_revision_id = lineage.revision_at(moment.branch_id, moment.sequence)?;
            let target_revision = lineage.revision(anchor_revision_id)?;
            let base_revision = self.nearest_image_revision(anchor_revision_id)?;
            let (base_revision, image, _) = self.revision_data(base_revision)?;

            (anchor_revision_id, target_revision, base_revision, image)
        };

        let image = self.moment_image(moment, &base_revision, &image, restore)?;
        let trace_image = self
            .state
            .trace
            .capture_image_through(target_revision.trace_sequence)?;

        self.restore_revision_image(anchor_revision, &image, &trace_image, restore)
    }

    /// Fork one child world from one specific moment.
    pub fn fork(
        &mut self,
        moment: Moment,
        name: impl Into<String>,
        restore: RestoreContext<'_>,
    ) -> RuntimeResult<World> {
        self.fork_from_moment(moment, name.into(), restore)
    }

    /// Commit one new revision for the active branch.
    pub(super) fn commit(
        &mut self,
        mode: CaptureMode,
        name: Option<String>,
    ) -> RuntimeResult<Commit> {
        // capture one exact world image first
        let image = self.capture_image(mode)?;
        let trace_image = self.state.trace.capture_image();

        // commit the revision in one authoritative lineage update
        let is_retained = self.state.trace.mode() != ExecutionMode::Record
            || name.is_some()
            || !matches!(mode, CaptureMode::Suspend);
        let image_id = self.lineage.write().allocate_image_id()?;
        let mut image = image;

        if is_retained {
            self.lineage
                .write()
                .retain_image_payloads(self.state.branch_id, &mut image)?;
        }

        let image = Arc::new(image);
        let trace_image = Arc::new(trace_image);
        let (revision_id, revision) = {
            let mut lineage = self.lineage.write();
            lineage.commit_revision(
                self.state.branch_id,
                image_id,
                self.state.moment,
                trace_image.next_sequence()?,
                Instant::from_nanos(self.wall()),
                Instant::from_nanos(self.mono()),
            )?
        };

        let retained_image = if is_retained {
            Some(Image {
                id: image_id,
                moment: revision.moment(),
                name,
                labels: LabelSet::new(),
            })
        } else {
            None
        };

        {
            let mut lineage = self.lineage.write();
            if let Some(retained_image) = retained_image.clone() {
                lineage.insert_image(retained_image, revision_id, image.clone());
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

        Ok(Commit {
            revision_id,
            image: retained_image,
            world_image: image,
        })
    }

    /// Capture one suspendable live revision for later local resume.
    pub fn suspend(&mut self) -> RuntimeResult<RevisionId> {
        let committed = self.commit(CaptureMode::Suspend, None)?;

        Ok(committed.revision_id)
    }

    /// Capture one hibernation snapshot for durable restore.
    pub fn hibernate_snapshot(&mut self) -> RuntimeResult<WorldSnapshot> {
        let revision = self.commit(CaptureMode::Hibernate, None)?.revision_id;

        self.snapshot_revision(revision, RestoreContext::empty())
    }

    /// Restore one specific revision image and update branch lineage.
    pub(crate) fn restore_revision_image(
        &mut self,
        revision_id: RevisionId,
        image: &WorldImage,
        trace_image: &TraceImage,
        restore: RestoreContext<'_>,
    ) -> RuntimeResult<()> {
        self.restore_image(image, restore)?;
        self.state.trace.restore_image(trace_image)?;
        self.state.trace.set_branch_id(self.state.branch_id);

        let mut lineage = self.lineage.write();
        lineage.set_branch_head(self.state.branch_id, revision_id)?;

        Ok(())
    }

    /// Fork one child world from one stored moment.
    fn fork_from_moment(
        &mut self,
        moment: Moment,
        name: String,
        restore: RestoreContext<'_>,
    ) -> RuntimeResult<World> {
        // resolve the retained fork point before mutating lineage
        let (anchor_revision_id, target_revision, base_revision, image, head_revision) = {
            let lineage = self.lineage.read();
            let head_revision = lineage.head_revision(moment.branch_id)?;
            if head_revision.sequence.get() < moment.sequence.get() {
                return Err(RuntimeError::moment_not_found(
                    moment.branch_id.get(),
                    moment.sequence.get(),
                )
                .boxed());
            }
            let anchor_revision_id = lineage.revision_at(moment.branch_id, moment.sequence)?;
            let target_revision = lineage.revision(anchor_revision_id)?;
            let base_revision_id = self.nearest_image_revision(anchor_revision_id)?;
            let (base_revision, image, _) = self.revision_data(base_revision_id)?;

            (
                anchor_revision_id,
                target_revision,
                base_revision,
                image,
                head_revision,
            )
        };
        let mut image = self.moment_image(moment, &base_revision, &image, restore)?;
        let trace_image = self
            .state
            .trace
            .capture_image_through(target_revision.trace_sequence)?;

        // canonicalize retained payloads against the parent branch
        self.lineage
            .write()
            .retain_image_payloads(moment.branch_id, &mut image)?;
        let image = Arc::new(image);
        let trace_image = Arc::new(trace_image);

        // allocate the child branch after retained resolution is complete
        let child_branch = {
            let mut lineage = self.lineage.write();
            lineage.fork_branch_with_image(
                moment,
                anchor_revision_id,
                name,
                Instant::from_nanos(self.wall()),
                Instant::from_nanos(self.mono()),
                image.clone(),
                trace_image.clone(),
            )?
        };

        // child trace: clone header but switch to the child branch
        let trace_header = self.fork_trace_header(child_branch.id);

        // direct live fork: current committed head with no uncommitted tail
        if moment.branch_id == self.state.branch_id
            && anchor_revision_id == self.revision_id()?
            && self.state.trace.store().next_sequence() == head_revision.trace_sequence
            && let Some(child) =
                self.try_fork_live_child(child_branch.id, trace_header.clone(), &trace_image)?
        {
            return Ok(child);
        }

        // child world: fresh mutable state over shared lineage data
        let mut child = self.fork_child_world(child_branch.id, moment.sequence, trace_header)?;

        // restore the child to the fork moment
        child.restore_image(&image, restore)?;
        child.state.trace.restore_image(&trace_image)?;
        child.state.trace.set_branch_id(child.state.branch_id);

        Ok(child)
    }

    /// Build options for one suffix-replay world.
    fn replay_world_options(&self) -> WorldOptions {
        let header = self.state.trace.store().header();
        WorldOptions {
            mode: ExecutionMode::Replay,
            replay_payload: header.replay_payload.into(),
            ..Default::default()
        }
    }

    /// Restore one exact image for one committed revision.
    pub(crate) fn revision_image(
        &self,
        target_revision: &Revision,
        base_revision: &Revision,
        image: &WorldImage,
        trace_image: &TraceImage,
        restore: RestoreContext<'_>,
    ) -> RuntimeResult<WorldImage> {
        if base_revision.branch_id == target_revision.branch_id
            && base_revision.sequence == target_revision.sequence
        {
            return Ok(image.clone());
        }

        let options = self.replay_world_options();
        let environment = self.state.trace.store().header().environment.clone();
        let mut replay_world =
            World::empty(target_revision.branch_id, &options, environment, None)?;
        replay_world.restore_image(image, restore)?;
        replay_world.state.trace.restore_replay_image(trace_image)?;
        replay_world
            .state
            .trace
            .set_branch_id(target_revision.branch_id);
        replay_world
            .state
            .trace
            .seek_sequence(base_revision.trace_sequence)?;
        let replay_trace =
            TraceLog::from_store(ExecutionMode::Replay, replay_world.trace().store().clone());
        replay_trace.set_branch_id(target_revision.branch_id);
        replay_trace.seek_sequence(base_revision.trace_sequence)?;
        replay_world.replay_to(&replay_trace, target_revision.trace_sequence, restore)?;

        replay_world.capture_image(CaptureMode::Suspend)
    }

    /// Restore one exact image for one moment.
    fn moment_image(
        &self,
        moment: Moment,
        base_revision: &Revision,
        image: &WorldImage,
        restore: RestoreContext<'_>,
    ) -> RuntimeResult<WorldImage> {
        if base_revision.sequence == moment.sequence {
            return Ok(image.clone());
        }

        let options = self.replay_world_options();
        let environment = self.state.trace.store().header().environment.clone();
        let mut replay_world = World::empty(moment.branch_id, &options, environment, None)?;
        replay_world.restore_image(image, restore)?;
        let replay_trace =
            TraceLog::from_store(ExecutionMode::Replay, self.state.trace.store().clone());
        replay_trace.set_branch_id(moment.branch_id);
        replay_trace.seek_sequence(base_revision.trace_sequence)?;
        let target_revision = {
            let lineage = self.lineage.read();
            let revision_id = lineage.revision_at(moment.branch_id, moment.sequence)?;
            lineage.revision(revision_id)?
        };
        replay_world.replay_to(&replay_trace, target_revision.trace_sequence, restore)?;

        replay_world.capture_image(CaptureMode::Suspend)
    }

    /// Materialize one exact image for one committed moment.
    pub(crate) fn image_at_moment(
        &self,
        moment: Moment,
        restore: RestoreContext<'_>,
    ) -> RuntimeResult<WorldImage> {
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
            let anchor_revision = lineage.revision_at(moment.branch_id, moment.sequence)?;
            let base_revision = self.nearest_image_revision(anchor_revision)?;
            let (base_revision, image, _) = self.revision_data(base_revision)?;

            (anchor_revision, base_revision, image)
        };

        self.moment_image(moment, &base_revision, &image, restore)
    }

    /// Replay one trace through one requested sequence boundary.
    fn replay_to(
        &mut self,
        trace: &TraceLog,
        target_sequence: TraceSequence,
        restore: RestoreContext<'_>,
    ) -> RuntimeResult<()> {
        while trace.sequence()? != target_sequence {
            let entry = trace
                .next_entry()?
                .ok_or_else(|| RuntimeError::trace_exhausted(target_sequence.get()).boxed())?;

            match entry.trace {
                Trace::Mutation(mutation) => {
                    self.apply_mutation_with_restore(*mutation, restore)?;
                }
                Trace::Entrypoint(invocation) => {
                    self.execute_entrypoint(
                        invocation.runtime_id,
                        &invocation.entry,
                        &invocation.args,
                    )?;
                }
                Trace::Binding(_) | Trace::Clock(_) | Trace::Random(_) => {
                    return Err(RuntimeError::Internal {
                        message: "unexpected low-level trace entry escaped world replay"
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
        let mut header = self.state.trace.store().header();
        header.branch_id = branch_id;
        header
    }

    /// Build one fresh child-world shell for one forked branch.
    fn fork_child_world(
        &self,
        branch_id: BranchId,
        moment_sequence: MomentSequence,
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
            moment: moment_sequence,
            policy: self.state.policy.clone(),
            debugger: self.state.debugger.clone(),
            next_runtime_id: 0,
            next_worker_id: 0,
            topology: Default::default(),
            clock,
            random,
            trace: TraceLog::new(trace_mode, trace_header),
            observations: ObservationLog::default(),
        };

        let poller = HostPoller::open()?;

        Ok(World {
            host: self.host.clone(),
            host_queue: HostQueue::new(),
            poller,
            runtimes: Default::default(),
            next_runtime_cursor: 0,
            memory: self.memory.clone(),
            collector: self.collector.clone(),
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
            // fork World memory once before rebuilding dependent heap metadata
            let memory = Arc::new(self.memory.fork_lazy().map_err(Box::<RuntimeError>::from)?);

            // direct live fork still requires all runtimes to be quiescent
            let execution_mode = self.state.trace.mode();
            let replay_payload = trace_header.replay_payload;
            let collector = self.collector.clone();
            let mut runtimes = BTreeMap::new();
            for (runtime_id, runtime) in &mut self.runtimes {
                let Some(runtime) = runtime.try_fork(
                    memory.clone(),
                    execution_mode,
                    replay_payload,
                    collector.clone(),
                )?
                else {
                    return Ok(None);
                };
                runtimes.insert(*runtime_id, runtime);
            }

            // fork branch-local time and random state
            let clock = self.state.clock.clone();
            let random = self.state.random.fork()?;

            // rebuild one live child trace over the retained head image
            let trace = TraceLog::new(execution_mode, trace_header);
            trace.restore_image(trace_image)?;
            trace.set_branch_id(branch_id);

            let state = WorldState {
                branch_id,
                moment: self.state.moment,
                policy: self.state.policy.clone(),
                debugger: self.state.debugger.clone(),
                next_runtime_id: self.state.next_runtime_id,
                next_worker_id: self.state.next_worker_id,
                topology: self.state.topology.clone(),
                clock,
                random,
                trace,
                observations: ObservationLog::default(),
            };

            let poller = HostPoller::open()?;

            Ok(Some(World {
                host: self.host.clone(),
                host_queue: HostQueue::new(),
                poller,
                runtimes,
                next_runtime_cursor: self.next_runtime_cursor,
                memory,
                collector: self.collector.clone(),
                state,
                lineage: self.lineage.clone(),
            }))
        })();

        self.resume_shared_gc();

        result
    }
}
