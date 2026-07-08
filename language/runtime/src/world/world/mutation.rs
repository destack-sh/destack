use destack_repository::ExecutionMode;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::ResourceId;
use crate::runtime::{RuntimeImage, WorkerId, WorkerImage};
use crate::world::policy::{Policy, Rule, RuleId};

use super::{
    Edge, EdgeDefinition, EdgeId, EdgeKind, Entity, EntityDefinition, EntityId, EntityKind,
    RestoreContext, RuntimeId, World,
};

/// One world mutation recorded in authoritative trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum Mutation {
    /// Add one runtime to the world.
    SpawnRuntime {
        /// The created runtime identifier.
        runtime_id: RuntimeId,
        /// The created runtime topology entity.
        runtime_entity: Entity,
        /// Captured runtime metadata for the created runtime.
        runtime: Arc<RuntimeImage>,
        /// Captured workers keyed by worker identifier.
        workers: BTreeMap<WorkerId, SpawnedWorker>,
    },
    /// One runtime removal mutation.
    RemoveRuntime {
        /// Runtime identifier to remove.
        runtime_id: RuntimeId,
    },
    /// Add one worker to an existing runtime.
    SpawnWorker {
        /// The owning runtime identifier.
        runtime_id: RuntimeId,
        /// The created worker identifier.
        worker_id: WorkerId,
        /// The created worker topology entity.
        worker_entity: Entity,
        /// Captured worker metadata for the created worker.
        worker: Arc<WorkerImage>,
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

/// One worker image paired with its spawn identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpawnedWorker {
    /// The created worker topology entity.
    pub entity: Entity,
    /// The captured worker image.
    pub image: Arc<WorkerImage>,
}

impl PartialEq for Mutation {
    /// Compare mutation payloads for replay validation.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::SpawnRuntime {
                    runtime_id,
                    runtime_entity,
                    runtime,
                    workers,
                },
                Self::SpawnRuntime {
                    runtime_id: other_runtime_id,
                    runtime_entity: other_runtime_entity,
                    runtime: other_runtime,
                    workers: other_workers,
                },
            ) => {
                runtime_id == other_runtime_id
                    && runtime_entity == other_runtime_entity
                    && runtime.is_same_image(other_runtime)
                    && workers == other_workers
            }
            (Self::RemoveRuntime { runtime_id }, Self::RemoveRuntime { runtime_id: other }) => {
                runtime_id == other
            }
            (
                Self::SpawnWorker {
                    runtime_id,
                    worker_id,
                    worker_entity,
                    worker,
                },
                Self::SpawnWorker {
                    runtime_id: other_runtime_id,
                    worker_id: other_worker_id,
                    worker_entity: other_worker_entity,
                    worker: other_worker,
                },
            ) => {
                runtime_id == other_runtime_id
                    && worker_id == other_worker_id
                    && worker_entity == other_worker_entity
                    && worker == other_worker
            }
            (Self::RemoveWorker { worker_id }, Self::RemoveWorker { worker_id: other }) => {
                worker_id == other
            }
            (
                Self::AddResource {
                    resource_id,
                    entity,
                },
                Self::AddResource {
                    resource_id: other_resource_id,
                    entity: other_entity,
                },
            ) => resource_id == other_resource_id && entity == other_entity,
            (Self::RemoveResource { resource_id }, Self::RemoveResource { resource_id: other }) => {
                resource_id == other
            }
            (Self::SetPolicy { policy }, Self::SetPolicy { policy: other }) => policy == other,
            (Self::AddRule { rule }, Self::AddRule { rule: other }) => rule == other,
            (Self::RemoveRule { rule_id }, Self::RemoveRule { rule_id: other }) => rule_id == other,
            (Self::EnableRule { rule_id }, Self::EnableRule { rule_id: other }) => rule_id == other,
            (Self::DisableRule { rule_id }, Self::DisableRule { rule_id: other }) => {
                rule_id == other
            }
            (
                Self::ReplaceRule { rule_id, rule },
                Self::ReplaceRule {
                    rule_id: other_rule_id,
                    rule: other_rule,
                },
            ) => rule_id == other_rule_id && rule == other_rule,
            (Self::DefineEntityKind { kind }, Self::DefineEntityKind { kind: other }) => {
                kind == other
            }
            (Self::UndefineEntityKind { kind }, Self::UndefineEntityKind { kind: other }) => {
                kind == other
            }
            (Self::DefineEdgeKind { kind }, Self::DefineEdgeKind { kind: other }) => kind == other,
            (Self::UndefineEdgeKind { kind }, Self::UndefineEdgeKind { kind: other }) => {
                kind == other
            }
            (Self::UpsertEntity { entity }, Self::UpsertEntity { entity: other }) => {
                entity == other
            }
            (Self::RemoveEntity { entity_id }, Self::RemoveEntity { entity_id: other }) => {
                entity_id == other
            }
            (Self::UpsertEdge { edge }, Self::UpsertEdge { edge: other }) => edge == other,
            (Self::RemoveEdge { edge_id }, Self::RemoveEdge { edge_id: other }) => edge_id == other,
            _ => false,
        }
    }
}

impl Mutation {
    /// Return the stable mutation name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::SpawnRuntime { .. } => "runtime.instance.spawned",
            Self::RemoveRuntime { .. } => "runtime.instance.remove",
            Self::SpawnWorker { .. } => "runtime.worker.spawned",
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
        self.apply_mutation_with_restore(mutation, RestoreContext::empty())
    }

    /// Apply one resolved world mutation with explicit restore context.
    pub(crate) fn apply_mutation_with_restore(
        &mut self,
        mutation: Mutation,
        restore: RestoreContext<'_>,
    ) -> RuntimeResult<()> {
        match mutation {
            // runtime lifecycle
            Mutation::SpawnRuntime {
                runtime_id,
                runtime_entity,
                runtime,
                workers,
            } => {
                self.restore_runtime_image(
                    runtime_id,
                    runtime_entity,
                    &runtime,
                    &workers,
                    restore,
                )?;
            }
            Mutation::RemoveRuntime { runtime_id } => {
                let _ = self.remove_stored_runtime(runtime_id)?;
            }

            // structural mutations
            Mutation::SpawnWorker {
                runtime_id,
                worker_id,
                worker_entity,
                worker,
            } => {
                self.restore_worker_image(runtime_id, worker_id, worker_entity, &worker, restore)?;
            }
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
