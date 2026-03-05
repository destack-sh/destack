use std::collections::BTreeMap;

use crate::runtime::AgentId;
use crate::runtime::policy::{Policy, Rule, RuleId};
use serde::{Deserialize, Serialize};

use super::resource::{WorldResource, WorldResourceId};
use super::topology::{
    RuntimeId, WorldEdge, WorldEdgeId, WorldEdgeKindDefinition, WorldEntity, WorldEntityId,
    WorldEntityKindDefinition,
};

/// World mutation command payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldCommand {
    /// Create one runtime and its primary agent.
    CreateRuntime {
        /// Runtime identifier to create.
        runtime_id: RuntimeId,
        /// Runtime name for selector matching.
        runtime_name: String,
        /// Runtime labels for selector matching.
        runtime_labels: BTreeMap<String, String>,
        /// Primary agent identifier to create.
        primary_agent_id: AgentId,
        /// Primary agent name for selector matching.
        primary_agent_name: String,
        /// Primary agent labels for selector matching.
        primary_agent_labels: BTreeMap<String, String>,
    },
    /// Create one agent in one existing runtime.
    CreateAgent {
        /// Runtime identifier that owns this agent.
        runtime_id: RuntimeId,
        /// Agent identifier to create.
        agent_id: AgentId,
        /// Agent name for selector matching.
        agent_name: String,
        /// Agent labels for selector matching.
        agent_labels: BTreeMap<String, String>,
    },
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
