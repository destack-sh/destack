use serde::{Deserialize, Serialize};
use tspp_program as program;
use tspp_serde::Reflect;

use crate::host::ResourceId;
use crate::runtime::RuntimeId;
use crate::worker::WorkerId;

/// Scope for one emitted Observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum ObservationScope {
    /// One World-scoped Observation.
    World,
    /// One Runtime-scoped Observation.
    Runtime {
        /// Runtime identifier for this scope.
        runtime_id: RuntimeId,
    },
    /// One Worker-scoped Observation.
    Worker {
        /// Worker identifier for this scope.
        worker_id: WorkerId,
    },
    /// One Runtime and Worker-scoped Observation.
    RuntimeWorker {
        /// Runtime identifier for this scope.
        runtime_id: RuntimeId,
        /// Worker identifier for this scope.
        worker_id: WorkerId,
    },
    /// One Fiber-scoped Observation.
    Fiber {
        /// Runtime identifier for this scope.
        runtime_id: RuntimeId,
        /// Worker identifier for this scope.
        worker_id: WorkerId,
        /// Fiber identifier for this scope.
        fiber_id: program::FiberId,
    },
    /// One topology Entity-scoped Observation.
    Entity {
        /// Entity identifier for this scope.
        entity_id: String,
    },
    /// One topology Edge-scoped Observation.
    Edge {
        /// Edge identifier for this scope.
        edge_id: String,
    },
    /// One Resource-scoped Observation.
    Resource {
        /// Resource identifier for this scope.
        resource_id: ResourceId,
    },
}

impl ObservationScope {
    /// Create one World scope.
    pub const fn world() -> Self {
        Self::World
    }

    /// Create one Runtime scope.
    pub const fn runtime(runtime_id: RuntimeId) -> Self {
        Self::Runtime { runtime_id }
    }

    /// Create one Worker scope.
    pub fn worker(runtime_id: Option<RuntimeId>, worker_id: WorkerId) -> Self {
        match runtime_id {
            Some(runtime_id) => Self::RuntimeWorker {
                runtime_id,
                worker_id,
            },
            None => Self::Worker { worker_id },
        }
    }

    /// Create one Fiber scope.
    pub fn fiber(runtime_id: RuntimeId, worker_id: WorkerId, fiber_id: program::FiberId) -> Self {
        Self::Fiber {
            runtime_id,
            worker_id,
            fiber_id,
        }
    }

    /// Create one Entity scope.
    pub fn entity(entity_id: impl Into<String>) -> Self {
        Self::Entity {
            entity_id: entity_id.into(),
        }
    }

    /// Create one Edge scope.
    pub fn edge(edge_id: impl Into<String>) -> Self {
        Self::Edge {
            edge_id: edge_id.into(),
        }
    }

    /// Create one Resource scope.
    pub const fn resource(resource_id: ResourceId) -> Self {
        Self::Resource { resource_id }
    }

    /// Return the Runtime identifier for this scope when present.
    pub const fn runtime_id(&self) -> Option<RuntimeId> {
        match self {
            Self::Runtime { runtime_id }
            | Self::RuntimeWorker { runtime_id, .. }
            | Self::Fiber { runtime_id, .. } => Some(*runtime_id),
            _ => None,
        }
    }

    /// Return the Worker identifier for this scope when present.
    pub const fn worker_id(&self) -> Option<WorkerId> {
        match self {
            Self::Worker { worker_id }
            | Self::RuntimeWorker { worker_id, .. }
            | Self::Fiber { worker_id, .. } => Some(*worker_id),
            Self::Resource { resource_id } => Some(resource_id.worker_id),
            _ => None,
        }
    }

    /// Return the Fiber identifier for this scope when present.
    pub const fn fiber_id(&self) -> Option<program::FiberId> {
        match self {
            Self::Fiber { fiber_id, .. } => Some(*fiber_id),
            _ => None,
        }
    }

    /// Return the Entity identifier for this scope when present.
    pub fn entity_id(&self) -> Option<&str> {
        match self {
            Self::Entity { entity_id } => Some(entity_id.as_str()),
            _ => None,
        }
    }

    /// Return the Edge identifier for this scope when present.
    pub fn edge_id(&self) -> Option<&str> {
        match self {
            Self::Edge { edge_id } => Some(edge_id.as_str()),
            _ => None,
        }
    }

    /// Return the Resource identifier for this scope when present.
    pub const fn resource_id(&self) -> Option<ResourceId> {
        match self {
            Self::Resource { resource_id } => Some(*resource_id),
            _ => None,
        }
    }
}
