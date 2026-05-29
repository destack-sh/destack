use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::builtin::{
    BUILTIN_EDGE_KINDS, BUILTIN_ENTITY_KINDS, is_builtin_edge_kind, is_builtin_entity_kind,
};
use super::{
    Edge, EdgeDefinition, EdgeId, EdgeKind, Entity, EntityDefinition, EntityId, EntityKind,
    EntityRole, RuntimeId, TopologyError, TopologyResult,
};
use crate::runtime::WorkerId;
use crate::world::Resource;
use crate::world::scenario::{base_edge_faults, base_entity_faults};

/// World topology graph and kind catalog.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Topology {
    /// User-defined entity kinds by kind id.
    entity_kinds: BTreeMap<EntityKind, EntityDefinition>,
    /// User-defined edge kinds by kind id.
    edge_kinds: BTreeMap<EdgeKind, EdgeDefinition>,
    /// Topology entities by entity identifier.
    entities: BTreeMap<EntityId, Entity>,
    /// Topology edges by edge identifier.
    edges: BTreeMap<EdgeId, Edge>,
}

impl Topology {
    /// Create one topology with builtin kinds.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Return all registered entity kinds.
    pub(crate) fn entity_kinds(&self) -> BTreeMap<EntityKind, EntityDefinition> {
        let mut kinds = BUILTIN_ENTITY_KINDS.clone();
        kinds.extend(self.entity_kinds.clone());

        kinds
    }

    /// Return all registered edge kinds.
    pub(crate) fn edge_kinds(&self) -> BTreeMap<EdgeKind, EdgeDefinition> {
        let mut kinds = BUILTIN_EDGE_KINDS.clone();
        kinds.extend(self.edge_kinds.clone());

        kinds
    }

    /// Return one registered entity kind by kind id.
    pub(crate) fn entity_kind(&self, kind: &str) -> Option<&EntityDefinition> {
        self.entity_kinds
            .get(kind)
            .or_else(|| BUILTIN_ENTITY_KINDS.get(kind))
    }

    /// Return one registered edge kind by kind id.
    pub(crate) fn edge_kind(&self, kind: &str) -> Option<&EdgeDefinition> {
        self.edge_kinds
            .get(kind)
            .or_else(|| BUILTIN_EDGE_KINDS.get(kind))
    }

    /// Return one entity kind supported fault set by kind id.
    pub(crate) fn entity_kind_supported_faults(&self, kind: &str) -> Option<&BTreeSet<String>> {
        self.entity_kind(kind).map(|kind| &kind.supported_faults)
    }

    /// Return one edge kind supported fault set by kind id.
    pub(crate) fn edge_kind_supported_faults(&self, kind: &str) -> Option<&BTreeSet<String>> {
        self.edge_kind(kind).map(|kind| &kind.supported_faults)
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

        Some((entity.name.as_str(), &entity.labels))
    }

    /// Return the rule subject metadata for one worker.
    pub(crate) fn worker_subject(
        &self,
        worker_id: WorkerId,
    ) -> Option<(&str, &BTreeMap<String, String>)> {
        let entity = self.entities.get(&worker_id.entity_id())?;

        Some((entity.name.as_str(), &entity.labels))
    }

    /// Return whether one runtime owns one worker.
    pub(crate) fn runtime_owns_worker(&self, runtime_id: RuntimeId, worker_id: WorkerId) -> bool {
        self.edges
            .contains_key(&runtime_id.owns_worker_edge_id(worker_id))
    }

    /// Add one runtime metadata record.
    pub(crate) fn add_runtime(
        &mut self,
        runtime_id: RuntimeId,
        runtime_entity: Entity,
    ) -> TopologyResult<()> {
        let runtime_kind = EntityKind::from(EntityKind::RUNTIME);

        // reject mismatched runtime metadata
        if runtime_entity.id != runtime_id.entity_id() {
            return Err(TopologyError::EntityIdMismatch {
                expected: runtime_id.entity_id(),
                actual: runtime_entity.id,
            });
        }

        // reject wrong runtime kind
        if runtime_entity.kind != runtime_kind {
            return Err(TopologyError::EntityKindMismatch {
                entity_id: runtime_entity.id,
                expected: runtime_kind,
                actual: runtime_entity.kind,
            });
        }

        // reject duplicate metadata upfront
        if self.entities.contains_key(&runtime_id.entity_id()) {
            return Err(TopologyError::DuplicateEntity {
                entity_id: runtime_id.entity_id(),
            });
        }

        // runtime entity
        self.upsert_entity(runtime_entity)?;

        Ok(())
    }

    /// Add one additional worker metadata record.
    pub(crate) fn add_worker(
        &mut self,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        worker_entity: Entity,
    ) -> TopologyResult<()> {
        let worker_kind = EntityKind::from(EntityKind::WORKER);

        // reject mismatched worker metadata
        if worker_entity.id != worker_id.entity_id() {
            return Err(TopologyError::EntityIdMismatch {
                expected: worker_id.entity_id(),
                actual: worker_entity.id,
            });
        }

        // reject wrong worker kind
        if worker_entity.kind != worker_kind {
            return Err(TopologyError::EntityKindMismatch {
                entity_id: worker_entity.id,
                expected: worker_kind,
                actual: worker_entity.kind,
            });
        }

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
        self.upsert_entity(worker_entity)?;

        // ownership edge
        let edge = Edge::new(
            runtime_id.owns_worker_edge_id(worker_id),
            EdgeKind::RUNTIME_OWNS_WORKER,
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
                edge.kind.as_str() == EdgeKind::WORKER_OWNS_RESOURCE
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

    /// Attach one resource entity to one worker.
    pub(crate) fn attach_resource(
        &mut self,
        resource: &Resource,
        entity: Entity,
    ) -> TopologyResult<()> {
        // reject mismatched resource metadata
        if entity.id != resource.entity_id() {
            return Err(TopologyError::EntityIdMismatch {
                expected: resource.entity_id(),
                actual: entity.id,
            });
        }

        // reject missing owning worker
        if !self
            .entities
            .contains_key(&resource.id.worker_id.entity_id())
        {
            return Err(TopologyError::UnknownEntity {
                entity_id: resource.id.worker_id.entity_id(),
                role: EntityRole::Source,
            });
        }

        // resource entity
        self.upsert_entity(entity)?;

        // ownership edge
        let edge = Edge::new(
            resource.ownership_edge_id(),
            EdgeKind::WORKER_OWNS_RESOURCE,
            resource.id.worker_id.entity_id(),
            resource.entity_id(),
        );
        self.upsert_edge(edge)?;

        Ok(())
    }

    /// Detach one resource metadata record from one worker.
    pub(crate) fn detach_resource(&mut self, resource: &Resource) -> bool {
        self.remove_entity(resource.entity_id().as_str())
    }

    /// Define one entity kind in topology.
    pub(crate) fn define_entity_kind(&mut self, mut kind: EntityDefinition) -> TopologyResult<()> {
        // reject empty kind identifiers
        if kind.kind.as_str().is_empty() {
            return Err(TopologyError::EmptyKindId);
        }

        // reject duplicate kind identifiers
        if self.has_entity_kind(kind.kind.as_str()) {
            return Err(TopologyError::DuplicateEntityKind {
                kind: kind.kind.clone(),
            });
        }

        // default to base entity faults for user-defined kinds
        if kind.supported_faults.is_empty() {
            kind.supported_faults = base_entity_faults();
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
        if self.has_edge_kind(kind.kind.as_str()) {
            return Err(TopologyError::DuplicateEdgeKind {
                kind: kind.kind.clone(),
            });
        }

        // default to base edge faults for user-defined kinds
        if kind.supported_faults.is_empty() {
            kind.supported_faults = base_edge_faults();
        }

        // insert one kind definition
        self.edge_kinds.insert(kind.kind.clone(), kind);

        Ok(())
    }

    /// Undefine one entity kind in topology.
    pub(crate) fn undefine_entity_kind(&mut self, kind: &EntityKind) -> TopologyResult<()> {
        // reject missing kind identifiers
        if !self.entity_kinds.contains_key(kind.as_str()) {
            return Err(TopologyError::UnknownEntityKind { kind: kind.clone() });
        }

        // protect builtin topology shape
        if is_builtin_entity_kind(kind.as_str()) {
            return Err(TopologyError::BuiltinKind {
                kind: kind.to_string(),
            });
        }

        // reject removing a kind still used by live entities
        if self.entities.values().any(|entity| entity.kind == *kind) {
            return Err(TopologyError::EntityKindInUse { kind: kind.clone() });
        }

        self.entity_kinds.remove(kind.as_str());

        Ok(())
    }

    /// Undefine one edge kind in topology.
    pub(crate) fn undefine_edge_kind(&mut self, kind: &EdgeKind) -> TopologyResult<()> {
        // reject missing kind identifiers
        if !self.edge_kinds.contains_key(kind.as_str()) {
            return Err(TopologyError::UnknownEdgeKind { kind: kind.clone() });
        }

        // protect builtin topology shape
        if is_builtin_edge_kind(kind.as_str()) {
            return Err(TopologyError::BuiltinKind {
                kind: kind.to_string(),
            });
        }

        // reject removing a kind still used by live edges
        if self.edges.values().any(|edge| edge.kind == *kind) {
            return Err(TopologyError::EdgeKindInUse { kind: kind.clone() });
        }

        self.edge_kinds.remove(kind.as_str());

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
        if self.has_entity_kind(kind_id) {
            return Ok(());
        }

        Err(TopologyError::UnknownEntityKind {
            kind: EntityKind::from(kind_id),
        })
    }

    /// Expect one edge kind to be defined.
    fn expect_edge_kind(&self, kind_id: &str) -> TopologyResult<()> {
        if self.has_edge_kind(kind_id) {
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

    /// Return whether one entity kind is registered.
    fn has_entity_kind(&self, kind_id: &str) -> bool {
        self.entity_kinds.contains_key(kind_id) || BUILTIN_ENTITY_KINDS.contains_key(kind_id)
    }

    /// Return whether one edge kind is registered.
    fn has_edge_kind(&self, kind_id: &str) -> bool {
        self.edge_kinds.contains_key(kind_id) || BUILTIN_EDGE_KINDS.contains_key(kind_id)
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
            edge.kind.as_str() == EdgeKind::RUNTIME_OWNS_WORKER && edge.to == worker_entity_id
        })?;

        Some(edge.from.clone())
    }

    /// Return whether one runtime still owns any worker metadata.
    fn runtime_has_workers(&self, runtime_entity_id: &str) -> bool {
        self.edges.values().any(|edge| {
            edge.kind.as_str() == EdgeKind::RUNTIME_OWNS_WORKER
                && edge.from.as_str() == runtime_entity_id
        })
    }
}
