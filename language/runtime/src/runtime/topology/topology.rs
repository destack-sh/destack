use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::runtime::world::{
    BUILTIN_RUNTIME_KIND_ID, BUILTIN_RUNTIME_OWNS_WORKER_EDGE_KIND_ID, BUILTIN_WORKER_KIND_ID,
    BUILTIN_WORKER_OWNS_RESOURCE_EDGE_KIND_ID, LABEL_RESOURCE_LABEL, LABEL_RUNTIME_NAME,
    LABEL_WORKER_NAME,
};

use super::builtin::{
    base_supported_edge_faults, base_supported_entity_faults, builtin_edge_kinds,
    builtin_entity_kinds, builtin_resource_entity_kinds,
};
use super::{
    Edge, EdgeDefinition, EdgeId, EdgeKind, Entity, EntityDefinition, EntityId, EntityKind,
    EntityRole, RuntimeId, TopologyError, TopologyResult,
};
use crate::platform::ResourceId;
use crate::runtime::WorkerId;
use crate::runtime::world::{resource_entity_id, resource_ownership_edge_id};

/// World topology graph and kind catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Topology {
    /// Registered entity kinds by kind id.
    entity_kinds: BTreeMap<EntityKind, EntityDefinition>,
    /// Registered edge kinds by kind id.
    edge_kinds: BTreeMap<EdgeKind, EdgeDefinition>,
    /// Topology entities by entity identifier.
    entities: BTreeMap<EntityId, Entity>,
    /// Topology edges by edge identifier.
    edges: BTreeMap<EdgeId, Edge>,
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
    pub(crate) fn entity_kinds(&self) -> &BTreeMap<EntityKind, EntityDefinition> {
        &self.entity_kinds
    }

    /// Return all registered edge kinds.
    pub(crate) fn edge_kinds(&self) -> &BTreeMap<EdgeKind, EdgeDefinition> {
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
    pub(crate) fn entities(&self) -> &BTreeMap<EntityId, Entity> {
        &self.entities
    }

    /// Return all topology edges.
    pub(crate) fn edges(&self) -> &BTreeMap<EdgeId, Edge> {
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

    /// Return the rule subject metadata for one worker.
    pub(crate) fn worker_subject(
        &self,
        worker_id: WorkerId,
    ) -> Option<(&str, &BTreeMap<String, String>)> {
        let entity = self.entities.get(&worker_id.entity_id())?;
        let name = entity.labels.get(LABEL_WORKER_NAME)?;

        Some((name.as_str(), &entity.labels))
    }

    /// Return whether one runtime owns one worker.
    pub(crate) fn runtime_owns_worker(&self, runtime_id: RuntimeId, worker_id: WorkerId) -> bool {
        self.edges
            .contains_key(&runtime_id.owns_worker_edge_id(worker_id))
    }

    /// Add one runtime and its default worker metadata.
    pub(crate) fn add_runtime(
        &mut self,
        runtime_id: RuntimeId,
        runtime_name: String,
        runtime_labels: BTreeMap<String, String>,
        default_worker_id: WorkerId,
        default_worker_name: String,
        default_worker_labels: BTreeMap<String, String>,
    ) -> TopologyResult<()> {
        // reject duplicate metadata upfront
        if self.entities.contains_key(&runtime_id.entity_id()) {
            return Err(TopologyError::DuplicateEntity {
                entity_id: runtime_id.entity_id(),
            });
        }
        if self.entities.contains_key(&default_worker_id.entity_id()) {
            return Err(TopologyError::DuplicateEntity {
                entity_id: default_worker_id.entity_id(),
            });
        }

        // runtime entity
        let runtime_entity = Entity::new(runtime_id.entity_id(), BUILTIN_RUNTIME_KIND_ID).labels(
            entity_labels_with_name(runtime_labels, LABEL_RUNTIME_NAME, runtime_name),
        );
        self.upsert_entity(runtime_entity)?;

        // default worker entity
        let worker_entity = Entity::new(default_worker_id.entity_id(), BUILTIN_WORKER_KIND_ID)
            .labels(entity_labels_with_name(
                default_worker_labels,
                LABEL_WORKER_NAME,
                default_worker_name,
            ));
        self.upsert_entity(worker_entity)?;

        // ownership edge
        let edge = Edge::new(
            runtime_id.owns_worker_edge_id(default_worker_id),
            BUILTIN_RUNTIME_OWNS_WORKER_EDGE_KIND_ID,
            runtime_id.entity_id(),
            default_worker_id.entity_id(),
        );
        self.upsert_edge(edge)?;

        Ok(())
    }

    /// Add one additional worker metadata record.
    pub(crate) fn add_worker(
        &mut self,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        worker_name: String,
        worker_labels: BTreeMap<String, String>,
    ) -> TopologyResult<()> {
        // reject missing owning runtime
        if !self.entities.contains_key(&runtime_id.entity_id()) {
            return Err(TopologyError::UnknownEntity {
                entity_id: runtime_id.entity_id(),
                role: EntityRole::Source,
            });
        }

        // reject duplicate worker metadata
        if self.entities.contains_key(&worker_id.entity_id()) {
            return Err(TopologyError::DuplicateEntity {
                entity_id: worker_id.entity_id(),
            });
        }

        // worker entity
        let worker_entity = Entity::new(worker_id.entity_id(), BUILTIN_WORKER_KIND_ID).labels(
            entity_labels_with_name(worker_labels, LABEL_WORKER_NAME, worker_name),
        );
        self.upsert_entity(worker_entity)?;

        // ownership edge
        let edge = Edge::new(
            runtime_id.owns_worker_edge_id(worker_id),
            BUILTIN_RUNTIME_OWNS_WORKER_EDGE_KIND_ID,
            runtime_id.entity_id(),
            worker_id.entity_id(),
        );
        self.upsert_edge(edge)?;

        Ok(())
    }

    /// Remove one worker metadata record and any now-orphaned runtime/resource metadata.
    pub(crate) fn remove_worker(&mut self, worker_id: WorkerId) -> bool {
        let worker_entity_id = worker_id.entity_id();
        let Some(runtime_entity_id) = self.worker_runtime_entity_id(worker_id) else {
            return false;
        };

        // resource metadata owned by the worker
        let owned_resource_ids = self
            .edges
            .values()
            .filter(|edge| {
                edge.kind.as_str() == BUILTIN_WORKER_OWNS_RESOURCE_EDGE_KIND_ID
                    && edge.from == worker_entity_id
            })
            .map(|edge| edge.to.clone())
            .collect::<Vec<_>>();
        for resource_entity_id in owned_resource_ids {
            self.remove_entity(resource_entity_id.as_str());
        }

        // worker metadata
        let is_worker_removed = self.remove_entity(worker_entity_id.as_str());

        // remove the runtime metadata when the last worker disappears
        if !self.runtime_has_workers(runtime_entity_id.as_str()) {
            self.remove_entity(runtime_entity_id.as_str());
        }

        is_worker_removed
    }

    /// Attach one resource metadata record to one worker.
    pub(crate) fn attach_resource(
        &mut self,
        resource_id: ResourceId,
        resource_kind: EntityKind,
        resource_label: Option<&str>,
    ) -> TopologyResult<()> {
        // reject missing owning worker
        if !self
            .entities
            .contains_key(&resource_id.worker_id.entity_id())
        {
            return Err(TopologyError::UnknownEntity {
                entity_id: resource_id.worker_id.entity_id(),
                role: EntityRole::Source,
            });
        }

        // resource entity
        let mut labels = BTreeMap::new();
        if let Some(resource_label) = resource_label {
            labels.insert(LABEL_RESOURCE_LABEL.to_string(), resource_label.to_string());
        }
        let resource_entity =
            Entity::new(resource_entity_id(resource_id), resource_kind).labels(labels);
        self.upsert_entity(resource_entity)?;

        // ownership edge
        let edge = Edge::new(
            resource_ownership_edge_id(resource_id),
            BUILTIN_WORKER_OWNS_RESOURCE_EDGE_KIND_ID,
            resource_id.worker_id.entity_id(),
            resource_entity_id(resource_id),
        );
        self.upsert_edge(edge)?;

        Ok(())
    }

    /// Detach one resource metadata record from one worker.
    pub(crate) fn detach_resource(&mut self, resource_id: ResourceId) -> bool {
        self.remove_entity(resource_entity_id(resource_id).as_str())
    }

    /// Define one entity kind in topology.
    pub(crate) fn define_entity_kind(&mut self, mut kind: EntityDefinition) -> TopologyResult<()> {
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
    pub(crate) fn define_edge_kind(&mut self, mut kind: EdgeDefinition) -> TopologyResult<()> {
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
    pub(crate) fn upsert_entity(&mut self, entity: Entity) -> TopologyResult<()> {
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
    pub(crate) fn upsert_edge(&mut self, edge: Edge) -> TopologyResult<()> {
        self.expect_edge_kind(edge.kind.as_str())?;
        self.expect_entity(edge.from.as_str(), EntityRole::Source)?;
        self.expect_entity(edge.to.as_str(), EntityRole::Destination)?;
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
            kind: EntityKind::from(kind_id),
        })
    }

    /// Expect one edge kind to be defined.
    fn expect_edge_kind(&self, kind_id: &str) -> TopologyResult<()> {
        if self.edge_kinds.contains_key(kind_id) {
            return Ok(());
        }

        Err(TopologyError::UnknownEdgeKind {
            kind: EdgeKind::from(kind_id),
        })
    }

    /// Expect one entity to exist.
    fn expect_entity(&self, entity_id: &str, role: EntityRole) -> TopologyResult<()> {
        if self.entities.contains_key(entity_id) {
            return Ok(());
        }

        Err(TopologyError::UnknownEntity {
            entity_id: EntityId::from(entity_id),
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

    /// Return the owning runtime id for one worker entity.
    fn worker_runtime_entity_id(&self, worker_id: WorkerId) -> Option<EntityId> {
        let worker_entity_id = worker_id.entity_id();
        let edge = self.edges.values().find(|edge| {
            edge.kind.as_str() == BUILTIN_RUNTIME_OWNS_WORKER_EDGE_KIND_ID
                && edge.to == worker_entity_id
        })?;

        Some(edge.from.clone())
    }

    /// Return whether one runtime still owns any worker metadata.
    fn runtime_has_workers(&self, runtime_entity_id: &str) -> bool {
        self.edges.values().any(|edge| {
            edge.kind.as_str() == BUILTIN_RUNTIME_OWNS_WORKER_EDGE_KIND_ID
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
