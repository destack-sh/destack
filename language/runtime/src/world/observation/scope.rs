use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::host::ResourceId;
use crate::runtime::{RuntimeId, WorkerId};

/// Scope for one emitted observation.
#[derive(Debug, Clone, PartialEq, Eq)]
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
        /// Worker identifier for this scope.
        worker_id: WorkerId,
    },
    /// One runtime-worker-scoped observation.
    RuntimeWorker(Box<RuntimeWorkerScope>),
    /// One topology entity-scoped observation.
    Entity(Box<String>),
    /// One topology edge-scoped observation.
    Edge(Box<String>),
    /// One resource-scoped observation.
    Resource(Box<ResourceId>),
}

/// Runtime-worker observation scope payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeWorkerScope {
    /// Runtime identifier for this scope.
    pub runtime_id: RuntimeId,
    /// Worker identifier for this scope.
    pub worker_id: WorkerId,
}

/// Serialized observation-scope record.
#[derive(Serialize, Deserialize)]
enum ObservationScopeRecord {
    /// One world-scoped observation.
    World,
    /// One runtime-scoped observation.
    Runtime {
        /// Runtime identifier for this scope.
        runtime_id: RuntimeId,
    },
    /// One worker-scoped observation.
    Worker {
        /// Worker identifier for this scope.
        worker_id: WorkerId,
    },
    /// One runtime-worker-scoped observation.
    RuntimeWorker {
        /// Runtime identifier for this scope.
        runtime_id: RuntimeId,
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
        /// World resource identifier.
        resource_id: ResourceId,
    },
}

impl Serialize for ObservationScope {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let record = match self {
            Self::World => ObservationScopeRecord::World,
            Self::Runtime { runtime_id } => ObservationScopeRecord::Runtime {
                runtime_id: *runtime_id,
            },
            Self::Worker { worker_id } => ObservationScopeRecord::Worker {
                worker_id: *worker_id,
            },
            Self::RuntimeWorker(scope) => ObservationScopeRecord::RuntimeWorker {
                runtime_id: scope.runtime_id,
                worker_id: scope.worker_id,
            },
            Self::Entity(entity_id) => ObservationScopeRecord::Entity {
                entity_id: entity_id.as_str().to_string(),
            },
            Self::Edge(edge_id) => ObservationScopeRecord::Edge {
                edge_id: edge_id.as_str().to_string(),
            },
            Self::Resource(resource_id) => ObservationScopeRecord::Resource {
                resource_id: **resource_id,
            },
        };

        record.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ObservationScope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let record = ObservationScopeRecord::deserialize(deserializer)?;
        let scope = match record {
            ObservationScopeRecord::World => Self::World,
            ObservationScopeRecord::Runtime { runtime_id } => Self::Runtime { runtime_id },
            ObservationScopeRecord::Worker { worker_id } => Self::Worker { worker_id },
            ObservationScopeRecord::RuntimeWorker {
                runtime_id,
                worker_id,
            } => Self::RuntimeWorker(Box::new(RuntimeWorkerScope {
                runtime_id,
                worker_id,
            })),
            ObservationScopeRecord::Entity { entity_id } => Self::Entity(Box::new(entity_id)),
            ObservationScopeRecord::Edge { edge_id } => Self::Edge(Box::new(edge_id)),
            ObservationScopeRecord::Resource { resource_id } => {
                Self::Resource(Box::new(resource_id))
            }
        };

        Ok(scope)
    }
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
    pub fn worker(runtime_id: Option<RuntimeId>, worker_id: WorkerId) -> Self {
        match runtime_id {
            Some(runtime_id) => Self::RuntimeWorker(Box::new(RuntimeWorkerScope {
                runtime_id,
                worker_id,
            })),
            None => Self::Worker { worker_id },
        }
    }

    /// Create one entity scope.
    pub fn entity(entity_id: impl Into<String>) -> Self {
        let entity_id = Box::new(entity_id.into());

        Self::Entity(entity_id)
    }

    /// Create one edge scope.
    pub fn edge(edge_id: impl Into<String>) -> Self {
        let edge_id = Box::new(edge_id.into());

        Self::Edge(edge_id)
    }

    /// Create one resource scope.
    pub fn resource(resource_id: ResourceId) -> Self {
        Self::Resource(Box::new(resource_id))
    }

    /// Return the runtime id for this scope when present.
    pub const fn runtime_id(&self) -> Option<RuntimeId> {
        match self {
            Self::Runtime { runtime_id } => Some(*runtime_id),
            Self::RuntimeWorker(scope) => Some(scope.runtime_id),
            _ => None,
        }
    }

    /// Return the worker id for this scope when present.
    pub const fn worker_id(&self) -> Option<WorkerId> {
        match self {
            Self::Worker { worker_id, .. } => Some(*worker_id),
            Self::RuntimeWorker(scope) => Some(scope.worker_id),
            Self::Resource(resource_id) => Some(resource_id.worker_id),
            _ => None,
        }
    }

    /// Return the entity id for this scope when present.
    pub fn entity_id(&self) -> Option<&str> {
        match self {
            Self::Entity(entity_id) => Some(entity_id.as_str()),
            _ => None,
        }
    }

    /// Return the edge id for this scope when present.
    pub fn edge_id(&self) -> Option<&str> {
        match self {
            Self::Edge(edge_id) => Some(edge_id.as_str()),
            _ => None,
        }
    }

    /// Return the resource id for this scope when present.
    pub const fn resource_id(&self) -> Option<ResourceId> {
        match self {
            Self::Resource(resource_id) => Some(**resource_id),
            _ => None,
        }
    }
}
