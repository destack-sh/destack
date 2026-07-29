use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use destack_core::CaptureMode;
use destack_memory::MemoryImage;
use destack_program as program;
use destack_program::FrameStateId;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::ResourceId;
use crate::runtime::RuntimeImage;
use crate::worker::scheduler::{Invocation, RunnableId};
use crate::worker::{WorkerId, WorkerImage};
use crate::world::debug::Debugger;
use crate::world::policy::Policy;
use crate::world::topology::{
    Edge, EdgeDefinition, EdgeKind, Entity, EntityDefinition, EntityKind, LabelSet, RuntimeId,
};
use crate::world::{World, WorldImage};

use super::Moment;

/// World state selected for inspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// Inspect the current live world state.
    Now,
    /// Inspect one committed moment.
    Moment(Moment),
}

/// One branch divergence between two committed branch heads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Divergence {
    /// The shared base moment before the branches diverged.
    pub base: Moment,
    /// The committed head moment on the left branch.
    pub left: Moment,
    /// The committed head moment on the right branch.
    pub right: Moment,
}

/// One frame visible inside a world view.
#[derive(Clone)]
pub struct FrameView<'a> {
    /// Runtime that owns the frame.
    pub runtime_id: RuntimeId,
    /// Worker that owns the frame.
    pub worker_id: WorkerId,
    /// Retained execution source for the frame.
    pub source: FrameSource,
    /// Captured frame state.
    state: FrameStateId,
    /// Canonical live frame bytes.
    bytes: Cow<'a, [u8]>,
}

/// Retained execution source for one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameSource {
    /// Execution retained at one debugger stop.
    Stopped {
        /// Stopped runnable identifier.
        runnable_id: RunnableId,
    },
    /// Continuation queued as one task.
    Task {
        /// Queued task identifier.
        runnable_id: RunnableId,
    },
    /// Continuation queued as one microtask.
    Microtask {
        /// Queued microtask identifier.
        runnable_id: RunnableId,
    },
    /// Continuation suspended on one language waiter.
    Waiter {
        /// Waiter that retains the continuation.
        waiter: program::Waiter,
    },
    /// First-class continuation retained by the machine.
    Continuation {
        /// Runtime continuation identity.
        continuation_id: program::ContinuationId,
    },
}

impl WorkerImage {
    /// Return all frames retained by one worker image.
    fn frames<'a>(
        &'a self,
        memory: &MemoryImage,
        program: &program::Program,
    ) -> RuntimeResult<Vec<FrameView<'a>>> {
        let mut frames = Vec::new();

        // materialize the stopped physical stack
        self.append_machine_frames(memory, program, &mut frames)?;

        // expose scheduler-owned continuations under their exact owners
        if let Some(snapshot) = self.event_loop.active() {
            for runnable in &snapshot.tasks {
                let source = FrameSource::Task {
                    runnable_id: runnable.id,
                };
                self.append_invocation_frames(program, source, &runnable.invocation, &mut frames)?;
            }
            for runnable in &snapshot.microtasks {
                let source = FrameSource::Microtask {
                    runnable_id: runnable.id,
                };
                self.append_invocation_frames(program, source, &runnable.invocation, &mut frames)?;
            }
            for (waiter, continuation) in snapshot.waiters() {
                let source = FrameSource::Waiter { waiter };
                self.append_continuation_frames(program, source, continuation, &mut frames)?;
            }
        }

        // expose first-class continuation handles retained by the machine
        for (continuation_id, continuation) in self.machine.continuations() {
            let source = FrameSource::Continuation { continuation_id };
            self.append_continuation_frames(program, source, continuation, &mut frames)?;
        }

        Ok(frames)
    }

    /// Append the physical call stack retained at one debugger stop.
    fn append_machine_frames(
        &self,
        memory: &MemoryImage,
        program: &program::Program,
        frames: &mut Vec<FrameView<'_>>,
    ) -> RuntimeResult<()> {
        let frame_count = self.machine.frame_count();
        if frame_count == 0 {
            return Ok(());
        }
        let Some(stop) = self.stop.as_ref() else {
            return Err(RuntimeError::inconsistent_image(format!(
                "worker {} retains frames without a stopped runnable",
                self.worker_id.0
            ))
            .boxed());
        };
        let source = FrameSource::Stopped {
            runnable_id: stop.id,
        };

        // project each physical frame into its canonical live layout
        for index in 0..frame_count {
            let Some(state) = self.machine.frame_state(index) else {
                return Err(RuntimeError::inconsistent_image(format!(
                    "worker {} frame {index} has no frame state",
                    self.worker_id.0
                ))
                .boxed());
            };
            let bytes = self.machine.frame_bytes(memory, program, index)?;

            frames.push(FrameView::new(self, source, state, Cow::Owned(bytes)));
        }

        Ok(())
    }

    /// Append the continuation carried by one queued invocation when present.
    fn append_invocation_frames<'a>(
        &'a self,
        program: &program::Program,
        source: FrameSource,
        invocation: &'a Invocation,
        frames: &mut Vec<FrameView<'a>>,
    ) -> RuntimeResult<()> {
        let Some(continuation) = invocation.continuation() else {
            return Ok(());
        };

        self.append_continuation_frames(program, source, continuation, frames)
    }

    /// Append one canonical continuation call chain.
    fn append_continuation_frames<'a>(
        &'a self,
        program: &program::Program,
        source: FrameSource,
        continuation: &'a program::Continuation,
        frames: &mut Vec<FrameView<'a>>,
    ) -> RuntimeResult<()> {
        let mut byte_offset = 0usize;

        // split canonical bytes through each retained frame layout
        for state in continuation.states() {
            let linked = program.frame_state(*state).ok_or_else(|| {
                RuntimeError::inconsistent_image(format!(
                    "worker {} continuation references an undefined frame state",
                    self.worker_id.0
                ))
                .boxed()
            })?;
            let layout = program.frame_layout(linked.layout).ok_or_else(|| {
                RuntimeError::inconsistent_image(format!(
                    "worker {} continuation references an undefined frame layout",
                    self.worker_id.0
                ))
                .boxed()
            })?;
            byte_offset = byte_offset.next_multiple_of(layout.alignment() as usize);
            let end = byte_offset + layout.byte_len() as usize;
            let bytes = continuation.bytes().get(byte_offset..end).ok_or_else(|| {
                RuntimeError::inconsistent_image(format!(
                    "worker {} continuation frame exceeds its canonical bytes",
                    self.worker_id.0
                ))
                .boxed()
            })?;
            frames.push(FrameView::new(self, source, *state, Cow::Borrowed(bytes)));
            byte_offset = end;
        }

        // reject trailing bytes that do not belong to any retained frame
        if byte_offset != continuation.bytes().len() {
            return Err(RuntimeError::inconsistent_image(format!(
                "worker {} continuation has unclaimed canonical bytes",
                self.worker_id.0
            ))
            .boxed());
        }

        Ok(())
    }
}

impl<'a> FrameView<'a> {
    /// Create one retained frame view.
    fn new(
        worker: &WorkerImage,
        source: FrameSource,
        state: FrameStateId,
        bytes: Cow<'a, [u8]>,
    ) -> Self {
        Self {
            runtime_id: worker.runtime_id,
            worker_id: worker.worker_id,
            source,
            state,
            bytes,
        }
    }

    /// Return the captured frame state id.
    pub const fn frame_state(&self) -> FrameStateId {
        self.state
    }

    /// Return the captured frame byte width.
    pub fn byte_len(&self) -> usize {
        self.bytes.len()
    }

    /// Return the canonical live frame bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl fmt::Debug for FrameView<'_> {
    /// Format one frame view without traversing canonical frame bytes.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FrameView")
            .field("runtime_id", &self.runtime_id)
            .field("worker_id", &self.worker_id)
            .field("source", &self.source)
            .field("frame_state", &self.frame_state())
            .finish()
    }
}

impl World {
    /// Return one exact world view.
    pub fn view(&mut self, view: View) -> RuntimeResult<WorldView> {
        match view {
            View::Now => {
                let moment = self.state.moment();
                let image = self.capture_image(CaptureMode::Suspend)?;

                Ok(WorldView::new(moment, image))
            }
            View::Moment(moment) => self.lineage().view(moment),
        }
    }
}

/// One inspectable committed world image view at one exact moment.
#[derive(Debug, Clone)]
pub struct WorldView {
    /// The moment this view represents.
    moment: Moment,
    /// The exact materialized image for this view.
    image: WorldImage,
}

impl WorldView {
    /// Create one committed world view from one exact moment and image.
    pub(super) fn new(moment: Moment, image: WorldImage) -> Self {
        Self { moment, image }
    }

    /// Return the exact moment this view represents.
    pub fn moment(&self) -> Moment {
        self.moment
    }

    /// Return the underlying materialized image.
    pub fn image(&self) -> &WorldImage {
        &self.image
    }

    /// Return the captured world memory map.
    pub fn memory(&self) -> &MemoryImage {
        self.image.memory()
    }

    /// Return the policy visible at this moment.
    pub fn policy(&self) -> &Policy {
        self.image.policy()
    }

    /// Return the debugger configuration visible at this moment.
    pub fn debugger(&self) -> &Debugger {
        self.image.debugger()
    }

    /// Return all retained runtime images visible at this moment.
    pub fn runtimes(&self) -> &BTreeMap<RuntimeId, Arc<RuntimeImage>> {
        self.image.runtimes()
    }

    /// Return all retained worker images visible at this moment.
    pub fn workers(&self) -> &BTreeMap<WorkerId, Arc<WorkerImage>> {
        self.image.workers()
    }

    /// Return all runtime ids visible at this moment.
    pub fn runtime_ids(&self) -> impl Iterator<Item = RuntimeId> + '_ {
        self.image.runtimes.keys().copied()
    }

    /// Return all worker ids visible at this moment.
    pub fn worker_ids(&self) -> impl Iterator<Item = WorkerId> + '_ {
        self.image.workers.keys().copied()
    }

    /// Return all frames visible at this moment.
    pub fn frames(&self) -> RuntimeResult<Vec<FrameView<'_>>> {
        let mut frames = Vec::new();

        // preserve worker order while resolving every referenced runtime loudly
        for worker in self.image.workers.values() {
            let runtime = self.runtime(worker.runtime_id)?;
            frames.extend(worker.frames(self.image.memory(), runtime.program.as_ref())?);
        }

        Ok(frames)
    }

    /// Return all frames retained by one worker at this moment.
    pub fn worker_frames(&self, worker_id: WorkerId) -> RuntimeResult<Vec<FrameView<'_>>> {
        let worker = self.worker(worker_id)?;
        let runtime = self.runtime(worker.runtime_id)?;

        worker.frames(self.image.memory(), runtime.program.as_ref())
    }

    /// Return the number of runtimes visible at this moment.
    pub fn runtime_count(&self) -> usize {
        self.image.runtime_count()
    }

    /// Return the number of workers visible at this moment.
    pub fn worker_count(&self) -> usize {
        self.image.worker_count()
    }

    /// Return the number of logical resources visible at this moment.
    pub fn resource_count(&self) -> usize {
        self.image.resource_count()
    }

    /// Return the number of topology entities visible at this moment.
    pub fn entity_count(&self) -> usize {
        self.image.entity_count()
    }

    /// Return the number of topology edges visible at this moment.
    pub fn edge_count(&self) -> usize {
        self.image.edge_count()
    }

    /// Report whether one runtime exists at this moment.
    pub fn has_runtime(&self, runtime_id: RuntimeId) -> bool {
        self.image.has_runtime(runtime_id)
    }

    /// Report whether one worker exists at this moment.
    pub fn has_worker(&self, worker_id: WorkerId) -> bool {
        self.image.has_worker(worker_id)
    }

    /// Report whether one resource exists at this moment.
    pub fn has_resource(&self, resource_id: ResourceId) -> bool {
        self.image.has_resource(resource_id)
    }

    /// Report whether one topology entity exists at this moment.
    pub fn has_entity(&self, entity_id: &str) -> bool {
        self.image.has_entity(entity_id)
    }

    /// Report whether one topology edge exists at this moment.
    pub fn has_edge(&self, edge_id: &str) -> bool {
        self.image.has_edge(edge_id)
    }

    /// Return one runtime image by id.
    pub fn runtime(&self, runtime_id: RuntimeId) -> RuntimeResult<&RuntimeImage> {
        self.image.runtime(runtime_id)
    }

    /// Return the program instantiated by one runtime at this moment.
    pub fn program(&self, runtime_id: RuntimeId) -> RuntimeResult<&program::Program> {
        let runtime = self.runtime(runtime_id)?;

        Ok(runtime.program.as_ref())
    }

    /// Return labels for one runtime visible at this moment.
    pub fn runtime_labels(&self, runtime_id: RuntimeId) -> RuntimeResult<&LabelSet> {
        self.image.runtime_labels(runtime_id)
    }

    /// Return one worker image by id.
    pub fn worker(&self, worker_id: WorkerId) -> RuntimeResult<&WorkerImage> {
        self.image.worker(worker_id)
    }

    /// Return labels for one worker visible at this moment.
    pub fn worker_labels(&self, worker_id: WorkerId) -> RuntimeResult<&LabelSet> {
        self.image.worker_labels(worker_id)
    }

    /// Return one topology entity by id when present.
    pub fn entity(&self, entity_id: &str) -> Option<&Entity> {
        self.image.entity(entity_id)
    }

    /// Return all topology entity kinds visible at this moment.
    pub fn entity_kinds(&self) -> BTreeMap<EntityKind, EntityDefinition> {
        self.image.topology.entity_kinds()
    }

    /// Return one topology entity kind by id when present.
    pub fn entity_kind(&self, kind_id: &str) -> Option<&EntityDefinition> {
        self.image.topology.entity_kind(kind_id)
    }

    /// Return one topology edge by id when present.
    pub fn edge(&self, edge_id: &str) -> Option<&Edge> {
        self.image.edge(edge_id)
    }

    /// Return all topology edge kinds visible at this moment.
    pub fn edge_kinds(&self) -> BTreeMap<EdgeKind, EdgeDefinition> {
        self.image.topology.edge_kinds()
    }

    /// Return one topology edge kind by id when present.
    pub fn edge_kind(&self, kind_id: &str) -> Option<&EdgeDefinition> {
        self.image.topology.edge_kind(kind_id)
    }
}
