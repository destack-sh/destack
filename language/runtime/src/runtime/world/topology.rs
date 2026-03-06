use std::borrow::Borrow;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use super::builtin::{
    base_supported_edge_faults, base_supported_entity_faults, builtin_edge_kinds,
    builtin_entity_kinds, builtin_resource_entity_kinds,
};
use super::constants::{
    BUILTIN_AGENT_KIND_ID, BUILTIN_AGENT_OWNS_RESOURCE_EDGE_KIND_ID, BUILTIN_RUNTIME_KIND_ID,
    BUILTIN_RUNTIME_OWNS_AGENT_EDGE_KIND_ID, LABEL_AGENT_NAME, LABEL_RESOURCE_LABEL,
    LABEL_RUNTIME_NAME,
};
use super::resource::WorldResourceId;
use crate::runtime::AgentId;

/// Stable identifier for one runtime instance in one world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RuntimeId(pub u64);

impl RuntimeId {
    /// Return the canonical topology entity id for this runtime.
    pub fn entity_id(self) -> WorldEntityId {
        WorldEntityId::new(format!("runtime.{}", self.0))
    }

    /// Return the canonical ownership edge id for one agent owned by this runtime.
    pub fn owns_agent_edge_id(self, agent_id: AgentId) -> WorldEdgeId {
        WorldEdgeId::new(format!("runtime.{}.owns.agent.{}", self.0, agent_id.0))
    }
}

impl fmt::Display for RuntimeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AgentId {
    /// Return the canonical topology entity id for this agent.
    pub fn entity_id(self) -> WorldEntityId {
        WorldEntityId::new(format!("agent.{}", self.0))
    }
}

/// Stable identifier for one topology entity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorldEntityId(pub String);

impl WorldEntityId {
    /// Create one entity id.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the stable id as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Borrow<str> for WorldEntityId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl From<String> for WorldEntityId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for WorldEntityId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Display for WorldEntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable identifier for one topology edge.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorldEdgeId(pub String);

impl WorldEdgeId {
    /// Create one edge id.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the stable id as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Borrow<str> for WorldEdgeId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl From<String> for WorldEdgeId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for WorldEdgeId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Display for WorldEdgeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable identifier for one topology entity kind.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorldEntityKind(pub String);

impl WorldEntityKind {
    /// Create one entity kind id.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the stable id as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Borrow<str> for WorldEntityKind {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl From<String> for WorldEntityKind {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for WorldEntityKind {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Display for WorldEntityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable identifier for one topology edge kind.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorldEdgeKind(pub String);

impl WorldEdgeKind {
    /// Create one edge kind id.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the stable id as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Borrow<str> for WorldEdgeKind {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl From<String> for WorldEdgeKind {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for WorldEdgeKind {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Display for WorldEdgeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error type for world topology definition and mutation failures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum TopologyError {
    /// One kind identifier was empty.
    EmptyKindId,
    /// One topology entity was already defined.
    DuplicateEntity {
        /// The duplicated entity identifier.
        entity_id: WorldEntityId,
    },
    /// One topology edge was already defined.
    DuplicateEdge {
        /// The duplicated edge identifier.
        edge_id: WorldEdgeId,
    },
    /// One entity kind was already defined.
    DuplicateEntityKind {
        /// The duplicated kind identifier.
        kind: WorldEntityKind,
    },
    /// One edge kind was already defined.
    DuplicateEdgeKind {
        /// The duplicated kind identifier.
        kind: WorldEdgeKind,
    },
    /// One entity kind was not defined.
    UnknownEntityKind {
        /// The missing kind identifier.
        kind: WorldEntityKind,
    },
    /// One edge kind was not defined.
    UnknownEdgeKind {
        /// The missing kind identifier.
        kind: WorldEdgeKind,
    },
    /// One topology entity was missing.
    UnknownEntity {
        /// The missing entity identifier.
        entity_id: WorldEntityId,
        /// The entity role in the failed relation.
        role: TopologyEntityRole,
    },
}

/// Result type for world topology operations.
pub(crate) type TopologyResult<T> = Result<T, TopologyError>;

/// Role of one referenced topology entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum TopologyEntityRole {
    /// The source side of one edge.
    Source,
    /// The destination side of one edge.
    Destination,
}

impl fmt::Display for TopologyEntityRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let role = match self {
            TopologyEntityRole::Source => "source",
            TopologyEntityRole::Destination => "destination",
        };

        f.write_str(role)
    }
}

impl fmt::Display for TopologyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TopologyError::EmptyKindId => f.write_str("topology kind id must not be empty"),
            TopologyError::DuplicateEntity { entity_id } => {
                write!(f, "topology entity {entity_id} is already defined")
            }
            TopologyError::DuplicateEdge { edge_id } => {
                write!(f, "topology edge {edge_id} is already defined")
            }
            TopologyError::DuplicateEntityKind { kind } => {
                write!(f, "topology entity kind {kind} is already defined")
            }
            TopologyError::DuplicateEdgeKind { kind } => {
                write!(f, "topology edge kind {kind} is already defined")
            }
            TopologyError::UnknownEntityKind { kind } => {
                write!(f, "topology entity kind {kind} is not defined")
            }
            TopologyError::UnknownEdgeKind { kind } => {
                write!(f, "topology edge kind {kind} is not defined")
            }
            TopologyError::UnknownEntity { entity_id, role } => {
                write!(f, "topology {role} entity {entity_id} does not exist")
            }
        }
    }
}

impl std::error::Error for TopologyError {}

/// Topology entity-kind registration payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldEntityKindDefinition {
    /// Stable kind identifier.
    pub kind: WorldEntityKind,
    /// Kind labels for selector matching.
    pub labels: BTreeMap<String, String>,
    /// Fault verbs supported by this kind.
    #[serde(default)]
    pub supported_faults: BTreeSet<String>,
}

impl WorldEntityKindDefinition {
    /// Create one entity kind definition.
    pub fn new(kind: impl Into<WorldEntityKind>) -> Self {
        Self {
            kind: kind.into(),
            labels: BTreeMap::new(),
            supported_faults: BTreeSet::new(),
        }
    }

    /// Add one label.
    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Replace labels.
    pub fn labels(mut self, labels: BTreeMap<String, String>) -> Self {
        self.labels = labels;
        self
    }

    /// Add one supported fault verb id.
    pub fn supports_fault(mut self, verb_id: impl Into<String>) -> Self {
        self.supported_faults.insert(verb_id.into());
        self
    }

    /// Extend supported fault verb ids.
    pub fn supports_faults(mut self, verb_ids: impl IntoIterator<Item = String>) -> Self {
        self.supported_faults.extend(verb_ids);
        self
    }
}

/// Topology edge-kind registration payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldEdgeKindDefinition {
    /// Stable kind identifier.
    pub kind: WorldEdgeKind,
    /// Kind labels for selector matching.
    pub labels: BTreeMap<String, String>,
    /// Fault verbs supported by this kind.
    #[serde(default)]
    pub supported_faults: BTreeSet<String>,
}

impl WorldEdgeKindDefinition {
    /// Create one edge kind definition.
    pub fn new(kind: impl Into<WorldEdgeKind>) -> Self {
        Self {
            kind: kind.into(),
            labels: BTreeMap::new(),
            supported_faults: BTreeSet::new(),
        }
    }

    /// Add one label.
    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Replace labels.
    pub fn labels(mut self, labels: BTreeMap<String, String>) -> Self {
        self.labels = labels;
        self
    }

    /// Add one supported fault verb id.
    pub fn supports_fault(mut self, verb_id: impl Into<String>) -> Self {
        self.supported_faults.insert(verb_id.into());
        self
    }

    /// Extend supported fault verb ids.
    pub fn supports_faults(mut self, verb_ids: impl IntoIterator<Item = String>) -> Self {
        self.supported_faults.extend(verb_ids);
        self
    }
}

/// Topology entity payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldEntity {
    /// Stable entity identifier.
    pub id: WorldEntityId,
    /// Stable entity kind identifier.
    pub kind: WorldEntityKind,
    /// Entity labels.
    pub labels: BTreeMap<String, String>,
}

impl WorldEntity {
    /// Create one entity payload.
    pub fn new(id: impl Into<WorldEntityId>, kind: impl Into<WorldEntityKind>) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            labels: BTreeMap::new(),
        }
    }

    /// Add one label.
    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Replace labels.
    pub fn labels(mut self, labels: BTreeMap<String, String>) -> Self {
        self.labels = labels;
        self
    }
}

/// Topology edge payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldEdge {
    /// Stable edge identifier.
    pub id: WorldEdgeId,
    /// Stable edge kind identifier.
    pub kind: WorldEdgeKind,
    /// Source entity identifier.
    pub from: WorldEntityId,
    /// Destination entity identifier.
    pub to: WorldEntityId,
    /// Edge labels.
    pub labels: BTreeMap<String, String>,
}

impl WorldEdge {
    /// Create one edge payload.
    pub fn new(
        id: impl Into<WorldEdgeId>,
        kind: impl Into<WorldEdgeKind>,
        from: impl Into<WorldEntityId>,
        to: impl Into<WorldEntityId>,
    ) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            from: from.into(),
            to: to.into(),
            labels: BTreeMap::new(),
        }
    }

    /// Add one label.
    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Replace labels.
    pub fn labels(mut self, labels: BTreeMap<String, String>) -> Self {
        self.labels = labels;
        self
    }
}

/// World topology graph and kind catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Topology {
    /// Registered entity kinds by kind id.
    entity_kinds: BTreeMap<WorldEntityKind, WorldEntityKindDefinition>,
    /// Registered edge kinds by kind id.
    edge_kinds: BTreeMap<WorldEdgeKind, WorldEdgeKindDefinition>,
    /// Topology entities by entity identifier.
    entities: BTreeMap<WorldEntityId, WorldEntity>,
    /// Topology edges by edge identifier.
    edges: BTreeMap<WorldEdgeId, WorldEdge>,
}

impl Default for Topology {
    fn default() -> Self {
        Self {
            entity_kinds: builtin_entity_kinds()
                .into_iter()
                .chain(builtin_resource_entity_kinds())
                .map(|kind| (kind.kind.clone(), kind))
                .collect(),
            edge_kinds: builtin_edge_kinds()
                .into_iter()
                .map(|kind| (kind.kind.clone(), kind))
                .collect(),
            entities: BTreeMap::new(),
            edges: BTreeMap::new(),
        }
    }
}

impl Topology {
    /// Create one topology with builtin kinds installed.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Return all registered entity kinds.
    pub(crate) fn entity_kinds(&self) -> &BTreeMap<WorldEntityKind, WorldEntityKindDefinition> {
        &self.entity_kinds
    }

    /// Return all registered edge kinds.
    pub(crate) fn edge_kinds(&self) -> &BTreeMap<WorldEdgeKind, WorldEdgeKindDefinition> {
        &self.edge_kinds
    }

    /// Return one entity kind supported fault set by kind id.
    pub(crate) fn entity_kind_supported_faults(&self, kind: &str) -> Option<&BTreeSet<String>> {
        self.entity_kinds
            .get(kind)
            .map(|kind| &kind.supported_faults)
    }

    /// Return one edge kind supported fault set by kind id.
    pub(crate) fn edge_kind_supported_faults(&self, kind: &str) -> Option<&BTreeSet<String>> {
        self.edge_kinds.get(kind).map(|kind| &kind.supported_faults)
    }

    /// Return all topology entities.
    pub(crate) fn entities(&self) -> &BTreeMap<WorldEntityId, WorldEntity> {
        &self.entities
    }

    /// Return all topology edges.
    pub(crate) fn edges(&self) -> &BTreeMap<WorldEdgeId, WorldEdge> {
        &self.edges
    }

    /// Return the rule subject metadata for one runtime.
    pub(crate) fn runtime_subject(
        &self,
        runtime_id: RuntimeId,
    ) -> Option<(&str, &BTreeMap<String, String>)> {
        let entity = self.entities.get(&runtime_id.entity_id())?;
        let name = entity.labels.get(LABEL_RUNTIME_NAME)?;

        Some((name.as_str(), &entity.labels))
    }

    /// Return the rule subject metadata for one agent.
    pub(crate) fn agent_subject(
        &self,
        agent_id: AgentId,
    ) -> Option<(&str, &BTreeMap<String, String>)> {
        let entity = self.entities.get(&agent_id.entity_id())?;
        let name = entity.labels.get(LABEL_AGENT_NAME)?;

        Some((name.as_str(), &entity.labels))
    }

    /// Add one runtime and its primary agent metadata.
    pub(crate) fn add_runtime(
        &mut self,
        runtime_id: RuntimeId,
        runtime_name: String,
        runtime_labels: BTreeMap<String, String>,
        primary_agent_id: AgentId,
        primary_agent_name: String,
        primary_agent_labels: BTreeMap<String, String>,
    ) -> TopologyResult<()> {
        // reject duplicate metadata upfront
        if self.entities.contains_key(&runtime_id.entity_id()) {
            return Err(TopologyError::DuplicateEntity {
                entity_id: runtime_id.entity_id(),
            });
        }
        if self.entities.contains_key(&primary_agent_id.entity_id()) {
            return Err(TopologyError::DuplicateEntity {
                entity_id: primary_agent_id.entity_id(),
            });
        }

        // runtime entity
        let runtime_entity =
            WorldEntity::new(runtime_id.entity_id(), BUILTIN_RUNTIME_KIND_ID).labels(
                entity_labels_with_name(runtime_labels, LABEL_RUNTIME_NAME, runtime_name),
            );
        self.upsert_entity(runtime_entity)?;

        // primary agent entity
        let agent_entity =
            WorldEntity::new(primary_agent_id.entity_id(), BUILTIN_AGENT_KIND_ID).labels(
                entity_labels_with_name(primary_agent_labels, LABEL_AGENT_NAME, primary_agent_name),
            );
        self.upsert_entity(agent_entity)?;

        // ownership edge
        let edge = WorldEdge::new(
            runtime_id.owns_agent_edge_id(primary_agent_id),
            BUILTIN_RUNTIME_OWNS_AGENT_EDGE_KIND_ID,
            runtime_id.entity_id(),
            primary_agent_id.entity_id(),
        );
        self.upsert_edge(edge)?;

        Ok(())
    }

    /// Add one additional agent metadata record.
    pub(crate) fn add_agent(
        &mut self,
        runtime_id: RuntimeId,
        agent_id: AgentId,
        agent_name: String,
        agent_labels: BTreeMap<String, String>,
    ) -> TopologyResult<()> {
        // reject missing owning runtime
        if !self.entities.contains_key(&runtime_id.entity_id()) {
            return Err(TopologyError::UnknownEntity {
                entity_id: runtime_id.entity_id(),
                role: TopologyEntityRole::Source,
            });
        }

        // reject duplicate agent metadata
        if self.entities.contains_key(&agent_id.entity_id()) {
            return Err(TopologyError::DuplicateEntity {
                entity_id: agent_id.entity_id(),
            });
        }

        // agent entity
        let agent_entity = WorldEntity::new(agent_id.entity_id(), BUILTIN_AGENT_KIND_ID).labels(
            entity_labels_with_name(agent_labels, LABEL_AGENT_NAME, agent_name),
        );
        self.upsert_entity(agent_entity)?;

        // ownership edge
        let edge = WorldEdge::new(
            runtime_id.owns_agent_edge_id(agent_id),
            BUILTIN_RUNTIME_OWNS_AGENT_EDGE_KIND_ID,
            runtime_id.entity_id(),
            agent_id.entity_id(),
        );
        self.upsert_edge(edge)?;

        Ok(())
    }

    /// Remove one agent metadata record and any now-orphaned runtime/resource metadata.
    pub(crate) fn remove_agent(&mut self, agent_id: AgentId) -> bool {
        let agent_entity_id = agent_id.entity_id();
        let Some(runtime_entity_id) = self.agent_runtime_entity_id(agent_id) else {
            return false;
        };

        // resource metadata owned by the agent
        let owned_resource_ids = self
            .edges
            .values()
            .filter(|edge| {
                edge.kind.as_str() == BUILTIN_AGENT_OWNS_RESOURCE_EDGE_KIND_ID
                    && edge.from == agent_entity_id
            })
            .map(|edge| edge.to.clone())
            .collect::<Vec<_>>();
        for resource_entity_id in owned_resource_ids {
            let _ = self.remove_entity(resource_entity_id.as_str());
        }

        // agent metadata
        let is_agent_removed = self.remove_entity(agent_entity_id.as_str());

        // remove the runtime metadata when the last agent disappears
        if !self.runtime_has_agents(runtime_entity_id.as_str()) {
            let _ = self.remove_entity(runtime_entity_id.as_str());
        }

        is_agent_removed
    }

    /// Attach one resource metadata record to one agent.
    pub(crate) fn attach_resource(
        &mut self,
        resource_id: WorldResourceId,
        resource_kind: WorldEntityKind,
        resource_label: Option<&str>,
    ) -> TopologyResult<()> {
        // reject missing owning agent
        if !self
            .entities
            .contains_key(&resource_id.agent_id.entity_id())
        {
            return Err(TopologyError::UnknownEntity {
                entity_id: resource_id.agent_id.entity_id(),
                role: TopologyEntityRole::Source,
            });
        }

        // resource entity
        let mut labels = BTreeMap::new();
        if let Some(resource_label) = resource_label {
            labels.insert(LABEL_RESOURCE_LABEL.to_string(), resource_label.to_string());
        }
        let resource_entity =
            WorldEntity::new(resource_id.entity_id(), resource_kind).labels(labels);
        self.upsert_entity(resource_entity)?;

        // ownership edge
        let edge = WorldEdge::new(
            resource_id.ownership_edge_id(),
            BUILTIN_AGENT_OWNS_RESOURCE_EDGE_KIND_ID,
            resource_id.agent_id.entity_id(),
            resource_id.entity_id(),
        );
        self.upsert_edge(edge)?;

        Ok(())
    }

    /// Detach one resource metadata record from one agent.
    pub(crate) fn detach_resource(&mut self, resource_id: WorldResourceId) -> bool {
        self.remove_entity(resource_id.entity_id().as_str())
    }

    /// Define one entity kind in topology.
    pub(crate) fn define_entity_kind(
        &mut self,
        mut kind: WorldEntityKindDefinition,
    ) -> TopologyResult<()> {
        // reject empty kind identifiers
        if kind.kind.as_str().is_empty() {
            return Err(TopologyError::EmptyKindId);
        }

        // reject duplicate kind identifiers
        if self.entity_kinds.contains_key(kind.kind.as_str()) {
            return Err(TopologyError::DuplicateEntityKind {
                kind: kind.kind.clone(),
            });
        }

        // default to base entity faults for user-defined kinds
        if kind.supported_faults.is_empty() {
            kind.supported_faults = base_supported_entity_faults();
        }

        // insert one kind definition
        self.entity_kinds.insert(kind.kind.clone(), kind);

        Ok(())
    }

    /// Define one edge kind in topology.
    pub(crate) fn define_edge_kind(
        &mut self,
        mut kind: WorldEdgeKindDefinition,
    ) -> TopologyResult<()> {
        // reject empty kind identifiers
        if kind.kind.as_str().is_empty() {
            return Err(TopologyError::EmptyKindId);
        }

        // reject duplicate kind identifiers
        if self.edge_kinds.contains_key(kind.kind.as_str()) {
            return Err(TopologyError::DuplicateEdgeKind {
                kind: kind.kind.clone(),
            });
        }

        // default to base edge faults for user-defined kinds
        if kind.supported_faults.is_empty() {
            kind.supported_faults = base_supported_edge_faults();
        }

        // insert one kind definition
        self.edge_kinds.insert(kind.kind.clone(), kind);

        Ok(())
    }

    /// Upsert one topology entity.
    pub(crate) fn upsert_entity(&mut self, entity: WorldEntity) -> TopologyResult<()> {
        self.expect_entity_kind(entity.kind.as_str())?;
        self.entities.insert(entity.id.clone(), entity);
        Ok(())
    }

    /// Remove one topology entity and all incident edges.
    pub(crate) fn remove_entity(&mut self, entity_id: &str) -> bool {
        let removed_edge_count = self.remove_incident_edges(entity_id);
        let is_entity_removed = self.entities.remove(entity_id).is_some();
        is_entity_removed || removed_edge_count > 0
    }

    /// Upsert one topology edge.
    pub(crate) fn upsert_edge(&mut self, edge: WorldEdge) -> TopologyResult<()> {
        self.expect_edge_kind(edge.kind.as_str())?;
        self.expect_entity(edge.from.as_str(), TopologyEntityRole::Source)?;
        self.expect_entity(edge.to.as_str(), TopologyEntityRole::Destination)?;
        self.edges.insert(edge.id.clone(), edge);
        Ok(())
    }

    /// Remove one topology edge.
    pub(crate) fn remove_edge(&mut self, edge_id: &str) -> bool {
        self.edges.remove(edge_id).is_some()
    }

    /// Expect one entity kind to be defined.
    fn expect_entity_kind(&self, kind_id: &str) -> TopologyResult<()> {
        if self.entity_kinds.contains_key(kind_id) {
            return Ok(());
        }

        Err(TopologyError::UnknownEntityKind {
            kind: WorldEntityKind::from(kind_id),
        })
    }

    /// Expect one edge kind to be defined.
    fn expect_edge_kind(&self, kind_id: &str) -> TopologyResult<()> {
        if self.edge_kinds.contains_key(kind_id) {
            return Ok(());
        }

        Err(TopologyError::UnknownEdgeKind {
            kind: WorldEdgeKind::from(kind_id),
        })
    }

    /// Expect one entity to exist.
    fn expect_entity(&self, entity_id: &str, role: TopologyEntityRole) -> TopologyResult<()> {
        if self.entities.contains_key(entity_id) {
            return Ok(());
        }

        Err(TopologyError::UnknownEntity {
            entity_id: WorldEntityId::from(entity_id),
            role,
        })
    }

    /// Remove all incident edges for one entity and return the number removed.
    fn remove_incident_edges(&mut self, entity_id: &str) -> usize {
        let before_edge_count = self.edges.len();
        self.edges
            .retain(|_, edge| edge.from.as_str() != entity_id && edge.to.as_str() != entity_id);

        before_edge_count.saturating_sub(self.edges.len())
    }

    /// Return the owning runtime id for one agent entity.
    fn agent_runtime_entity_id(&self, agent_id: AgentId) -> Option<WorldEntityId> {
        let agent_entity_id = agent_id.entity_id();
        let edge = self.edges.values().find(|edge| {
            edge.kind.as_str() == BUILTIN_RUNTIME_OWNS_AGENT_EDGE_KIND_ID
                && edge.to == agent_entity_id
        })?;

        Some(edge.from.clone())
    }

    /// Return whether one runtime still owns any agent metadata.
    fn runtime_has_agents(&self, runtime_entity_id: &str) -> bool {
        self.edges.values().any(|edge| {
            edge.kind.as_str() == BUILTIN_RUNTIME_OWNS_AGENT_EDGE_KIND_ID
                && edge.from.as_str() == runtime_entity_id
        })
    }
}

/// Add one reserved display-name label to one metadata label set.
fn entity_labels_with_name(
    mut labels: BTreeMap<String, String>,
    name_key: &str,
    name: String,
) -> BTreeMap<String, String> {
    labels.insert(name_key.to_string(), name);

    labels
}
