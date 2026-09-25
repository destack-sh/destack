use std::collections::BTreeMap;
use std::sync::Arc;

use tspp_core::{CaptureMode, fnv1a_128};
use tspp_memory::MemoryImage;
use tspp_program as program;
use tspp_serde as serde;

use ::serde::{Deserialize, Serialize};

use crate::debugger::{Debugger, Frame};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::ResourceId;
use crate::runtime::{Runtime, RuntimeId, RuntimeImage};
use crate::worker::{WorkerId, WorkerImage};
use crate::world::policy::Policy;
use crate::world::random::RandomImage;
use crate::world::time::ClockImage;
use crate::world::topology::{
    Edge, EdgeDefinition, EdgeId, EdgeKind, Entity, EntityDefinition, EntityId, EntityKind,
    LabelSet, Topology,
};

use super::{MomentSequence, RestoreContext, World};

/// World image payload for one materialized world restore point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldImage {
    /// Captured copy-on-write world memory.
    pub(crate) memory: MemoryImage,
    /// Captured branch-local moment sequence.
    pub(crate) moment: MomentSequence,
    /// The next runtime id to allocate after restore.
    pub(crate) next_runtime_id: u64,
    /// The next worker id to allocate after restore.
    pub(crate) next_worker_id: u64,
    /// The next runtime slot to schedule first.
    pub(crate) next_runtime_cursor: usize,
    /// Captured dynamic policy state.
    pub(crate) policy: Policy,
    /// Captured debugger configuration.
    pub(crate) debugger: Debugger,
    /// Captured topology metadata graph.
    pub(crate) topology: Topology,
    /// Captured world clock state.
    pub(crate) clock: ClockImage,
    /// Captured world random state.
    pub(crate) random: RandomImage,
    /// Captured runtime metadata keyed by runtime id.
    pub(crate) runtimes: BTreeMap<RuntimeId, Arc<RuntimeImage>>,
    /// Captured worker metadata keyed by worker id.
    pub(crate) workers: BTreeMap<WorkerId, Arc<WorkerImage>>,
}

impl WorldImage {
    /// Return the captured world memory map.
    pub fn memory(&self) -> &MemoryImage {
        &self.memory
    }

    /// Return the captured world policy specification.
    pub fn policy(&self) -> &Policy {
        &self.policy
    }

    /// Return the captured world debugger configuration.
    pub fn debugger(&self) -> &Debugger {
        &self.debugger
    }

    /// Return the captured runtimes keyed by runtime id.
    pub fn runtimes(&self) -> &BTreeMap<RuntimeId, Arc<RuntimeImage>> {
        &self.runtimes
    }

    /// Return the captured workers keyed by worker id.
    pub fn workers(&self) -> &BTreeMap<WorkerId, Arc<WorkerImage>> {
        &self.workers
    }

    /// Return all captured Runtime identifiers.
    pub fn runtime_ids(&self) -> impl Iterator<Item = RuntimeId> + '_ {
        self.runtimes.keys().copied()
    }

    /// Return all captured Worker identifiers.
    pub fn worker_ids(&self) -> impl Iterator<Item = WorkerId> + '_ {
        self.workers.keys().copied()
    }

    /// Return the number of captured runtimes.
    pub fn runtime_count(&self) -> usize {
        self.runtimes.len()
    }

    /// Return the number of captured workers.
    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    /// Return the number of captured logical resources.
    pub fn resource_count(&self) -> usize {
        self.topology.resource_count()
    }

    /// Return the number of captured topology entities.
    pub fn entity_count(&self) -> usize {
        self.topology.entities().len()
    }

    /// Return the number of captured topology edges.
    pub fn edge_count(&self) -> usize {
        self.topology.edges().len()
    }

    /// Report whether one runtime exists in this image.
    pub fn has_runtime(&self, runtime_id: RuntimeId) -> bool {
        self.runtimes.contains_key(&runtime_id)
    }

    /// Report whether one worker exists in this image.
    pub fn has_worker(&self, worker_id: WorkerId) -> bool {
        self.workers.contains_key(&worker_id)
    }

    /// Report whether one resource exists in this image.
    pub fn has_resource(&self, resource_id: ResourceId) -> bool {
        self.topology.has_resource(resource_id)
    }

    /// Report whether one topology entity exists in this image.
    pub fn has_entity(&self, entity_id: &str) -> bool {
        self.topology.entities().contains_key(entity_id)
    }

    /// Report whether one topology edge exists in this image.
    pub fn has_edge(&self, edge_id: &str) -> bool {
        self.topology.edges().contains_key(edge_id)
    }

    /// Return one runtime image by id.
    pub fn runtime(&self, runtime_id: RuntimeId) -> RuntimeResult<&RuntimeImage> {
        self.runtimes
            .get(&runtime_id)
            .map(Arc::as_ref)
            .ok_or_else(|| RuntimeError::runtime_not_found(runtime_id.0).boxed())
    }

    /// Return the Program instantiated by one captured Runtime.
    pub fn program(&self, runtime_id: RuntimeId) -> RuntimeResult<&program::Program> {
        let runtime = self.runtime(runtime_id)?;

        Ok(runtime.program.as_ref())
    }

    /// Return labels for one runtime image.
    pub fn runtime_labels(&self, runtime_id: RuntimeId) -> RuntimeResult<&LabelSet> {
        let entity_id = runtime_id.entity_id();
        let entity = self
            .topology
            .entities()
            .get(entity_id.as_str())
            .ok_or(RuntimeError::runtime_not_found(runtime_id.0))?;

        Ok(&entity.labels)
    }

    /// Return the topology name for one runtime image.
    pub fn runtime_name(&self, runtime_id: RuntimeId) -> RuntimeResult<&str> {
        let entity = self
            .topology
            .entities()
            .get(&runtime_id.entity_id())
            .ok_or(RuntimeError::runtime_not_found(runtime_id.0))?;

        Ok(entity.name.as_str())
    }

    /// Return one worker image by id.
    pub fn worker(&self, worker_id: WorkerId) -> RuntimeResult<&WorkerImage> {
        self.workers
            .get(&worker_id)
            .map(Arc::as_ref)
            .ok_or_else(|| RuntimeError::worker_not_found(worker_id.0).boxed())
    }

    /// Return all captured Frames.
    pub fn frames(&self) -> RuntimeResult<Vec<Frame>> {
        let mut frames = Vec::new();

        // preserve Worker order while resolving every referenced Runtime loudly
        for worker in self.workers.values() {
            let runtime = self.runtime(worker.runtime_id)?;
            frames.extend(worker.frames(runtime, self.memory())?);
        }

        Ok(frames)
    }

    /// Return all captured Frames for one Worker.
    pub fn worker_frames(&self, worker_id: WorkerId) -> RuntimeResult<Vec<Frame>> {
        let worker = self.worker(worker_id)?;
        let runtime = self.runtime(worker.runtime_id)?;

        worker.frames(runtime, self.memory())
    }

    /// Return labels for one worker image.
    pub fn worker_labels(&self, worker_id: WorkerId) -> RuntimeResult<&LabelSet> {
        let entity_id = worker_id.entity_id();
        let entity = self
            .topology
            .entities()
            .get(entity_id.as_str())
            .ok_or(RuntimeError::worker_not_found(worker_id.0))?;

        Ok(&entity.labels)
    }

    /// Return the topology name for one worker image.
    pub fn worker_name(&self, worker_id: WorkerId) -> RuntimeResult<&str> {
        let entity = self
            .topology
            .entities()
            .get(&worker_id.entity_id())
            .ok_or(RuntimeError::worker_not_found(worker_id.0))?;

        Ok(entity.name.as_str())
    }

    /// Return whether one captured runtime owns one captured worker.
    pub fn runtime_owns_worker(&self, runtime_id: RuntimeId, worker_id: WorkerId) -> bool {
        self.topology.runtime_owns_worker(runtime_id, worker_id)
    }

    /// Return the owning runtime for one captured worker.
    pub fn worker_runtime_id(&self, worker_id: WorkerId) -> RuntimeResult<RuntimeId> {
        let worker = self.worker(worker_id)?;

        Ok(worker.runtime_id)
    }

    /// Return one topology entity by id.
    pub fn entity(&self, entity_id: &str) -> Option<&Entity> {
        self.topology.entities().get(entity_id)
    }

    /// Return all captured topology entity kinds.
    pub fn entity_kinds(&self) -> BTreeMap<EntityKind, EntityDefinition> {
        self.topology.entity_kinds()
    }

    /// Return all captured topology entities.
    pub fn entities(&self) -> BTreeMap<EntityId, Entity> {
        self.topology.entities().clone()
    }

    /// Return one captured topology entity kind when present.
    pub fn entity_kind(&self, kind_id: &str) -> Option<&EntityDefinition> {
        self.topology.entity_kind(kind_id)
    }

    /// Return one topology edge by id.
    pub fn edge(&self, edge_id: &str) -> Option<&Edge> {
        self.topology.edges().get(edge_id)
    }

    /// Return all captured topology edge kinds.
    pub fn edge_kinds(&self) -> BTreeMap<EdgeKind, EdgeDefinition> {
        self.topology.edge_kinds()
    }

    /// Return all captured topology edges.
    pub fn edges(&self) -> BTreeMap<EdgeId, Edge> {
        self.topology.edges().clone()
    }

    /// Return one captured topology edge kind when present.
    pub fn edge_kind(&self, kind_id: &str) -> Option<&EdgeDefinition> {
        self.topology.edge_kind(kind_id)
    }
}

impl World {
    /// Capture the current World as one materialized image.
    pub fn image(&mut self) -> RuntimeResult<WorldImage> {
        self.capture_image(CaptureMode::Suspend)
    }

    /// Capture one materialized world image while the world is under exclusive access.
    pub(crate) fn capture_image(&mut self, mode: CaptureMode) -> RuntimeResult<WorldImage> {
        self.quiesce_shared_gc();

        let image = (|| {
            let memory = self.memory.capture().map_err(Box::<RuntimeError>::from)?;
            let mut runtime_images = BTreeMap::new();
            let mut worker_images = BTreeMap::new();

            for runtime in self.runtimes.values_mut() {
                let (runtime_image, runtime_workers) = runtime.capture_image(mode)?;
                runtime_images.insert(runtime.runtime_id(), runtime_image);
                worker_images.extend(runtime_workers);
            }

            Ok(WorldImage {
                memory,
                moment: self.state.moment,
                next_runtime_id: self.state.next_runtime_id,
                next_worker_id: self.state.next_worker_id,
                next_runtime_cursor: self.next_runtime_cursor,
                policy: self.state.policy.clone(),
                debugger: self.state.debugger.clone(),
                topology: self.state.topology.clone(),
                clock: self.state.clock.image(),
                random: self.state.random.image(),
                runtimes: runtime_images,
                workers: worker_images,
            })
        })();

        self.resume_shared_gc();

        image
    }

    /// Restore one materialized image into this world while the world is quiesced.
    pub(crate) fn restore_image(
        &mut self,
        image: &WorldImage,
        restore: RestoreContext<'_>,
    ) -> RuntimeResult<()> {
        self.quiesce_shared_gc();

        let result = (|| {
            // isolate the retained image pages for live mutation
            let memory = Arc::new(image.memory.restore().map_err(Box::<RuntimeError>::from)?);

            self.state.next_runtime_id = image.next_runtime_id;
            self.state.next_worker_id = image.next_worker_id;
            self.next_runtime_cursor = image.next_runtime_cursor;
            self.state.moment = image.moment;
            self.state.policy = image.policy.clone();
            self.state.debugger = image.debugger.clone();
            self.state.topology = image.topology.clone();

            self.state.clock.restore(&image.clock);
            self.state.random.restore(&image.random)?;
            self.state.observations.reset();

            let mut restored_runtimes = BTreeMap::new();
            for (runtime_id, runtime_image) in &image.runtimes {
                let runtime_worker_images = image
                    .workers
                    .iter()
                    .filter(|(worker_id, _)| image.runtime_owns_worker(*runtime_id, **worker_id))
                    .map(|(worker_id, worker_image)| (*worker_id, worker_image.clone()))
                    .collect::<BTreeMap<_, _>>();
                let collector = self.collector.clone();

                let runtime = Runtime::from_image(
                    &mut self.state,
                    memory.clone(),
                    collector,
                    *runtime_id,
                    runtime_image.as_ref(),
                    &runtime_worker_images,
                    restore,
                )?;
                restored_runtimes.insert(*runtime_id, runtime);
            }

            self.memory = memory;
            self.runtimes = restored_runtimes;

            Ok(())
        })();

        self.resume_shared_gc();

        result
    }

    /// Return the encoded size and hash for one image.
    pub(crate) fn image_size_and_hash(image: &WorldImage) -> RuntimeResult<(u64, u128)> {
        let bytes = serde::to_vec(image).map_err(|_| {
            RuntimeError::inconsistent_image("failed to encode world image".to_string()).boxed()
        })?;
        let size_bytes = bytes.len() as u64;
        let hash = fnv1a_128(&bytes);

        Ok((size_bytes, hash))
    }
}
