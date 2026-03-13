use crate::runtime::AgentId;
use crate::runtime::policy::{Policy, Rule, RuleId};
use serde::{Deserialize, Serialize};

use super::resource::{WorldResource, WorldResourceId};
use super::topology::{
    WorldEdge, WorldEdgeId, WorldEdgeKindDefinition, WorldEntity, WorldEntityId,
    WorldEntityKindDefinition,
};

/// One explicit structural world-state mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mutation {
    /// Remove one agent from the world.
    RemoveAgent {
        /// Agent identifier to remove.
        agent_id: AgentId,
    },
    /// Create one logical world resource.
    CreateResource {
        /// Resource payload to create.
        resource: WorldResource,
    },
    /// Destroy one logical world resource.
    DestroyResource {
        /// Resource identifier to destroy.
        resource_id: WorldResourceId,
    },
    /// Replace the active world policy.
    SetPolicy {
        /// Full policy replacement payload.
        policy: Policy,
    },
    /// Install one rule into the active world policy.
    InstallRule {
        /// Rule payload to install.
        rule: Rule,
    },
    /// Remove one rule from the active world policy.
    RemoveRule {
        /// Stable rule identifier.
        rule_id: RuleId,
    },
    /// Enable one rule in the active world policy.
    EnableRule {
        /// Stable rule identifier.
        rule_id: RuleId,
    },
    /// Disable one rule in the active world policy.
    DisableRule {
        /// Stable rule identifier.
        rule_id: RuleId,
    },
    /// Replace one rule in the active world policy.
    ReplaceRule {
        /// Stable rule identifier to replace.
        rule_id: RuleId,
        /// Replacement rule payload.
        rule: Rule,
    },
    /// Define one entity kind in world topology.
    DefineEntityKind {
        /// Entity kind definition.
        kind: WorldEntityKindDefinition,
    },
    /// Define one edge kind in world topology.
    DefineEdgeKind {
        /// Edge kind definition.
        kind: WorldEdgeKindDefinition,
    },
    /// Upsert one entity in world topology.
    UpsertEntity {
        /// Entity payload.
        entity: WorldEntity,
    },
    /// Remove one entity from world topology.
    RemoveEntity {
        /// Stable entity identifier.
        entity_id: WorldEntityId,
    },
    /// Upsert one edge in world topology.
    UpsertEdge {
        /// Edge payload.
        edge: WorldEdge,
    },
    /// Remove one edge from world topology.
    RemoveEdge {
        /// Stable edge identifier.
        edge_id: WorldEdgeId,
    },
}

impl Mutation {
    /// Return the stable mutation name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::RemoveAgent { .. } => "agent.remove",
            Self::CreateResource { .. } => "resource.create",
            Self::DestroyResource { .. } => "resource.destroy",
            Self::SetPolicy { .. } => "policy.set",
            Self::InstallRule { .. } => "rule.install",
            Self::RemoveRule { .. } => "rule.remove",
            Self::EnableRule { .. } => "rule.enable",
            Self::DisableRule { .. } => "rule.disable",
            Self::ReplaceRule { .. } => "rule.replace",
            Self::DefineEntityKind { .. } => "topology.define_entity_kind",
            Self::DefineEdgeKind { .. } => "topology.define_edge_kind",
            Self::UpsertEntity { .. } => "topology.upsert_entity",
            Self::RemoveEntity { .. } => "topology.remove_entity",
            Self::UpsertEdge { .. } => "topology.upsert_edge",
            Self::RemoveEdge { .. } => "topology.remove_edge",
        }
    }
}
