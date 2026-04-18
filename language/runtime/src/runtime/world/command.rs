use destack_heap as heap;
use serde::{Deserialize, Serialize};

use crate::runtime::engine::Entry;
use crate::runtime::policy::{Policy, Rule, RuleId};
use crate::runtime::{WorkerId, WorldResource, WorldResourceId};

use super::{
    RuntimeId, WorldEdge, WorldEdgeId, WorldEdgeKindDefinition, WorldEntity, WorldEntityId,
    WorldEntityKindDefinition,
};

/// One world command recorded in authoritative trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum Command {
    /// One world tick command.
    Tick,
    /// One runtime removal command.
    RemoveRuntime {
        /// Runtime identifier to remove.
        runtime_id: RuntimeId,
    },
    /// One runtime entrypoint command.
    RunEntrypoint {
        /// Runtime identifier that owns the entrypoint execution.
        runtime_id: RuntimeId,
        /// Replayable entrypoint reference.
        entry: Entry,
        /// Invocation arguments.
        args: Vec<heap::Value>,
    },
    /// Remove one worker from the world.
    RemoveWorker {
        /// Worker identifier to remove.
        worker_id: WorkerId,
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

impl Command {
    /// Return the stable command name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Tick => "world.tick",
            Self::RemoveRuntime { .. } => "runtime.remove",
            Self::RunEntrypoint { .. } => "runtime.run_entrypoint",
            Self::RemoveWorker { .. } => "worker.remove",
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
