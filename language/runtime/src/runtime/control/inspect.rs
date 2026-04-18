use std::collections::BTreeMap;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::world::{
    BranchId, Revision, RuntimeId, WorldEdge, WorldEntity, WorldImage, WorldResource,
    WorldResourceId,
};
use crate::runtime::{WorkerId, WorkerImage, RuntimeImage};

/// Runtime list filter decoded from one low-level ABI surface.
#[derive(Debug, Clone, Default)]
pub(crate) struct RuntimeListFilter {
    /// Optional exact runtime name match.
    pub name: Option<String>,
    /// Optional label selectors.
    pub labels: Vec<(String, Option<String>)>,
}

/// Worker list filter decoded from one low-level ABI surface.
#[derive(Debug, Clone, Default)]
pub(crate) struct WorkerListFilter {
    /// Optional owning runtime id.
    pub runtime_id: Option<RuntimeId>,
    /// Optional exact worker name match.
    pub name: Option<String>,
    /// Optional pending-work predicate.
    pub has_pending_work: Option<bool>,
    /// Optional label selectors.
    pub labels: Vec<(String, Option<String>)>,
}

/// Branch list filter decoded from one low-level ABI surface.
#[derive(Debug, Clone, Default)]
pub(crate) struct BranchListFilter {
    /// Optional exact branch name match.
    pub name: Option<String>,
    /// Optional label selectors.
    pub labels: Vec<(String, Option<String>)>,
}

/// Revision list filter decoded from one low-level ABI surface.
#[derive(Debug, Clone, Default)]
pub(crate) struct RevisionListFilter {
    /// Optional owning branch id.
    pub branch_id: Option<BranchId>,
}

/// Checkpoint list filter decoded from one low-level ABI surface.
#[derive(Debug, Clone, Default)]
pub(crate) struct CheckpointListFilter {
    /// Optional revision id filter.
    pub revision: Option<Revision>,
    /// Optional exact checkpoint name.
    pub name: Option<String>,
    /// Optional label selectors.
    pub labels: Vec<(String, Option<String>)>,
}

/// WorldImage list filter decoded from one low-level ABI surface.
#[derive(Debug, Clone, Default)]
pub(crate) struct ImageListFilter {
    /// Optional owning revision id.
    pub revision: Option<Revision>,
}

/// Resource list filter decoded from one low-level ABI surface.
#[derive(Debug, Clone, Default)]
pub(crate) struct ResourceListFilter {
    /// Optional owning runtime id.
    pub runtime_id: Option<RuntimeId>,
    /// Optional owning worker id.
    pub worker_id: Option<WorkerId>,
    /// Optional exact resource kind.
    pub kind: Option<String>,
    /// Optional exact resource label.
    pub label: Option<String>,
}

/// Entity list filter decoded from one low-level ABI surface.
#[derive(Debug, Clone, Default)]
pub(crate) struct EntityListFilter {
    /// Optional exact entity kind.
    pub kind: Option<String>,
    /// Optional label selectors.
    pub labels: Vec<(String, Option<String>)>,
}

/// Edge list filter decoded from one low-level ABI surface.
#[derive(Debug, Clone, Default)]
pub(crate) struct EdgeListFilter {
    /// Optional exact edge kind.
    pub kind: Option<String>,
    /// Optional exact source entity id.
    pub from: Option<String>,
    /// Optional exact target entity id.
    pub to: Option<String>,
    /// Optional label selectors.
    pub labels: Vec<(String, Option<String>)>,
}

/// Return whether one label map matches all selectors.
pub(crate) fn labels_match_selectors(
    labels: &BTreeMap<String, String>,
    selectors: &[(String, Option<String>)],
) -> bool {
    selectors.iter().all(|(key, value)| match labels.get(key) {
        Some(label) => value.as_ref().is_none_or(|value| label == value),
        None => false,
    })
}

/// List runtimes in stable id order.
pub(crate) fn list_runtimes<'a>(
    image: &'a WorldImage,
    filter: &RuntimeListFilter,
    after: Option<RuntimeId>,
    limit: Option<usize>,
) -> RuntimeResult<Vec<(RuntimeId, &'a RuntimeImage)>> {
    let after = after.map(|id| id.0).unwrap_or(0);
    let mut runtimes = Vec::new();

    // stable runtime scan
    for (runtime_id, runtime) in &image.runtimes {
        if runtime_id.0 <= after {
            continue;
        }

        if let Some(name) = filter.name.as_deref()
            && image.runtime_name(*runtime_id)? != name
        {
            continue;
        }

        let labels = image.runtime_labels(*runtime_id)?;
        if !labels_match_selectors(labels, &filter.labels) {
            continue;
        }

        runtimes.push((*runtime_id, runtime.as_ref()));

        if let Some(limit) = limit
            && runtimes.len() >= limit
        {
            break;
        }
    }

    Ok(runtimes)
}

/// Resolve one runtime from one pinned image.
pub(crate) fn runtime_in_image(
    image: &WorldImage,
    runtime_id: RuntimeId,
) -> RuntimeResult<&RuntimeImage> {
    let runtime = image.runtimes.get(&runtime_id).ok_or_else(|| {
        RuntimeError::RuntimeNotFound {
            runtime_id: runtime_id.0,
        }
        .boxed()
    })?;

    Ok(runtime.as_ref())
}

/// List workers in stable id order.
pub(crate) fn list_workers<'a>(
    image: &'a WorldImage,
    filter: &WorkerListFilter,
    after: Option<WorkerId>,
    limit: Option<usize>,
) -> RuntimeResult<Vec<(WorkerId, &'a WorkerImage)>> {
    let after = after.map(|id| id.0).unwrap_or(0);
    let mut workers = Vec::new();

    // stable worker scan
    for (worker_id, worker) in &image.workers {
        if worker_id.0 <= after {
            continue;
        }

        if let Some(runtime_id) = filter.runtime_id
            && !image.runtime_owns_worker(runtime_id, *worker_id)
        {
            continue;
        }

        if let Some(name) = filter.name.as_deref()
            && image.worker_name(*worker_id)? != name
        {
            continue;
        }

        let has_pending_work = worker.has_pending_work();
        if let Some(expected) = filter.has_pending_work
            && has_pending_work != expected
        {
            continue;
        }

        let labels = image.worker_labels(*worker_id)?;
        if !labels_match_selectors(labels, &filter.labels) {
            continue;
        }

        workers.push((*worker_id, worker.as_ref()));

        if let Some(limit) = limit
            && workers.len() >= limit
        {
            break;
        }
    }

    Ok(workers)
}

/// Resolve one worker from one pinned image.
pub(crate) fn worker_in_image(image: &WorldImage, worker_id: WorkerId) -> RuntimeResult<&WorkerImage> {
    let worker = image.workers.get(&worker_id).ok_or_else(|| {
        RuntimeError::WorkerNotFound {
            worker_id: worker_id.0,
        }
        .boxed()
    })?;

    Ok(worker.as_ref())
}

/// Return whether one logical resource matches the structured filter.
pub(crate) fn resource_matches_filter(
    image: &WorldImage,
    resource: &WorldResource,
    filter: &ResourceListFilter,
) -> bool {
    if let Some(runtime_id) = filter.runtime_id {
        if !image.has_worker(resource.id.worker_id) {
            return false;
        }

        if !image.runtime_owns_worker(runtime_id, resource.id.worker_id) {
            return false;
        }
    }

    if let Some(worker_id) = filter.worker_id
        && resource.id.worker_id != worker_id
    {
        return false;
    }

    if let Some(kind) = filter.kind.as_deref()
        && resource.kind.as_str() != kind
    {
        return false;
    }

    if let Some(label) = filter.label.as_deref()
        && resource.label.as_deref() != Some(label)
    {
        return false;
    }

    true
}

/// List logical resources in stable id order.
pub(crate) fn list_resources<'a>(
    image: &'a WorldImage,
    filter: &ResourceListFilter,
    after: Option<WorldResourceId>,
    limit: Option<usize>,
) -> Vec<&'a WorldResource> {
    let mut resources = Vec::new();

    // stable resource scan
    for resource in image.resources.values() {
        if let Some(after) = after
            && resource.id <= after
        {
            continue;
        }

        if !resource_matches_filter(image, resource, filter) {
            continue;
        }

        resources.push(resource);

        if let Some(limit) = limit
            && resources.len() >= limit
        {
            break;
        }
    }

    resources
}

/// Resolve one logical resource from one pinned image.
pub(crate) fn resource_in_image(
    image: &WorldImage,
    resource_id: WorldResourceId,
) -> RuntimeResult<&WorldResource> {
    image.resource(resource_id).ok_or_else(|| {
        RuntimeError::ResourceNotFound {
            resource_id: resource_id.resource_id.0,
            resource_kind: None,
        }
        .boxed()
    })
}

/// List topology entities in stable id order.
pub(crate) fn list_entities<'a>(
    image: &'a WorldImage,
    filter: &EntityListFilter,
    after: Option<&str>,
    limit: Option<usize>,
) -> Vec<&'a WorldEntity> {
    let mut entities = Vec::new();

    // stable topology entity scan
    for entity in image.topology.entities().values() {
        if let Some(after) = after
            && entity.id.as_str() <= after
        {
            continue;
        }

        if let Some(kind) = filter.kind.as_deref()
            && entity.kind.as_str() != kind
        {
            continue;
        }

        if !labels_match_selectors(&entity.labels, &filter.labels) {
            continue;
        }

        entities.push(entity);

        if let Some(limit) = limit
            && entities.len() >= limit
        {
            break;
        }
    }

    entities
}

/// Resolve one topology entity from one pinned image.
pub(crate) fn entity_in_image<'a>(
    image: &'a WorldImage,
    entity_id: &str,
) -> RuntimeResult<&'a WorldEntity> {
    image.entity(entity_id).ok_or_else(|| {
        RuntimeError::ResourceNotFound {
            resource_id: 0,
            resource_kind: Some(format!("entity:{entity_id}")),
        }
        .boxed()
    })
}

/// List topology edges in stable id order.
pub(crate) fn list_edges<'a>(
    image: &'a WorldImage,
    filter: &EdgeListFilter,
    after: Option<&str>,
    limit: Option<usize>,
) -> Vec<&'a WorldEdge> {
    let mut edges = Vec::new();

    // stable topology edge scan
    for edge in image.topology.edges().values() {
        if let Some(after) = after
            && edge.id.as_str() <= after
        {
            continue;
        }

        if let Some(kind) = filter.kind.as_deref()
            && edge.kind.as_str() != kind
        {
            continue;
        }

        if let Some(from) = filter.from.as_deref()
            && edge.from.as_str() != from
        {
            continue;
        }

        if let Some(to) = filter.to.as_deref()
            && edge.to.as_str() != to
        {
            continue;
        }

        if !labels_match_selectors(&edge.labels, &filter.labels) {
            continue;
        }

        edges.push(edge);

        if let Some(limit) = limit
            && edges.len() >= limit
        {
            break;
        }
    }

    edges
}

/// Resolve one topology edge from one pinned image.
pub(crate) fn edge_in_image<'a>(
    image: &'a WorldImage,
    edge_id: &str,
) -> RuntimeResult<&'a WorldEdge> {
    image.edge(edge_id).ok_or_else(|| {
        RuntimeError::ResourceNotFound {
            resource_id: 0,
            resource_kind: Some(format!("edge:{edge_id}")),
        }
        .boxed()
    })
}
