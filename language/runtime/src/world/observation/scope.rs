use serde::{Deserialize, Serialize};

use crate::host::ResourceId;
use crate::runtime::{RuntimeId, WorkerId};

/// Scope for one emitted observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationScope {
    /// One world-scoped observation.
    World,
    /// One runtime-scoped observation.
    Runtime {
        /// Runtime identifier for this scope.
        runtime_id: RuntimeId,
    },
    /// One worker-scoped observation.
    Worker {
        /// Optional owning runtime identifier when known.
        runtime_id: Option<RuntimeId>,
        /// Worker identifier for this scope.
        worker_id: WorkerId,
    },
    /// One topology entity-scoped observation.
    Entity {
        /// Entity identifier for this scope.
        entity_id: String,
    },
    /// One topology edge-scoped observation.
    Edge {
        /// Edge identifier for this scope.
        edge_id: String,
    },
    /// One resource-scoped observation.
    Resource {
        /// Worker identifier that owns the resource.
        worker_id: WorkerId,
        /// World resource identifier.
        resource_id: ResourceId,
    },
}

impl ObservationScope {
    /// Create one world scope.
    pub const fn world() -> Self {
        Self::World
    }

    /// Create one runtime scope.
    pub const fn runtime(runtime_id: RuntimeId) -> Self {
        Self::Runtime { runtime_id }
    }

    /// Create one worker scope.
    pub const fn worker(runtime_id: Option<RuntimeId>, worker_id: WorkerId) -> Self {
        Self::Worker {
            runtime_id,
            worker_id,
        }
    }

    /// Create one entity scope.
    pub fn entity(entity_id: impl Into<String>) -> Self {
        Self::Entity {
            entity_id: entity_id.into(),
        }
    }

    /// Create one edge scope.
    pub fn edge(edge_id: impl Into<String>) -> Self {
        Self::Edge {
            edge_id: edge_id.into(),
        }
    }

    /// Create one resource scope.
    pub const fn resource(worker_id: WorkerId, resource_id: ResourceId) -> Self {
        Self::Resource {
            worker_id,
            resource_id,
        }
    }

    /// Return the runtime id for this scope when present.
    pub const fn runtime_id(&self) -> Option<RuntimeId> {
        match self {
            Self::Runtime { runtime_id } => Some(*runtime_id),
            Self::Worker {
                runtime_id: Some(runtime_id),
                ..
            } => Some(*runtime_id),
            _ => None,
        }
    }

    /// Return the worker id for this scope when present.
    pub const fn worker_id(&self) -> Option<WorkerId> {
        match self {
            Self::Worker { worker_id, .. } => Some(*worker_id),
            Self::Resource { worker_id, .. } => Some(*worker_id),
            _ => None,
        }
    }

    /// Return the entity id for this scope when present.
    pub fn entity_id(&self) -> Option<&str> {
        match self {
            Self::Entity { entity_id } => Some(entity_id),
            _ => None,
        }
    }

    /// Return the edge id for this scope when present.
    pub fn edge_id(&self) -> Option<&str> {
        match self {
            Self::Edge { edge_id } => Some(edge_id),
            _ => None,
        }
    }

    /// Return the resource id for this scope when present.
    pub const fn resource_id(&self) -> Option<ResourceId> {
        match self {
            Self::Resource { resource_id, .. } => Some(*resource_id),
            _ => None,
        }
    }
}
