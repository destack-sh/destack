use std::collections::BTreeMap;

use crate::diagnostic::RuntimeResult;
use crate::runtime::policy::Policy;
use crate::runtime::topology::{
    RuntimeId, WorldEdge, WorldEdgeKind, WorldEdgeKindDefinition, WorldEntity, WorldEntityKind,
    WorldEntityKindDefinition,
};
use crate::runtime::world::{WorldResource, WorldResourceId};
use crate::runtime::{AgentId, AgentImage, RuntimeImage};

use super::{Image, Moment};

/// One branch divergence between two committed branch heads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Divergence {
    /// The shared base moment before the branches diverged.
    pub base: Moment,
    /// The committed head moment on the left branch.
    pub left: Moment,
    /// The committed head moment on the right branch.
    pub right: Moment,
}

/// One inspectable committed world image view at one exact moment.
#[derive(Debug, Clone)]
pub struct WorldView {
    /// The moment this view represents.
    moment: Moment,
    /// The exact materialized image for this view.
    image: Image,
}

impl WorldView {
    /// Create one committed world view from one exact moment and image.
    pub(super) fn new(moment: Moment, image: Image) -> Self {
        Self { moment, image }
    }

    /// Return the exact moment this view represents.
    pub fn moment(&self) -> Moment {
        self.moment
    }

    /// Return the underlying materialized image.
    pub fn image(&self) -> &Image {
        &self.image
    }

    /// Return the policy visible at this moment.
    pub fn policy(&self) -> &Policy {
        self.image.policy()
    }

    /// Return all runtimes visible at this moment.
    pub fn runtimes(&self) -> &BTreeMap<RuntimeId, RuntimeImage> {
        self.image.runtimes()
    }

    /// Return all agents visible at this moment.
    pub fn agents(&self) -> &BTreeMap<AgentId, AgentImage> {
        self.image.agents()
    }

    /// Return all logical world resources visible at this moment.
    pub fn resources(&self) -> &BTreeMap<WorldResourceId, WorldResource> {
        self.image.resources()
    }

    /// Return all runtime ids visible at this moment.
    pub fn runtime_ids(&self) -> Vec<RuntimeId> {
        self.image.runtimes.keys().copied().collect()
    }

    /// Return all agent ids visible at this moment.
    pub fn agent_ids(&self) -> Vec<AgentId> {
        self.image.agents.keys().copied().collect()
    }

    /// Return the number of runtimes visible at this moment.
    pub fn runtime_count(&self) -> usize {
        self.image.runtime_count()
    }

    /// Return the number of agents visible at this moment.
    pub fn agent_count(&self) -> usize {
        self.image.agent_count()
    }

    /// Return the number of logical resources visible at this moment.
    pub fn resource_count(&self) -> usize {
        self.image.resource_count()
    }

    /// Return the number of topology entities visible at this moment.
    pub fn entity_count(&self) -> usize {
        self.image.entity_count()
    }

    /// Return the number of topology edges visible at this moment.
    pub fn edge_count(&self) -> usize {
        self.image.edge_count()
    }

    /// Report whether one runtime exists at this moment.
    pub fn has_runtime(&self, runtime_id: RuntimeId) -> bool {
        self.image.has_runtime(runtime_id)
    }

    /// Report whether one agent exists at this moment.
    pub fn has_agent(&self, agent_id: AgentId) -> bool {
        self.image.has_agent(agent_id)
    }

    /// Report whether one resource exists at this moment.
    pub fn has_resource(&self, resource_id: WorldResourceId) -> bool {
        self.image.has_resource(resource_id)
    }

    /// Report whether one topology entity exists at this moment.
    pub fn has_entity(&self, entity_id: &str) -> bool {
        self.image.has_entity(entity_id)
    }

    /// Report whether one topology edge exists at this moment.
    pub fn has_edge(&self, edge_id: &str) -> bool {
        self.image.has_edge(edge_id)
    }

    /// Return one runtime image by id.
    pub fn runtime(&self, runtime_id: RuntimeId) -> RuntimeResult<&RuntimeImage> {
        self.image.runtime(runtime_id)
    }

    /// Return labels for one runtime visible at this moment.
    pub fn runtime_labels(
        &self,
        runtime_id: RuntimeId,
    ) -> RuntimeResult<&BTreeMap<String, String>> {
        self.image.runtime_labels(runtime_id)
    }

    /// Return one agent image by id.
    pub fn agent(&self, agent_id: AgentId) -> RuntimeResult<&AgentImage> {
        self.image.agent(agent_id)
    }

    /// Return labels for one agent visible at this moment.
    pub fn agent_labels(&self, agent_id: AgentId) -> RuntimeResult<&BTreeMap<String, String>> {
        self.image.agent_labels(agent_id)
    }

    /// Return one resource by id when present.
    pub fn resource(&self, resource_id: WorldResourceId) -> Option<&WorldResource> {
        self.image.resource(resource_id)
    }

    /// Return one topology entity by id when present.
    pub fn entity(&self, entity_id: &str) -> Option<&WorldEntity> {
        self.image.entity(entity_id)
    }

    /// Return all topology entity kinds visible at this moment.
    pub fn entity_kinds(&self) -> &BTreeMap<WorldEntityKind, WorldEntityKindDefinition> {
        self.image.topology.entity_kinds()
    }

    /// Return one topology entity kind by id when present.
    pub fn entity_kind(&self, kind_id: &str) -> Option<&WorldEntityKindDefinition> {
        self.image.topology.entity_kinds().get(kind_id)
    }

    /// Return one topology edge by id when present.
    pub fn edge(&self, edge_id: &str) -> Option<&WorldEdge> {
        self.image.edge(edge_id)
    }

    /// Return all topology edge kinds visible at this moment.
    pub fn edge_kinds(&self) -> &BTreeMap<WorldEdgeKind, WorldEdgeKindDefinition> {
        self.image.topology.edge_kinds()
    }

    /// Return one topology edge kind by id when present.
    pub fn edge_kind(&self, kind_id: &str) -> Option<&WorldEdgeKindDefinition> {
        self.image.topology.edge_kinds().get(kind_id)
    }
}
