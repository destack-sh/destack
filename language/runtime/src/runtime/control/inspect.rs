use std::collections::BTreeMap;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::world::{
    BranchId, Image, RevisionId, RuntimeId, WorldEdge, WorldEntity, WorldResource, WorldResourceId,
};
use crate::runtime::{AgentId, AgentImage, RuntimeImage};

/// Runtime list filter decoded from one low-level ABI surface.
#[derive(Debug, Clone, Default)]
pub(crate) struct RuntimeListFilter {
    /// Optional exact runtime name match.
    pub name: Option<String>,
    /// Optional label selectors.
    pub labels: Vec<(String, Option<String>)>,
}

/// Agent list filter decoded from one low-level ABI surface.
#[derive(Debug, Clone, Default)]
pub(crate) struct AgentListFilter {
    /// Optional owning runtime id.
    pub runtime_id: Option<RuntimeId>,
    /// Optional exact agent name match.
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
    pub revision_id: Option<RevisionId>,
    /// Optional exact checkpoint name.
    pub name: Option<String>,
    /// Optional label selectors.
    pub labels: Vec<(String, Option<String>)>,
}

/// Image list filter decoded from one low-level ABI surface.
#[derive(Debug, Clone, Default)]
pub(crate) struct ImageListFilter {
    /// Optional owning revision id.
    pub revision_id: Option<RevisionId>,
}

/// Resource list filter decoded from one low-level ABI surface.
#[derive(Debug, Clone, Default)]
pub(crate) struct ResourceListFilter {
    /// Optional owning runtime id.
    pub runtime_id: Option<RuntimeId>,
    /// Optional owning agent id.
    pub agent_id: Option<AgentId>,
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
    image: &'a Image,
    filter: &RuntimeListFilter,
    after: Option<RuntimeId>,
    limit: Option<usize>,
) -> RuntimeResult<Vec<&'a RuntimeImage>> {
    let after = after.map(|id| id.0).unwrap_or(0);
    let mut runtimes = Vec::new();

    // stable runtime scan
    for runtime in image.runtimes.values() {
        if runtime.runtime_id.0 <= after {
            continue;
        }

        if let Some(name) = filter.name.as_deref()
            && runtime.name != name
        {
            continue;
        }

        let labels = image.runtime_labels(runtime.runtime_id)?;
        if !labels_match_selectors(labels, &filter.labels) {
            continue;
        }

        runtimes.push(runtime);

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
    image: &Image,
    runtime_id: RuntimeId,
) -> RuntimeResult<&RuntimeImage> {
    image.runtimes.get(&runtime_id).ok_or_else(|| {
        RuntimeError::RuntimeNotFound {
            runtime_id: runtime_id.0,
        }
        .boxed()
    })
}

/// List agents in stable id order.
pub(crate) fn list_agents<'a>(
    image: &'a Image,
    filter: &AgentListFilter,
    after: Option<AgentId>,
    limit: Option<usize>,
) -> RuntimeResult<Vec<&'a AgentImage>> {
    let after = after.map(|id| id.0).unwrap_or(0);
    let mut agents = Vec::new();

    // stable agent scan
    for agent in image.agents.values() {
        if agent.agent_id.0 <= after {
            continue;
        }

        if let Some(runtime_id) = filter.runtime_id
            && agent.runtime_id != runtime_id
        {
            continue;
        }

        if let Some(name) = filter.name.as_deref()
            && agent.name != name
        {
            continue;
        }

        let has_pending_work = agent.has_pending_work();
        if let Some(expected) = filter.has_pending_work
            && has_pending_work != expected
        {
            continue;
        }

        let labels = image.agent_labels(agent.agent_id)?;
        if !labels_match_selectors(labels, &filter.labels) {
            continue;
        }

        agents.push(agent);

        if let Some(limit) = limit
            && agents.len() >= limit
        {
            break;
        }
    }

    Ok(agents)
}

/// Resolve one agent from one pinned image.
pub(crate) fn agent_in_image(image: &Image, agent_id: AgentId) -> RuntimeResult<&AgentImage> {
    image.agents.get(&agent_id).ok_or_else(|| {
        RuntimeError::AgentNotFound {
            agent_id: agent_id.0,
        }
        .boxed()
    })
}

/// Return whether one logical resource matches the structured filter.
pub(crate) fn resource_matches_filter(
    image: &Image,
    resource: &WorldResource,
    filter: &ResourceListFilter,
) -> bool {
    if let Some(runtime_id) = filter.runtime_id {
        let Some(agent) = image.agents.get(&resource.id.agent_id) else {
            return false;
        };

        if agent.runtime_id != runtime_id {
            return false;
        }
    }

    if let Some(agent_id) = filter.agent_id
        && resource.id.agent_id != agent_id
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
    image: &'a Image,
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
pub(crate) fn resource_in_image<'a>(
    image: &'a Image,
    resource_id: WorldResourceId,
) -> RuntimeResult<&'a WorldResource> {
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
    image: &'a Image,
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
    image: &'a Image,
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
    image: &'a Image,
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
pub(crate) fn edge_in_image<'a>(image: &'a Image, edge_id: &str) -> RuntimeResult<&'a WorldEdge> {
    image.edge(edge_id).ok_or_else(|| {
        RuntimeError::ResourceNotFound {
            resource_id: 0,
            resource_kind: Some(format!("edge:{edge_id}")),
        }
        .boxed()
    })
}
