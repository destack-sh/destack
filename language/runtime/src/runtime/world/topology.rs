use std::borrow::Borrow;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::constants::{
    BUILTIN_AGENT_KIND_ID, BUILTIN_AGENT_OWNS_RESOURCE_EDGE_KIND_ID, BUILTIN_RUNTIME_KIND_ID,
    BUILTIN_RUNTIME_OWNS_AGENT_EDGE_KIND_ID, INITIAL_AGENT_ID, INITIAL_CONTROL_REVISION,
    INITIAL_RUNTIME_ID, LABEL_AGENT_NAME, LABEL_RUNTIME_NAME, LABEL_TOPOLOGY_KIND,
};
use crate::platform::{ResourceId, ResourceKind};
use crate::runtime::AgentId;

/// Stable identifier for one runtime instance in one world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RuntimeId(pub u64);

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

/// World topology registry for runtime and simulation identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Topology {
    /// Next runtime id to allocate.
    next_runtime_id: u64,
    /// Next agent id to allocate.
    next_agent_id: u64,
    /// Command revision for world control changes.
    control_revision: u64,
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
            next_runtime_id: INITIAL_RUNTIME_ID,
            next_agent_id: INITIAL_AGENT_ID,
            control_revision: INITIAL_CONTROL_REVISION,
            entity_kinds: BTreeMap::new(),
            edge_kinds: BTreeMap::new(),
            entities: BTreeMap::new(),
            edges: BTreeMap::new(),
        }
    }
}

impl Topology {
    /// Create one topology with all builtin kind registrations installed.
    pub(crate) fn with_builtin_kinds() -> Result<Self, String> {
        let mut topology = Self::default();
        topology.seed_builtin_kinds()?;

        Ok(topology)
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

    /// Return the current control revision.
    pub(crate) const fn control_revision(&self) -> u64 {
        self.control_revision
    }

    /// Increment and return the current control revision.
    pub(crate) fn bump_control_revision(&mut self) -> u64 {
        self.control_revision = self.control_revision.saturating_add(1);
        self.control_revision
    }

    /// Allocate and return one runtime id.
    pub(crate) fn allocate_runtime_id(&mut self) -> RuntimeId {
        let runtime_id = RuntimeId(self.next_runtime_id);
        self.next_runtime_id = self.next_runtime_id.saturating_add(1);
        runtime_id
    }

    /// Allocate and return one agent id.
    pub(crate) fn allocate_agent_id(&mut self) -> AgentId {
        let agent_id = AgentId(self.next_agent_id);
        self.next_agent_id = self.next_agent_id.saturating_add(1);
        agent_id
    }

    /// Return all topology entities.
    pub(crate) fn entities(&self) -> &BTreeMap<WorldEntityId, WorldEntity> {
        &self.entities
    }

    /// Return all topology edges.
    pub(crate) fn edges(&self) -> &BTreeMap<WorldEdgeId, WorldEdge> {
        &self.edges
    }

    /// Return one runtime name and labels by runtime id.
    pub(crate) fn runtime_identity(
        &self,
        runtime_id: RuntimeId,
    ) -> Option<(&str, &BTreeMap<String, String>)> {
        let entity_id = Self::runtime_entity_id(runtime_id);
        let entity = self.entities.get(entity_id.as_str())?;
        let runtime_name = entity.labels.get(LABEL_RUNTIME_NAME)?.as_str();
        let runtime_labels = &entity.labels;

        Some((runtime_name, runtime_labels))
    }

    /// Return one agent name and labels by agent id.
    pub(crate) fn agent_identity(
        &self,
        agent_id: AgentId,
    ) -> Option<(&str, &BTreeMap<String, String>)> {
        let entity_id = Self::agent_entity_id(agent_id);
        let entity = self.entities.get(entity_id.as_str())?;
        let agent_name = entity.labels.get(LABEL_AGENT_NAME)?.as_str();
        let agent_labels = &entity.labels;

        Some((agent_name, agent_labels))
    }

    /// Define one entity kind in topology.
    pub(crate) fn define_entity_kind(
        &mut self,
        mut kind: WorldEntityKindDefinition,
    ) -> Result<(), String> {
        // reject empty kind identifiers
        if kind.kind.as_str().is_empty() {
            return Err("topology kind id must not be empty".to_string());
        }

        // reject duplicate kind identifiers
        if self.entity_kinds.contains_key(kind.kind.as_str()) {
            return Err(format!("topology kind {} is already defined", kind.kind.0));
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
    ) -> Result<(), String> {
        // reject empty kind identifiers
        if kind.kind.as_str().is_empty() {
            return Err("topology kind id must not be empty".to_string());
        }

        // reject duplicate kind identifiers
        if self.edge_kinds.contains_key(kind.kind.as_str()) {
            return Err(format!("topology kind {} is already defined", kind.kind.0));
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
    pub(crate) fn upsert_entity(&mut self, entity: WorldEntity) -> Result<(), String> {
        // reject unknown kinds before mutating entity state
        self.ensure_entity_kind_defined(entity.kind.as_str())?;

        // insert or replace one entity record
        self.entities.insert(entity.id.clone(), entity);

        Ok(())
    }

    /// Remove one topology entity and all incident edges.
    pub(crate) fn remove_entity(&mut self, entity_id: &str) -> bool {
        // remove all incident edges first
        let removed_edge_count = self.remove_incident_edges(entity_id);

        // remove the entity itself
        let is_entity_removed = self.entities.remove(entity_id).is_some();
        is_entity_removed || removed_edge_count > 0
    }

    /// Upsert one topology edge.
    pub(crate) fn upsert_edge(&mut self, edge: WorldEdge) -> Result<(), String> {
        // reject unknown edge kinds
        self.ensure_edge_kind_defined(edge.kind.as_str())?;

        // reject missing source entities
        self.ensure_entity_exists(edge.from.as_str(), "source")?;

        // reject missing destination entities
        self.ensure_entity_exists(edge.to.as_str(), "destination")?;

        // insert or replace one edge record
        self.edges.insert(edge.id.clone(), edge);

        Ok(())
    }

    /// Remove one topology edge.
    pub(crate) fn remove_edge(&mut self, edge_id: &str) -> bool {
        let is_edge_removed = self.edges.remove(edge_id).is_some();
        is_edge_removed
    }

    /// Register one runtime node.
    pub(crate) fn register_runtime(
        &mut self,
        runtime_id: RuntimeId,
        name: String,
        labels: BTreeMap<String, String>,
    ) -> Result<(), String> {
        let runtime_entity_id = Self::runtime_entity_id(runtime_id);

        // reject duplicate runtime identifiers
        if self.entities.contains_key(runtime_entity_id.as_str()) {
            return Err(format!(
                "topology runtime {} is already registered",
                runtime_id.0
            ));
        }

        // mirror runtime identity into the topology entity graph
        let runtime_labels = Self::runtime_labels(runtime_id, name, labels);
        self.entities.insert(
            runtime_entity_id.clone(),
            WorldEntity {
                id: runtime_entity_id,
                kind: WorldEntityKind::from(BUILTIN_RUNTIME_KIND_ID),
                labels: runtime_labels,
            },
        );

        Ok(())
    }

    /// Register one agent node under one runtime.
    pub(crate) fn register_agent(
        &mut self,
        runtime_id: RuntimeId,
        agent_id: AgentId,
        name: String,
        labels: BTreeMap<String, String>,
    ) -> Result<(), String> {
        let runtime_entity_id = Self::runtime_entity_id(runtime_id);
        let agent_entity_id = Self::agent_entity_id(agent_id);

        // reject unknown runtimes
        if !self.entity_is_kind(runtime_entity_id.as_str(), BUILTIN_RUNTIME_KIND_ID) {
            return Err(format!("topology runtime {} does not exist", runtime_id.0));
        }

        // reject duplicate agent identifiers
        if self.entities.contains_key(agent_entity_id.as_str()) {
            return Err(format!(
                "topology agent {} is already registered",
                agent_id.0
            ));
        }

        // upsert one agent entity
        let agent_labels = Self::agent_labels(runtime_id, agent_id, name, labels);
        self.entities.insert(
            agent_entity_id.clone(),
            WorldEntity {
                id: agent_entity_id.clone(),
                kind: WorldEntityKind::from(BUILTIN_AGENT_KIND_ID),
                labels: agent_labels,
            },
        );

        // upsert one runtime ownership edge
        let runtime_agent_edge_id = Self::runtime_agent_edge_id(runtime_id, agent_id);
        self.edges.insert(
            runtime_agent_edge_id.clone(),
            WorldEdge {
                id: runtime_agent_edge_id,
                kind: WorldEdgeKind::from(BUILTIN_RUNTIME_OWNS_AGENT_EDGE_KIND_ID),
                from: runtime_entity_id,
                to: agent_entity_id,
                labels: BTreeMap::new(),
            },
        );

        Ok(())
    }

    /// Deregister one agent and all agent-owned topology state.
    pub(crate) fn deregister_agent(&mut self, agent_id: AgentId) -> bool {
        let agent_entity_id = Self::agent_entity_id(agent_id);

        // reject unknown agents
        if !self.entity_is_kind(agent_entity_id.as_str(), BUILTIN_AGENT_KIND_ID) {
            return false;
        }

        // capture owner runtimes before removing runtime ownership edges
        let runtime_entity_ids = self.runtime_owners_for_agent(agent_entity_id.as_str());

        // remove agent-owned resource entities and edges
        self.remove_agent_resource_attachments(agent_entity_id.as_str());

        // remove all remaining agent incident edges and the agent entity
        self.remove_incident_edges(agent_entity_id.as_str());
        self.entities.remove(agent_entity_id.as_str());

        // remove orphaned runtimes
        for runtime_entity_id in runtime_entity_ids {
            self.remove_runtime_if_orphaned(runtime_entity_id.as_str());
        }

        true
    }

    /// Create one resource under one agent in the topology entity graph.
    pub(crate) fn create_resource_for_agent(
        &mut self,
        agent_id: AgentId,
        resource_id: ResourceId,
        resource_kind: &str,
        resource_label: Option<&str>,
    ) -> Result<(), String> {
        let agent_entity_id = Self::agent_entity_id(agent_id);

        // reject unknown agents
        if !self.entity_is_kind(agent_entity_id.as_str(), BUILTIN_AGENT_KIND_ID) {
            return Err(format!("topology agent {} does not exist", agent_id.0));
        }

        // reject unknown resource kinds
        self.ensure_entity_kind_defined(resource_kind)?;

        // upsert one resource entity
        let resource_entity_id = Self::resource_entity_id(agent_id, resource_id);
        let resource_labels = Self::resource_labels(agent_id, resource_id, resource_label);
        self.entities.insert(
            resource_entity_id.clone(),
            WorldEntity {
                id: resource_entity_id.clone(),
                kind: WorldEntityKind::from(resource_kind),
                labels: resource_labels,
            },
        );

        // upsert one agent ownership edge
        let resource_edge_id = Self::agent_resource_edge_id(agent_id, resource_id);
        self.edges.insert(
            resource_edge_id.clone(),
            WorldEdge {
                id: resource_edge_id,
                kind: WorldEdgeKind::from(BUILTIN_AGENT_OWNS_RESOURCE_EDGE_KIND_ID),
                from: agent_entity_id,
                to: resource_entity_id,
                labels: BTreeMap::new(),
            },
        );

        Ok(())
    }

    /// Destroy one resource under one agent in the topology entity graph.
    pub(crate) fn destroy_resource_for_agent(
        &mut self,
        agent_id: AgentId,
        resource_id: ResourceId,
    ) -> bool {
        let resource_entity_id = Self::resource_entity_id(agent_id, resource_id);
        let resource_edge_id = Self::agent_resource_edge_id(agent_id, resource_id);

        let is_edge_removed = self.edges.remove(resource_edge_id.as_str()).is_some();
        let is_entity_removed = self.entities.remove(resource_entity_id.as_str()).is_some();
        is_edge_removed || is_entity_removed
    }

    /// Seed all builtin entity and edge kinds.
    fn seed_builtin_kinds(&mut self) -> Result<(), String> {
        // seed builtin entity kinds
        for &kind_id in builtin_entity_kind_definitions() {
            self.define_entity_kind(WorldEntityKindDefinition {
                kind: WorldEntityKind::from(kind_id),
                labels: labels_for_kind(kind_id),
                supported_faults: supported_entity_faults_for_builtin_kind(kind_id),
            })?;
        }

        // seed builtin resource entity kinds
        for resource_kind in ResourceKind::all() {
            self.define_entity_kind(WorldEntityKindDefinition {
                kind: WorldEntityKind::from(resource_kind.kind_id()),
                labels: labels_for_kind(resource_kind.kind_id()),
                supported_faults: base_supported_entity_faults(),
            })?;
        }

        // seed builtin edge kinds
        for &kind_id in builtin_edge_kind_definitions() {
            self.define_edge_kind(WorldEdgeKindDefinition {
                kind: WorldEdgeKind::from(kind_id),
                labels: labels_for_kind(kind_id),
                supported_faults: supported_edge_faults_for_builtin_kind(kind_id),
            })?;
        }

        Ok(())
    }

    /// Return one stable runtime entity identifier.
    fn runtime_entity_id(runtime_id: RuntimeId) -> WorldEntityId {
        WorldEntityId(format!("runtime.{}", runtime_id.0))
    }

    /// Return one stable agent entity identifier.
    fn agent_entity_id(agent_id: AgentId) -> WorldEntityId {
        WorldEntityId(format!("agent.{}", agent_id.0))
    }

    /// Return one stable resource entity identifier.
    fn resource_entity_id(agent_id: AgentId, resource_id: ResourceId) -> WorldEntityId {
        WorldEntityId(format!("resource.{}.{}", agent_id.0, resource_id.0))
    }

    /// Return one stable runtime-to-agent edge identifier.
    fn runtime_agent_edge_id(runtime_id: RuntimeId, agent_id: AgentId) -> WorldEdgeId {
        WorldEdgeId(format!(
            "runtime.owns.agent.{}.{}",
            runtime_id.0, agent_id.0
        ))
    }

    /// Return one stable agent-to-resource edge identifier.
    fn agent_resource_edge_id(agent_id: AgentId, resource_id: ResourceId) -> WorldEdgeId {
        WorldEdgeId(format!(
            "agent.owns.resource.{}.{}",
            agent_id.0, resource_id.0
        ))
    }

    /// Return runtime labels mirrored into topology entities.
    fn runtime_labels(
        runtime_id: RuntimeId,
        name: String,
        mut labels: BTreeMap<String, String>,
    ) -> BTreeMap<String, String> {
        labels.insert("runtime.id".to_string(), runtime_id.0.to_string());
        labels.insert(LABEL_RUNTIME_NAME.to_string(), name);
        labels
    }

    /// Return agent labels mirrored into topology entities.
    fn agent_labels(
        runtime_id: RuntimeId,
        agent_id: AgentId,
        name: String,
        mut labels: BTreeMap<String, String>,
    ) -> BTreeMap<String, String> {
        labels.insert("runtime.id".to_string(), runtime_id.0.to_string());
        labels.insert("agent.id".to_string(), agent_id.0.to_string());
        labels.insert(LABEL_AGENT_NAME.to_string(), name);
        labels
    }

    /// Return resource labels mirrored into topology entities.
    fn resource_labels(
        agent_id: AgentId,
        resource_id: ResourceId,
        resource_label: Option<&str>,
    ) -> BTreeMap<String, String> {
        let mut labels = BTreeMap::new();
        labels.insert("agent.id".to_string(), agent_id.0.to_string());
        labels.insert("resource.id".to_string(), resource_id.0.to_string());
        if let Some(resource_label) = resource_label {
            labels.insert("resource.label".to_string(), resource_label.to_string());
        }
        labels
    }

    /// Return true when one entity exists with the expected kind.
    fn entity_is_kind(&self, entity_id: &str, kind_id: &str) -> bool {
        let Some(entity) = self.entities.get(entity_id) else {
            return false;
        };

        entity.kind.as_str() == kind_id
    }

    /// Ensure one entity kind is defined.
    fn ensure_entity_kind_defined(&self, kind_id: &str) -> Result<(), String> {
        if self.entity_kinds.contains_key(kind_id) {
            return Ok(());
        }

        Err(format!("topology entity kind {kind_id} is not defined"))
    }

    /// Ensure one edge kind is defined.
    fn ensure_edge_kind_defined(&self, kind_id: &str) -> Result<(), String> {
        if self.edge_kinds.contains_key(kind_id) {
            return Ok(());
        }

        Err(format!("topology edge kind {kind_id} is not defined"))
    }

    /// Ensure one entity exists.
    fn ensure_entity_exists(&self, entity_id: &str, role: &str) -> Result<(), String> {
        if self.entities.contains_key(entity_id) {
            return Ok(());
        }

        Err(format!("topology {role} entity {entity_id} does not exist"))
    }

    /// Remove all incident edges for one entity and return the number removed.
    fn remove_incident_edges(&mut self, entity_id: &str) -> usize {
        let before_edge_count = self.edges.len();
        self.edges
            .retain(|_, edge| edge.from.as_str() != entity_id && edge.to.as_str() != entity_id);

        before_edge_count.saturating_sub(self.edges.len())
    }

    /// Return runtime entity ids that own one agent entity.
    fn runtime_owners_for_agent(&self, agent_entity_id: &str) -> BTreeSet<WorldEntityId> {
        self.edges
            .values()
            .filter(|edge| {
                edge.kind.as_str() == BUILTIN_RUNTIME_OWNS_AGENT_EDGE_KIND_ID
                    && edge.to.as_str() == agent_entity_id
            })
            .map(|edge| edge.from.clone())
            .collect()
    }

    /// Remove all resource attachments owned by one agent entity.
    fn remove_agent_resource_attachments(&mut self, agent_entity_id: &str) {
        let resource_entity_ids = self
            .edges
            .values()
            .filter(|edge| {
                edge.kind.as_str() == BUILTIN_AGENT_OWNS_RESOURCE_EDGE_KIND_ID
                    && edge.from.as_str() == agent_entity_id
            })
            .map(|edge| edge.to.clone())
            .collect::<Vec<_>>();

        self.edges.retain(|_, edge| {
            !(edge.kind.as_str() == BUILTIN_AGENT_OWNS_RESOURCE_EDGE_KIND_ID
                && edge.from.as_str() == agent_entity_id)
        });

        for resource_entity_id in resource_entity_ids {
            self.entities.remove(resource_entity_id.as_str());
        }
    }

    /// Remove one runtime and all runtime incident edges when it has no attached agents.
    fn remove_runtime_if_orphaned(&mut self, runtime_entity_id: &str) -> bool {
        if !self.entity_is_kind(runtime_entity_id, BUILTIN_RUNTIME_KIND_ID) {
            return false;
        }

        let has_agents = self.edges.values().any(|edge| {
            edge.kind.as_str() == BUILTIN_RUNTIME_OWNS_AGENT_EDGE_KIND_ID
                && edge.from.as_str() == runtime_entity_id
        });
        if has_agents {
            return false;
        }

        self.remove_incident_edges(runtime_entity_id);
        self.entities.remove(runtime_entity_id);
        true
    }
}

/// Return builtin entity kind identifiers.
fn builtin_entity_kind_definitions() -> &'static [&'static str] {
    &[
        "runtime.instance",
        "runtime.agent",
        "process.instance",
        "time.timer",
        "thread.instance",
        "io.stream",
        "fs.inode",
        "fs.dentry",
        "fs.open_file",
        "fs.mount",
        "fs.watch",
        "net.namespace",
        "net.interface",
        "net.socket",
        "net.listener",
        "net.connection",
        "net.resolver",
        "process.child",
        "audio.device",
        "audio.stream",
        "input.device",
        "gpu.device",
        "gpu.queue",
        "ipc.channel",
        "device.handle",
        "display.surface",
        "memory.region",
        "thread.worker",
        "time.clock",
        "tls.session",
        "security.policy",
        "os.service",
        "random.stream",
        "resource.handle",
        "tty.device",
        "ffi.handle",
        "crypto.key_store",
        "error.channel",
        "debug.channel",
    ]
}

/// Return builtin edge kind identifiers.
fn builtin_edge_kind_definitions() -> &'static [&'static str] {
    &[
        "fs.parent_child",
        "fs.fd_binding",
        "fs.mount_attachment",
        "net.network_link",
        "net.stream_link",
        "net.route",
        "runtime.instance.owns.agent",
        "runtime.agent.owns.resource",
        "ipc.channel",
        "process.pipe",
    ]
}

/// Return base fault verbs supported by all entity kinds.
fn base_supported_entity_faults() -> BTreeSet<String> {
    [
        "call.error",
        "call.timeout",
        "timing.delay",
        "call.block",
        "scheduler.starve",
        "resource.exhaust",
        "resource.quota",
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect()
}

/// Return base fault verbs supported by all edge kinds.
fn base_supported_edge_faults() -> BTreeSet<String> {
    base_supported_entity_faults()
}

/// Extend one supported fault set with transport verbs.
fn extend_transport_faults(supported_faults: &mut BTreeSet<String>) {
    for verb_id in [
        "transport.drop",
        "transport.duplicate",
        "transport.reorder",
        "transport.corrupt",
        "transport.truncate",
        "transport.partial",
        "transport.disconnect",
        "transport.reset",
        "transport.partition",
        "transport.blackhole",
        "transport.throttle",
        "transport.limit",
    ] {
        supported_faults.insert(verb_id.to_string());
    }
}

/// Extend one supported fault set with lifecycle verbs.
fn extend_lifecycle_faults(supported_faults: &mut BTreeSet<String>) {
    for verb_id in ["process.crash", "process.restart", "process.reboot"] {
        supported_faults.insert(verb_id.to_string());
    }
}

/// Extend one supported fault set with clock verbs.
fn extend_clock_faults(supported_faults: &mut BTreeSet<String>) {
    for verb_id in ["clock.jump", "clock.drift", "clock.freeze"] {
        supported_faults.insert(verb_id.to_string());
    }
}

/// Extend one supported fault set with durability verbs.
fn extend_durability_faults(supported_faults: &mut BTreeSet<String>) {
    supported_faults.insert("durability.violate".to_string());
}

/// Return supported fault verbs for one builtin entity kind.
fn supported_entity_faults_for_builtin_kind(kind_id: &str) -> BTreeSet<String> {
    let mut supported_faults = base_supported_entity_faults();

    if matches!(
        kind_id,
        "io.stream"
            | "net.namespace"
            | "net.interface"
            | "net.socket"
            | "net.listener"
            | "net.connection"
            | "net.resolver"
            | "audio.stream"
            | "ipc.channel"
            | "tls.session"
            | "tty.device"
    ) {
        extend_transport_faults(&mut supported_faults);
    }

    if matches!(
        kind_id,
        "process.instance" | "thread.instance" | "process.child" | "thread.worker"
    ) {
        extend_lifecycle_faults(&mut supported_faults);
    }

    if matches!(kind_id, "runtime.instance" | "time.timer" | "time.clock") {
        extend_clock_faults(&mut supported_faults);
    }

    if matches!(
        kind_id,
        "fs.inode" | "fs.dentry" | "fs.open_file" | "fs.mount" | "fs.watch" | "crypto.key_store"
    ) {
        extend_durability_faults(&mut supported_faults);
    }

    supported_faults
}

/// Return supported fault verbs for one builtin edge kind.
fn supported_edge_faults_for_builtin_kind(kind_id: &str) -> BTreeSet<String> {
    let mut supported_faults = base_supported_edge_faults();

    if matches!(
        kind_id,
        "net.network_link" | "net.stream_link" | "net.route" | "ipc.channel" | "process.pipe"
    ) {
        extend_transport_faults(&mut supported_faults);
    }

    supported_faults
}

/// Build system labels for one kind id and capability list.
fn labels_for_kind(kind_id: &str) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    labels.insert(LABEL_TOPOLOGY_KIND.to_string(), kind_id.to_string());

    labels
}
