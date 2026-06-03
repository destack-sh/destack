use destack_workspace::ExecutionMode;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::ResourceId;
use crate::runtime::WorkerId;
use crate::world::policy::{Policy, Rule, RuleId};

use super::{
    Edge, EdgeDefinition, EdgeId, EdgeKind, Entity, EntityDefinition, EntityId, EntityKind,
    RuntimeId, World,
};

/// One world mutation recorded in authoritative trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum Mutation {
    /// One runtime removal mutation.
    RemoveRuntime {
        /// Runtime identifier to remove.
        runtime_id: RuntimeId,
    },
    /// Remove one worker from the world.
    RemoveWorker {
        /// Worker identifier to remove.
        worker_id: WorkerId,
    },
    /// Add one logical world resource.
    AddResource {
        /// Resource identifier to add.
        resource_id: ResourceId,
        /// Topology entity for this resource.
        entity: Entity,
    },
    /// Remove one logical world resource.
    RemoveResource {
        /// Resource identifier to remove.
        resource_id: ResourceId,
    },
    /// Replace the active world policy.
    SetPolicy {
        /// Full policy replacement payload.
        policy: Policy,
    },
    /// Add one rule into the active world policy.
    AddRule {
        /// Rule payload to add.
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
        kind: EntityDefinition,
    },
    /// Undefine one entity kind in world topology.
    UndefineEntityKind {
        /// Entity kind identifier.
        kind: EntityKind,
    },
    /// Define one edge kind in world topology.
    DefineEdgeKind {
        /// Edge kind definition.
        kind: EdgeDefinition,
    },
    /// Undefine one edge kind in world topology.
    UndefineEdgeKind {
        /// Edge kind identifier.
        kind: EdgeKind,
    },
    /// Upsert one entity in world topology.
    UpsertEntity {
        /// Entity payload.
        entity: Entity,
    },
    /// Remove one entity from world topology.
    RemoveEntity {
        /// Stable entity identifier.
        entity_id: EntityId,
    },
    /// Upsert one edge in world topology.
    UpsertEdge {
        /// Edge payload.
        edge: Edge,
    },
    /// Remove one edge from world topology.
    RemoveEdge {
        /// Stable edge identifier.
        edge_id: EdgeId,
    },
}

impl Mutation {
    /// Return the stable mutation name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::RemoveRuntime { .. } => "runtime.instance.remove",
            Self::RemoveWorker { .. } => "runtime.worker.remove",
            Self::AddResource { .. } => "runtime.resource.add",
            Self::RemoveResource { .. } => "runtime.resource.remove",
            Self::SetPolicy { .. } => "runtime.policy.set",
            Self::AddRule { .. } => "runtime.policy.rule.add",
            Self::RemoveRule { .. } => "runtime.policy.rule.remove",
            Self::EnableRule { .. } => "runtime.policy.rule.enable",
            Self::DisableRule { .. } => "runtime.policy.rule.disable",
            Self::ReplaceRule { .. } => "runtime.policy.rule.replace",
            Self::DefineEntityKind { .. } => "runtime.topology.entity.kind.define",
            Self::UndefineEntityKind { .. } => "runtime.topology.entity.kind.undefine",
            Self::DefineEdgeKind { .. } => "runtime.topology.edge.kind.define",
            Self::UndefineEdgeKind { .. } => "runtime.topology.edge.kind.undefine",
            Self::UpsertEntity { .. } => "runtime.topology.entity.upsert",
            Self::RemoveEntity { .. } => "runtime.topology.entity.remove",
            Self::UpsertEdge { .. } => "runtime.topology.edge.upsert",
            Self::RemoveEdge { .. } => "runtime.topology.edge.remove",
        }
    }
}

impl World {
    /// Apply one resolved world mutation without touching authoritative trace.
    pub(crate) fn apply_mutation(&mut self, mutation: Mutation) -> RuntimeResult<()> {
        match mutation {
            // runtime lifecycle
            Mutation::RemoveRuntime { runtime_id } => {
                let _ = self.remove_stored_runtime(runtime_id)?;
            }

            // structural mutations
            Mutation::RemoveWorker { worker_id } => {
                // live runtime worker
                if let Some(runtime_id) = self.runtime_id_for_worker(worker_id) {
                    let runtime = self.runtime_mut(runtime_id)?;

                    runtime.remove_worker(worker_id)?;
                    runtime.heap.remove_worker(worker_id);
                }
                // detached worker metadata
                else if self.state.topology.worker_subject(worker_id).is_none() {
                    return Err(RuntimeError::worker_not_found(worker_id.0).boxed());
                }

                self.remove_worker_metadata(worker_id);
            }
            Mutation::AddResource {
                resource_id,
                entity,
            } => {
                self.state.attach_resource(resource_id, entity)?;
            }
            Mutation::RemoveResource { resource_id } => {
                self.state.detach_resource(resource_id);
            }
            Mutation::SetPolicy { policy } => {
                self.state.policy.set_policy(policy)?;
            }
            Mutation::AddRule { rule } => {
                self.state.policy.add_rule(rule)?;
            }
            Mutation::RemoveRule { rule_id } => {
                self.state.policy.remove_rule(&rule_id)?;
            }
            Mutation::EnableRule { rule_id } => {
                self.state.policy.enable_rule(&rule_id)?;
            }
            Mutation::DisableRule { rule_id } => {
                self.state.policy.disable_rule(&rule_id)?;
            }
            Mutation::ReplaceRule { rule_id, rule } => {
                self.state.policy.replace_rule(&rule_id, rule)?;
            }
            Mutation::DefineEntityKind { kind } => {
                self.state
                    .topology
                    .define_entity_kind(kind)
                    .map_err(Self::internal_error)?;
            }
            Mutation::UndefineEntityKind { kind } => {
                self.state
                    .topology
                    .undefine_entity_kind(&kind)
                    .map_err(Self::internal_error)?;
            }
            Mutation::DefineEdgeKind { kind } => {
                self.state
                    .topology
                    .define_edge_kind(kind)
                    .map_err(Self::internal_error)?;
            }
            Mutation::UndefineEdgeKind { kind } => {
                self.state
                    .topology
                    .undefine_edge_kind(&kind)
                    .map_err(Self::internal_error)?;
            }
            Mutation::UpsertEntity { entity } => {
                self.state
                    .topology
                    .upsert_entity(entity)
                    .map_err(Self::internal_error)?;
            }
            Mutation::RemoveEntity { entity_id } => {
                self.state.topology.remove_entity(entity_id.as_str());
            }
            Mutation::UpsertEdge { edge } => {
                self.state
                    .topology
                    .upsert_edge(edge)
                    .map_err(Self::internal_error)?;
            }
            Mutation::RemoveEdge { edge_id } => {
                self.state.topology.remove_edge(edge_id.as_str());
            }
        }

        Ok(())
    }

    /// Apply one world mutation through the authoritative input boundary.
    pub(crate) fn mutate(&mut self, mutation: Mutation) -> RuntimeResult<()> {
        // resolve the requested mutation first
        let mode = self.state.trace.mode();
        let mutation = self.resolve_mutation(mutation)?;

        // apply the mutation before appending it to trace
        self.apply_mutation(mutation.clone())?;

        // append the authoritative input only after the mutation succeeds
        if mode == ExecutionMode::Record {
            self.record_mutation(mutation)?;
        }

        Ok(())
    }

    /// Remove one worker from the world.
    pub fn remove_worker(&mut self, worker_id: WorkerId) -> RuntimeResult<()> {
        self.mutate(Mutation::RemoveWorker { worker_id })
    }

    /// Add one world resource.
    pub fn add_resource(&mut self, resource_id: ResourceId, entity: Entity) -> RuntimeResult<()> {
        self.mutate(Mutation::AddResource {
            resource_id,
            entity,
        })
    }

    /// Remove one world resource.
    pub fn remove_resource(&mut self, resource_id: ResourceId) -> RuntimeResult<()> {
        self.mutate(Mutation::RemoveResource { resource_id })
    }

    /// Replace the active policy.
    pub fn set_policy(&mut self, policy: Policy) -> RuntimeResult<()> {
        self.mutate(Mutation::SetPolicy { policy })
    }

    /// Add one rule into the active policy.
    pub fn add_rule(&mut self, rule: Rule) -> RuntimeResult<()> {
        self.mutate(Mutation::AddRule { rule })
    }

    /// Add many rules into the active policy in declaration order.
    pub fn add_rules(&mut self, rules: impl IntoIterator<Item = Rule>) -> RuntimeResult<()> {
        for rule in rules {
            self.add_rule(rule)?;
        }

        Ok(())
    }

    /// Remove one rule from the active policy.
    pub fn remove_rule(&mut self, rule_id: RuleId) -> RuntimeResult<()> {
        self.mutate(Mutation::RemoveRule { rule_id })
    }

    /// Enable one rule in the active policy.
    pub fn enable_rule(&mut self, rule_id: RuleId) -> RuntimeResult<()> {
        self.mutate(Mutation::EnableRule { rule_id })
    }

    /// Disable one rule in the active policy.
    pub fn disable_rule(&mut self, rule_id: RuleId) -> RuntimeResult<()> {
        self.mutate(Mutation::DisableRule { rule_id })
    }

    /// Replace one active rule.
    pub fn replace_rule(&mut self, rule_id: RuleId, rule: Rule) -> RuntimeResult<()> {
        self.mutate(Mutation::ReplaceRule { rule_id, rule })
    }

    /// Define one world entity kind.
    pub fn define_entity_kind(&mut self, kind: EntityDefinition) -> RuntimeResult<()> {
        self.mutate(Mutation::DefineEntityKind { kind })
    }

    /// Undefine one world entity kind.
    pub fn undefine_entity_kind(&mut self, kind: EntityKind) -> RuntimeResult<()> {
        self.mutate(Mutation::UndefineEntityKind { kind })
    }

    /// Define one world edge kind.
    pub fn define_edge_kind(&mut self, kind: EdgeDefinition) -> RuntimeResult<()> {
        self.mutate(Mutation::DefineEdgeKind { kind })
    }

    /// Undefine one world edge kind.
    pub fn undefine_edge_kind(&mut self, kind: EdgeKind) -> RuntimeResult<()> {
        self.mutate(Mutation::UndefineEdgeKind { kind })
    }

    /// Upsert one world entity.
    pub fn upsert_entity(&mut self, entity: Entity) -> RuntimeResult<()> {
        self.mutate(Mutation::UpsertEntity { entity })
    }

    /// Remove one world entity.
    pub fn remove_entity(&mut self, entity_id: EntityId) -> RuntimeResult<()> {
        self.mutate(Mutation::RemoveEntity { entity_id })
    }

    /// Upsert one world edge.
    pub fn upsert_edge(&mut self, edge: Edge) -> RuntimeResult<()> {
        self.mutate(Mutation::UpsertEdge { edge })
    }

    /// Remove one world edge.
    pub fn remove_edge(&mut self, edge_id: EdgeId) -> RuntimeResult<()> {
        self.mutate(Mutation::RemoveEdge { edge_id })
    }

    /// Build one internal runtime error.
    fn internal_error(message: impl fmt::Display) -> Box<RuntimeError> {
        RuntimeError::Internal {
            message: message.to_string(),
        }
        .boxed()
    }

    /// Return the runtime that owns one live worker.
    fn runtime_id_for_worker(&self, worker_id: WorkerId) -> Option<RuntimeId> {
        self.runtimes.iter().find_map(|(runtime_id, runtime)| {
            runtime.worker(worker_id).is_some().then_some(*runtime_id)
        })
    }

    /// Remove worker metadata from world-owned registries.
    pub(super) fn remove_worker_metadata(&mut self, worker_id: WorkerId) {
        self.state.topology.remove_worker(worker_id);
    }
}
