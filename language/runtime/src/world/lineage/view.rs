use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use destack_core::CaptureMode;
use destack_memory::MemoryImage;
use destack_program as program;
use destack_program::FrameStateId;
use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::ResourceId;
use crate::runtime::RuntimeImage;
use crate::scheduler::RunnableId;
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
pub struct FrameView {
    /// Runtime that owns the frame.
    pub runtime_id: RuntimeId,
    /// Worker that owns the frame.
    pub worker_id: WorkerId,
    /// Retained execution source for the frame.
    pub source: FrameSource,
    /// Captured frame state.
    state: FrameStateId,
    /// Canonical live frame bytes.
    bytes: Vec<u8>,
}

/// Retained execution source for one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameSource {
    /// Execution retained at one handshake or debugger stop.
    Retained {
        /// Retained runnable identifier.
        runnable_id: RunnableId,
    },
    /// Fiber parked or woken in the scheduler.
    Fiber {
        /// Scheduler fiber identity.
        fiber: program::Fiber,
    },
}

impl WorkerImage {
    /// Return all frames retained by one worker image.
    fn frames(
        &self,
        program: &Arc<program::Program>,
        memory: &MemoryImage,
    ) -> RuntimeResult<Vec<FrameView>> {
        // fibers only execute through bytecode; native tiers deopt before inspection
        if program.bytecode().is_none() {
            return Ok(Vec::new());
        }
        let machine = vm::Machine::new(program.clone(), vm::MachineLimits::default())
            .map_err(Box::<RuntimeError>::from)?;
        let mut frames = Vec::new();

        // expose the execution retained at one handshake or debugger stop
        if let Some(image) = self.machine.stopped() {
            let Some(retained) = self.retained.as_ref() else {
                return Err(RuntimeError::inconsistent_image(format!(
                    "worker {} retains frames without a stopped runnable",
                    self.worker_id.0
                ))
                .boxed());
            };
            let source = FrameSource::Retained {
                runnable_id: retained.id,
            };
            self.append_fiber_frames(&machine, memory, source, image, &mut frames)?;
        }

        // expose every parked or woken scheduler fiber
        if let Some(snapshot) = self.event_loop.active() {
            for (fiber, image) in snapshot.fibers().executions() {
                let source = FrameSource::Fiber { fiber };
                self.append_fiber_frames(&machine, memory, source, image, &mut frames)?;
            }
        }

        Ok(frames)
    }

    /// Append one fiber image's frames through their canonical projections.
    fn append_fiber_frames(
        &self,
        machine: &vm::Machine,
        memory: &MemoryImage,
        source: FrameSource,
        image: &vm::FiberImage,
        frames: &mut Vec<FrameView>,
    ) -> RuntimeResult<()> {
        let mut read = |offset: usize, byte_len: usize| memory.read_bytes(offset, byte_len).ok();
        let projected = machine
            .project_frames(image, &mut read)
            .map_err(Box::<RuntimeError>::from)?;

        for (state, bytes) in projected {
            frames.push(FrameView::new(self, source, state, bytes));
        }

        Ok(())
    }
}

impl FrameView {
    /// Create one retained frame view.
    fn new(worker: &WorkerImage, source: FrameSource, state: FrameStateId, bytes: Vec<u8>) -> Self {
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

impl fmt::Debug for FrameView {
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
    pub fn frames(&self) -> RuntimeResult<Vec<FrameView>> {
        let mut frames = Vec::new();

        // preserve worker order while resolving every referenced runtime loudly
        for worker in self.image.workers.values() {
            let runtime = self.runtime(worker.runtime_id)?;
            frames.extend(worker.frames(&runtime.program, self.image.memory())?);
        }

        Ok(frames)
    }

    /// Return all frames retained by one worker at this moment.
    pub fn worker_frames(&self, worker_id: WorkerId) -> RuntimeResult<Vec<FrameView>> {
        let worker = self.worker(worker_id)?;
        let runtime = self.runtime(worker.runtime_id)?;

        worker.frames(&runtime.program, self.image.memory())
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
