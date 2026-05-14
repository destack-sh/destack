use std::collections::BTreeMap;
use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::host::ResourceId;
use crate::runtime::{RuntimeImage, WorkerId, WorkerImage};
use crate::world::policy::Policy;
use crate::world::topology::{
    Edge, EdgeDefinition, EdgeKind, Entity, EntityDefinition, EntityKind, RuntimeId,
};
use crate::world::{Resource, WorldImage};

use super::Moment;

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

    /// Return all retained runtime images visible at this moment.
    pub fn runtimes(&self) -> &BTreeMap<RuntimeId, Arc<RuntimeImage>> {
        self.image.runtimes()
    }

    /// Return all retained worker images visible at this moment.
    pub fn workers(&self) -> &BTreeMap<WorkerId, Arc<WorkerImage>> {
        self.image.workers()
    }

    /// Return all world resources visible at this moment.
    pub fn resources(&self) -> &BTreeMap<ResourceId, Resource> {
        self.image.resources()
    }

    /// Return all runtime ids visible at this moment.
    pub fn runtime_ids(&self) -> Vec<RuntimeId> {
        self.image.runtimes.keys().copied().collect()
    }

    /// Return all worker ids visible at this moment.
    pub fn worker_ids(&self) -> Vec<WorkerId> {
        self.image.workers.keys().copied().collect()
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
    pub fn runtime_labels(
        &self,
        runtime_id: RuntimeId,
    ) -> RuntimeResult<&BTreeMap<String, String>> {
        self.image.runtime_labels(runtime_id)
    }

    /// Return one worker image by id.
    pub fn worker(&self, worker_id: WorkerId) -> RuntimeResult<&WorkerImage> {
        self.image.worker(worker_id)
    }

    /// Return labels for one worker visible at this moment.
    pub fn worker_labels(&self, worker_id: WorkerId) -> RuntimeResult<&BTreeMap<String, String>> {
        self.image.worker_labels(worker_id)
    }

    /// Return one resource by id when present.
    pub fn resource(&self, resource_id: ResourceId) -> Option<&Resource> {
        self.image.resource(resource_id)
    }

    /// Return one topology entity by id when present.
    pub fn entity(&self, entity_id: &str) -> Option<&Entity> {
        self.image.entity(entity_id)
    }

    /// Return all topology entity kinds visible at this moment.
    pub fn entity_kinds(&self) -> &BTreeMap<EntityKind, EntityDefinition> {
        self.image.topology.entity_kinds()
    }

    /// Return one topology entity kind by id when present.
    pub fn entity_kind(&self, kind_id: &str) -> Option<&EntityDefinition> {
        self.image.topology.entity_kinds().get(kind_id)
    }

    /// Return one topology edge by id when present.
    pub fn edge(&self, edge_id: &str) -> Option<&Edge> {
        self.image.edge(edge_id)
    }

    /// Return all topology edge kinds visible at this moment.
    pub fn edge_kinds(&self) -> &BTreeMap<EdgeKind, EdgeDefinition> {
        self.image.topology.edge_kinds()
    }

    /// Return one topology edge kind by id when present.
    pub fn edge_kind(&self, kind_id: &str) -> Option<&EdgeDefinition> {
        self.image.topology.edge_kinds().get(kind_id)
    }
}
