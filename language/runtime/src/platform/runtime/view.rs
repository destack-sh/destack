use std::collections::BTreeMap;
use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::runtime::{
    EngineDescriptor, EventLoopDescriptor, HeapDescriptor, ImageDescriptor,
    ResourceDescriptorValue, RevisionDescriptor, RuntimeDescriptorValue, TopologyEdgeValue,
    TopologyEntityValue, TraceDescriptor, TraceSequence, WorkerDescriptorValue,
    WorldDescriptorValue, WorldHandle,
};
use crate::runtime;
use crate::runtime::control::inspect::{
    EdgeListFilter, EntityListFilter, ResourceListFilter, RuntimeListFilter, WorkerListFilter,
    edge_in_image, entity_in_image, list_edges, list_entities, list_resources, list_runtimes,
    list_workers, resource_in_image, runtime_in_image, worker_in_image,
};
use crate::runtime::world::{RevisionState, WorldImage};

use crate::runtime::control::Control;

use super::{RuntimeDescriptorCodec, RuntimeHandleCodec};

/// One pinned world view exposed through the low-level runtime binding surface.
#[derive(Debug, Clone)]
pub(crate) struct PinnedWorldView {
    /// The owning world handle.
    pub world_handle: WorldHandle,
    /// The stored world labels.
    pub labels: BTreeMap<String, String>,
    /// The pinned revision handle.
    pub revision_handle: runtime::world::Revision,
    /// The pinned revision metadata.
    pub revision: RevisionState,
    /// The pinned world image.
    pub image: Arc<WorldImage>,
}

impl PinnedWorldView {
    /// Resolve one pinned world view through the control table.
    pub(crate) fn from_handle(
        table: &Control,
        handle: super::WorldViewHandle,
    ) -> RuntimeResult<Self> {
        let handle_id = RuntimeHandleCodec::decode_world_view_handle(handle);
        let entry = table.world_view(handle_id)?;

        Ok(RuntimeDescriptorCodec::world_view_entry(entry))
    }

    /// Return one captured worker from this pinned world view.
    pub(crate) fn worker(
        &self,
        worker_id: runtime::WorkerId,
    ) -> RuntimeResult<&runtime::WorkerImage> {
        worker_in_image(&self.image, worker_id)
    }

    /// Return one owned world descriptor from this pinned world view.
    pub(crate) fn world_descriptor(&self) -> RuntimeResult<WorldDescriptorValue> {
        RuntimeDescriptorCodec::world_descriptor_for_revision(
            self.world_handle,
            self.revision_handle,
            self.revision.clone(),
            &self.image,
            self.labels.clone(),
        )
    }

    /// Return one owned revision descriptor from this pinned world view.
    pub(crate) fn revision_descriptor(&self) -> RuntimeResult<RevisionDescriptor> {
        RuntimeDescriptorCodec::revision_descriptor(
            self.revision_handle,
            self.revision.clone(),
            &self.image,
        )
    }

    /// Return one owned image descriptor from this pinned world view.
    pub(crate) fn image_descriptor(&self) -> RuntimeResult<ImageDescriptor> {
        RuntimeDescriptorCodec::image_descriptor(
            self.revision_handle,
            self.revision.image_id,
            &self.image,
        )
    }

    /// Return one owned trace descriptor from this pinned world view.
    pub(crate) fn trace_descriptor(&self) -> RuntimeResult<TraceDescriptor> {
        Ok(TraceDescriptor {
            branch_id: RuntimeHandleCodec::encode_branch_id(self.revision.branch_id)?,
            sequence: TraceSequence(self.revision.sequence.get()),
        })
    }

    /// Return one owned runtime descriptor from this pinned world view.
    pub(crate) fn runtime_descriptor(
        &self,
        runtime_id: runtime::world::RuntimeId,
    ) -> RuntimeResult<RuntimeDescriptorValue> {
        let runtime = runtime_in_image(&self.image, runtime_id)?;

        RuntimeDescriptorCodec::runtime_descriptor_for_image(&self.image, runtime_id, runtime)
    }

    /// Return owned runtime descriptors from this pinned world view.
    pub(crate) fn runtime_descriptors(
        &self,
        filter: &RuntimeListFilter,
        after: Option<runtime::world::RuntimeId>,
        limit: Option<usize>,
    ) -> RuntimeResult<Vec<RuntimeDescriptorValue>> {
        let runtimes = list_runtimes(&self.image, filter, after, limit)?;
        let mut descriptors = Vec::with_capacity(runtimes.len());

        // collect owned runtime descriptors
        for (runtime_id, runtime) in runtimes {
            let descriptor = RuntimeDescriptorCodec::runtime_descriptor_for_image(
                &self.image,
                runtime_id,
                runtime,
            )?;
            descriptors.push(descriptor);
        }

        Ok(descriptors)
    }

    /// Return one owned worker descriptor from this pinned world view.
    pub(crate) fn worker_descriptor(
        &self,
        worker_id: runtime::WorkerId,
    ) -> RuntimeResult<WorkerDescriptorValue> {
        let worker = self.worker(worker_id)?;

        RuntimeDescriptorCodec::worker_descriptor_for_image(&self.image, worker_id, worker)
    }

    /// Return one owned event-loop descriptor from this pinned world view.
    pub(crate) fn event_loop_descriptor(
        &self,
        worker_id: runtime::WorkerId,
    ) -> RuntimeResult<EventLoopDescriptor> {
        let worker = self.worker(worker_id)?;

        RuntimeDescriptorCodec::event_loop_descriptor(&worker.event_loop)
    }

    /// Return one owned heap descriptor from this pinned world view.
    pub(crate) fn heap_descriptor(
        &self,
        worker_id: runtime::WorkerId,
    ) -> RuntimeResult<HeapDescriptor> {
        let worker = self.worker(worker_id)?;

        RuntimeDescriptorCodec::heap_descriptor(worker)
    }

    /// Return one owned engine descriptor from this pinned world view.
    pub(crate) fn engine_descriptor(
        &self,
        worker_id: runtime::WorkerId,
    ) -> RuntimeResult<EngineDescriptor> {
        let worker = self.worker(worker_id)?;

        RuntimeDescriptorCodec::engine_descriptor(worker)
    }

    /// Return owned worker descriptors from this pinned world view.
    pub(crate) fn worker_descriptors(
        &self,
        filter: &WorkerListFilter,
        after: Option<runtime::WorkerId>,
        limit: Option<usize>,
    ) -> RuntimeResult<Vec<WorkerDescriptorValue>> {
        let workers = list_workers(&self.image, filter, after, limit)?;
        let mut descriptors = Vec::with_capacity(workers.len());

        // collect owned worker descriptors
        for (worker_id, worker) in workers {
            let descriptor = RuntimeDescriptorCodec::worker_descriptor_for_image(
                &self.image,
                worker_id,
                worker,
            )?;
            descriptors.push(descriptor);
        }

        Ok(descriptors)
    }

    /// Return one owned resource descriptor from this pinned world view.
    pub(crate) fn resource_descriptor(
        &self,
        resource_id: runtime::world::WorldResourceId,
    ) -> RuntimeResult<ResourceDescriptorValue> {
        let resource = resource_in_image(&self.image, resource_id)?;

        Ok(RuntimeDescriptorCodec::resource_descriptor(resource))
    }

    /// Return owned resource descriptors from this pinned world view.
    pub(crate) fn resource_descriptors(
        &self,
        filter: &ResourceListFilter,
        after: Option<runtime::world::WorldResourceId>,
        limit: Option<usize>,
    ) -> RuntimeResult<Vec<ResourceDescriptorValue>> {
        let resources = list_resources(&self.image, filter, after, limit);
        let mut descriptors = Vec::with_capacity(resources.len());

        // collect owned resource descriptors
        for resource in resources {
            descriptors.push(RuntimeDescriptorCodec::resource_descriptor(resource));
        }

        Ok(descriptors)
    }

    /// Return one owned topology entity descriptor from this pinned world view.
    pub(crate) fn entity_descriptor(&self, entity_id: &str) -> RuntimeResult<TopologyEntityValue> {
        let entity = entity_in_image(&self.image, entity_id)?;

        Ok(RuntimeDescriptorCodec::entity_descriptor(entity))
    }

    /// Return owned topology entity descriptors from this pinned world view.
    pub(crate) fn entity_descriptors(
        &self,
        filter: &EntityListFilter,
        after: Option<&str>,
        limit: Option<usize>,
    ) -> RuntimeResult<Vec<TopologyEntityValue>> {
        let entities = list_entities(&self.image, filter, after, limit);
        let mut descriptors = Vec::with_capacity(entities.len());

        // collect owned topology entities
        for entity in entities {
            descriptors.push(RuntimeDescriptorCodec::entity_descriptor(entity));
        }

        Ok(descriptors)
    }

    /// Return one owned topology edge descriptor from this pinned world view.
    pub(crate) fn edge_descriptor(&self, edge_id: &str) -> RuntimeResult<TopologyEdgeValue> {
        let edge = edge_in_image(&self.image, edge_id)?;

        Ok(RuntimeDescriptorCodec::edge_descriptor(edge))
    }

    /// Return owned topology edge descriptors from this pinned world view.
    pub(crate) fn edge_descriptors(
        &self,
        filter: &EdgeListFilter,
        after: Option<&str>,
        limit: Option<usize>,
    ) -> RuntimeResult<Vec<TopologyEdgeValue>> {
        let edges = list_edges(&self.image, filter, after, limit);
        let mut descriptors = Vec::with_capacity(edges.len());

        // collect owned topology edges
        for edge in edges {
            descriptors.push(RuntimeDescriptorCodec::edge_descriptor(edge));
        }

        Ok(descriptors)
    }
}
