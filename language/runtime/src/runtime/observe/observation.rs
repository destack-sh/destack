use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::platform::{ResourceBacking, ResourceCapture, ResourcePortability};
use crate::runtime::time::WorldInstant;
use crate::runtime::world::{Moment, WorldResourceId};
use crate::runtime::{AgentId, RuntimeId};

/// Stable sequence number for one observation entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ObservationSequence(u64);

impl ObservationSequence {
    /// Create one observation sequence number.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw observation sequence.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Return the next observation sequence.
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// Stable identifier for one observation subscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ObservationSubscriptionId(u64);

impl ObservationSubscriptionId {
    /// Create one observation subscription identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw observation subscription identifier.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Observation category for emitted runtime or user facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationCategory {
    /// Runtime lifecycle and engine diagnostics.
    Runtime,
    /// Topology mutation and graph diagnostics.
    Topology,
    /// Resource lifecycle and capability diagnostics.
    Resource,
    /// Scheduler and execution-lane diagnostics.
    Scheduler,
    /// General diagnostic and policy notices.
    Diagnostic,
    /// Telemetry, tracing, and performance instrumentation.
    Telemetry,
    /// Domain-level user or library observations.
    Domain,
}

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
    /// One agent-scoped observation.
    Agent {
        /// Optional owning runtime identifier when known.
        runtime_id: Option<RuntimeId>,
        /// Agent identifier for this scope.
        agent_id: AgentId,
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
        /// Agent identifier that owns the resource.
        agent_id: AgentId,
        /// Logical world resource identifier.
        resource_id: WorldResourceId,
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

    /// Create one agent scope.
    pub const fn agent(runtime_id: Option<RuntimeId>, agent_id: AgentId) -> Self {
        Self::Agent {
            runtime_id,
            agent_id,
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
    pub const fn resource(agent_id: AgentId, resource_id: WorldResourceId) -> Self {
        Self::Resource {
            agent_id,
            resource_id,
        }
    }

    /// Return the runtime id for this scope when present.
    pub const fn runtime_id(&self) -> Option<RuntimeId> {
        match self {
            Self::Runtime { runtime_id } => Some(*runtime_id),
            Self::Agent {
                runtime_id: Some(runtime_id),
                ..
            } => Some(*runtime_id),
            _ => None,
        }
    }

    /// Return the agent id for this scope when present.
    pub const fn agent_id(&self) -> Option<AgentId> {
        match self {
            Self::Agent { agent_id, .. } => Some(*agent_id),
            Self::Resource { agent_id, .. } => Some(*agent_id),
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
    pub const fn resource_id(&self) -> Option<WorldResourceId> {
        match self {
            Self::Resource { resource_id, .. } => Some(*resource_id),
            _ => None,
        }
    }
}

/// Filter options for one observation subscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationOptions {
    /// Include runtime observations.
    pub runtime: bool,
    /// Include topology observations.
    pub topology: bool,
    /// Include resource observations.
    pub resource: bool,
    /// Include scheduler observations.
    pub scheduler: bool,
    /// Include diagnostic observations.
    pub diagnostic: bool,
    /// Include telemetry observations.
    pub telemetry: bool,
    /// Include domain observations.
    pub domain: bool,
}

impl Default for ObservationOptions {
    fn default() -> Self {
        Self {
            runtime: true,
            topology: true,
            resource: true,
            scheduler: true,
            diagnostic: true,
            telemetry: true,
            domain: true,
        }
    }
}

impl ObservationOptions {
    /// Return whether this filter allows one observation category.
    pub const fn allows(self, category: ObservationCategory) -> bool {
        match category {
            ObservationCategory::Runtime => self.runtime,
            ObservationCategory::Topology => self.topology,
            ObservationCategory::Resource => self.resource,
            ObservationCategory::Scheduler => self.scheduler,
            ObservationCategory::Diagnostic => self.diagnostic,
            ObservationCategory::Telemetry => self.telemetry,
            ObservationCategory::Domain => self.domain,
        }
    }
}

/// One emitted observable fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    /// The observation category.
    pub category: ObservationCategory,
    /// The observation scope.
    pub scope: ObservationScope,
    /// Stable observation name.
    pub name: String,
    /// Structured observation labels.
    pub labels: BTreeMap<String, String>,
    /// Structured observation annotations.
    pub annotations: BTreeMap<String, String>,
}

impl Observation {
    /// Create one observation from explicit parts.
    pub fn new(
        category: ObservationCategory,
        scope: ObservationScope,
        name: impl Into<String>,
    ) -> Self {
        Self {
            category,
            scope,
            name: name.into(),
            labels: BTreeMap::new(),
            annotations: BTreeMap::new(),
        }
    }

    /// Create one world-scoped structured-annotation observation.
    pub fn world_annotations<K, V>(
        category: ObservationCategory,
        name: impl Into<String>,
        annotations: impl IntoIterator<Item = (K, V)>,
    ) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        Self::annotations(category, ObservationScope::world(), name, annotations)
    }

    /// Create one runtime-scoped structured-annotation observation.
    pub fn runtime_annotations<K, V>(
        category: ObservationCategory,
        runtime_id: RuntimeId,
        name: impl Into<String>,
        annotations: impl IntoIterator<Item = (K, V)>,
    ) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        Self::annotations(
            category,
            ObservationScope::runtime(runtime_id),
            name,
            annotations,
        )
    }

    /// Create one agent-scoped structured-annotation observation.
    pub fn agent_annotations<K, V>(
        category: ObservationCategory,
        runtime_id: Option<RuntimeId>,
        agent_id: AgentId,
        name: impl Into<String>,
        annotations: impl IntoIterator<Item = (K, V)>,
    ) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        Self::annotations(
            category,
            ObservationScope::agent(runtime_id, agent_id),
            name,
            annotations,
        )
    }

    /// Create one entity-scoped structured-annotation observation.
    pub fn entity_annotations<K, V>(
        category: ObservationCategory,
        entity_id: impl Into<String>,
        name: impl Into<String>,
        annotations: impl IntoIterator<Item = (K, V)>,
    ) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        Self::annotations(
            category,
            ObservationScope::entity(entity_id),
            name,
            annotations,
        )
    }

    /// Create one edge-scoped structured-annotation observation.
    pub fn edge_annotations<K, V>(
        category: ObservationCategory,
        edge_id: impl Into<String>,
        name: impl Into<String>,
        annotations: impl IntoIterator<Item = (K, V)>,
    ) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        Self::annotations(category, ObservationScope::edge(edge_id), name, annotations)
    }

    /// Create one structured-annotation observation.
    pub fn annotations<K, V>(
        category: ObservationCategory,
        scope: ObservationScope,
        name: impl Into<String>,
        annotations: impl IntoIterator<Item = (K, V)>,
    ) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        let annotations = annotations
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect();

        Self {
            category,
            scope,
            name: name.into(),
            labels: BTreeMap::new(),
            annotations,
        }
    }

    /// Create one resource-attached observation.
    pub fn resource_attached(
        agent_id: AgentId,
        resource_id: WorldResourceId,
        backing: ResourceBacking,
        capture: ResourceCapture,
        portability: ResourcePortability,
    ) -> Self {
        Self::annotations(
            ObservationCategory::Resource,
            ObservationScope::resource(agent_id, resource_id),
            "resource.attached",
            [
                ("agent_id", agent_id.0.to_string()),
                ("resource_id", resource_id.resource_id.0.to_string()),
                ("backing", format!("{backing:?}")),
                ("capture", format!("{capture:?}")),
                ("portability", format!("{portability:?}")),
            ],
        )
    }

    /// Create one resource-detached observation.
    pub fn resource_detached(agent_id: AgentId, resource_id: WorldResourceId) -> Self {
        Self::annotations(
            ObservationCategory::Resource,
            ObservationScope::resource(agent_id, resource_id),
            "resource.detached",
            [
                ("agent_id", agent_id.0.to_string()),
                ("resource_id", resource_id.resource_id.0.to_string()),
            ],
        )
    }

    /// Create one scheduler-progress observation.
    pub fn scheduler_progressed() -> Self {
        Self::new(
            ObservationCategory::Scheduler,
            ObservationScope::world(),
            "scheduler.progressed",
        )
    }

    /// Create one scheduler-advanced-time observation.
    pub fn scheduler_advanced_time(deadline: WorldInstant) -> Self {
        Self::annotations(
            ObservationCategory::Scheduler,
            ObservationScope::world(),
            "scheduler.advanced_time",
            [("deadline_ns", deadline.get().to_string())],
        )
    }

    /// Return one copy of this observation with one additional label.
    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Return the value for one observation annotation when present.
    pub fn annotation(&self, key: &str) -> Option<&str> {
        self.annotations.get(key).map(String::as_str)
    }

    /// Return the value for one observation label when present.
    pub fn label_value(&self, key: &str) -> Option<&str> {
        self.labels.get(key).map(String::as_str)
    }
}

/// One recorded observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationRecord {
    /// Observation sequence number.
    pub sequence: ObservationSequence,
    /// Execution coordinate where this observation was emitted.
    pub moment: Moment,
    /// Emitted observation payload.
    pub observation: Observation,
}
