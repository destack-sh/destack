use std::collections::BTreeMap;
use std::sync::Arc;

use destack_core::CaptureMode;
use destack_program as program;

use crate::diagnostic::RuntimeResult;
use crate::host::ResourceId;
use crate::runtime::scheduler::{RunnableId, WakeKey};
use crate::runtime::{RuntimeImage, WorkerId, WorkerImage};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameView {
    /// Runtime that owns the frame.
    pub runtime_id: RuntimeId,
    /// Worker that owns the frame.
    pub worker_id: WorkerId,
    /// Retained execution source for the frame.
    pub source: FrameSource,
    /// Frame position inside its retained frame chain.
    pub frame_index: usize,
    /// Captured frame state.
    pub frame_state: program::FrameStateId,
    /// Caller return frame state.
    pub return_state: Option<program::FrameStateId>,
    /// Captured frame byte width.
    pub byte_len: usize,
}

/// Retained execution source for one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameSource {
    /// Active worker machine stack.
    Active,
    /// Runnable stopped at a breakpoint or other stop point.
    Stopped {
        /// Stopped runnable identifier.
        runnable_id: RunnableId,
    },
    /// Queued task continuation.
    Task {
        /// Task runnable identifier.
        runnable_id: RunnableId,
    },
    /// Queued microtask continuation.
    Microtask {
        /// Microtask runnable identifier.
        runnable_id: RunnableId,
    },
    /// Suspended continuation waiting on one wake source.
    Waiter {
        /// Wake source for the suspended continuation.
        wake: WakeKey,
    },
}

impl FrameView {
    /// Create one active frame view.
    fn active(worker: &WorkerImage, frame_index: usize, frame: &program::FrameImage) -> Self {
        Self {
            runtime_id: worker.runtime_id,
            worker_id: worker.worker_id,
            source: FrameSource::Active,
            frame_index,
            frame_state: frame.frame_state,
            return_state: frame.return_state,
            byte_len: frame.byte_len,
        }
    }

    /// Create one continuation frame view.
    fn continuation(
        worker: &WorkerImage,
        source: FrameSource,
        frame_index: usize,
        frame: &program::ContinuationFrame,
    ) -> Self {
        Self {
            runtime_id: worker.runtime_id,
            worker_id: worker.worker_id,
            source,
            frame_index,
            frame_state: frame.frame_state,
            return_state: frame.return_state,
            byte_len: frame.byte_len,
        }
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
    pub fn runtime_ids(&self) -> Vec<RuntimeId> {
        self.image.runtimes.keys().copied().collect()
    }

    /// Return all worker ids visible at this moment.
    pub fn worker_ids(&self) -> Vec<WorkerId> {
        self.image.workers.keys().copied().collect()
    }

    /// Return all frames visible at this moment.
    pub fn frames(&self) -> Vec<FrameView> {
        let mut frames = Vec::new();

        for worker in self.image.workers.values() {
            Self::append_worker_frames(&mut frames, worker);
        }

        frames
    }

    /// Return all frames retained by one worker at this moment.
    pub fn worker_frames(&self, worker_id: WorkerId) -> RuntimeResult<Vec<FrameView>> {
        let worker = self.worker(worker_id)?;
        let mut frames = Vec::new();

        Self::append_worker_frames(&mut frames, worker);

        Ok(frames)
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

    /// Append every frame retained by one worker image.
    fn append_worker_frames(frames: &mut Vec<FrameView>, worker: &WorkerImage) {
        // active machine frames
        for (frame_index, frame) in worker.machine_image.frames().iter().enumerate() {
            frames.push(FrameView::active(worker, frame_index, frame));
        }

        // stopped runnable frames
        if let Some(stop) = &worker.stop {
            let source = FrameSource::Stopped {
                runnable_id: stop.id,
            };
            Self::append_continuation_frames(frames, worker, source, &stop.continuation.program);
        }

        // queued scheduler frames
        let Some(snapshot) = worker.event_loop.active() else {
            return;
        };
        for task in &snapshot.tasks {
            let source = FrameSource::Task {
                runnable_id: task.id,
            };
            Self::append_continuation_frames(frames, worker, source, &task.continuation.program);
        }
        for microtask in &snapshot.microtasks {
            let source = FrameSource::Microtask {
                runnable_id: microtask.id,
            };
            Self::append_continuation_frames(
                frames,
                worker,
                source,
                &microtask.continuation.program,
            );
        }
        for waiter in &snapshot.waiters {
            let source = FrameSource::Waiter { wake: waiter.key };
            Self::append_continuation_frames(
                frames,
                worker,
                source,
                &waiter.waiter.continuation.program,
            );
        }
    }

    /// Append every frame retained by one continuation.
    fn append_continuation_frames(
        frames: &mut Vec<FrameView>,
        worker: &WorkerImage,
        source: FrameSource,
        continuation: &program::Continuation,
    ) {
        for (frame_index, frame) in continuation.frames.iter().enumerate() {
            frames.push(FrameView::continuation(worker, source, frame_index, frame));
        }
    }
}
