use destack_engine as engine;
use destack_workspace::ExecutionMode;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::ResourceId;
use crate::runtime::engine::Entry;
use crate::runtime::policy::{Policy, Rule, RuleId};
use crate::runtime::{Resource, WorkerId};

use super::{Edge, EdgeDefinition, EdgeId, Entity, EntityDefinition, EntityId, RuntimeId, World};

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
        args: Vec<engine::Value>,
    },
    /// Remove one worker from the world.
    RemoveWorker {
        /// Worker identifier to remove.
        worker_id: WorkerId,
    },
    /// Create one logical world resource.
    CreateResource {
        /// Resource payload to create.
        resource: Resource,
    },
    /// Destroy one logical world resource.
    DestroyResource {
        /// Resource identifier to destroy.
        resource_id: ResourceId,
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
        kind: EntityDefinition,
    },
    /// Define one edge kind in world topology.
    DefineEdgeKind {
        /// Edge kind definition.
        kind: EdgeDefinition,
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

impl Command {
    /// Return the stable command name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Tick => "runtime.world.tick",
            Self::RemoveRuntime { .. } => "runtime.instance.remove",
            Self::RunEntrypoint { .. } => "runtime.instance.entrypoint.run",
            Self::RemoveWorker { .. } => "runtime.worker.remove",
            Self::CreateResource { .. } => "runtime.resource.create",
            Self::DestroyResource { .. } => "runtime.resource.destroy",
            Self::SetPolicy { .. } => "runtime.policy.set",
            Self::InstallRule { .. } => "runtime.policy.rule.install",
            Self::RemoveRule { .. } => "runtime.policy.rule.remove",
            Self::EnableRule { .. } => "runtime.policy.rule.enable",
            Self::DisableRule { .. } => "runtime.policy.rule.disable",
            Self::ReplaceRule { .. } => "runtime.policy.rule.replace",
            Self::DefineEntityKind { .. } => "runtime.topology.entity.kind.define",
            Self::DefineEdgeKind { .. } => "runtime.topology.edge.kind.define",
            Self::UpsertEntity { .. } => "runtime.topology.entity.upsert",
            Self::RemoveEntity { .. } => "runtime.topology.entity.remove",
            Self::UpsertEdge { .. } => "runtime.topology.edge.upsert",
            Self::RemoveEdge { .. } => "runtime.topology.edge.remove",
        }
    }
}

impl World {
    /// Apply one resolved world command without touching authoritative trace.
    pub(crate) fn apply_command(&mut self, command: Command) -> RuntimeResult<()> {
        match command {
            // scheduler progress
            Command::Tick => {
                let _ = self.tick_unrecorded()?;
            }

            // runtime lifecycle
            Command::RemoveRuntime { runtime_id } => {
                let _ = self.remove_runtime_unrecorded(runtime_id)?;
            }

            // entry execution
            Command::RunEntrypoint {
                runtime_id,
                entry,
                args,
            } => {
                let _ = self.run_entrypoint_unrecorded(runtime_id, &entry, &args)?;
            }

            // structural commands
            Command::RemoveWorker { worker_id } => {
                // live runtime worker
                if let Some(runtime_id) = self.runtime_id_for_worker(worker_id) {
                    let runtime = self.runtime_mut(runtime_id)?;

                    runtime.remove_worker(worker_id)?;
                    runtime.shared.remove_worker(worker_id);
                }
                // detached worker metadata
                else if self.state.topology.worker_subject(worker_id).is_none() {
                    return Err(RuntimeError::WorkerNotFound {
                        worker_id: worker_id.0,
                    }
                    .boxed());
                }

                self.remove_worker_metadata(worker_id);
            }
            Command::CreateResource { resource } => {
                self.state.attach_resource(resource)?;
            }
            Command::DestroyResource { resource_id } => {
                self.state.detach_resource(resource_id);
            }
            Command::SetPolicy { policy } => {
                self.state.policy.set_policy(policy, &self.state.topology)?;
            }
            Command::InstallRule { rule } => {
                self.state.policy.install_rule(rule, &self.state.topology)?;
            }
            Command::RemoveRule { rule_id } => {
                self.state.policy.remove_rule(&rule_id)?;
            }
            Command::EnableRule { rule_id } => {
                self.state.policy.enable_rule(&rule_id)?;
            }
            Command::DisableRule { rule_id } => {
                self.state.policy.disable_rule(&rule_id)?;
            }
            Command::ReplaceRule { rule_id, rule } => {
                self.state
                    .policy
                    .replace_rule(&rule_id, rule, &self.state.topology)?;
            }
            Command::DefineEntityKind { kind } => {
                self.state
                    .topology
                    .define_entity_kind(kind)
                    .map_err(Self::internal_error)?;
            }
            Command::DefineEdgeKind { kind } => {
                self.state
                    .topology
                    .define_edge_kind(kind)
                    .map_err(Self::internal_error)?;
            }
            Command::UpsertEntity { entity } => {
                self.state
                    .topology
                    .upsert_entity(entity)
                    .map_err(Self::internal_error)?;
            }
            Command::RemoveEntity { entity_id } => {
                self.state.topology.remove_entity(entity_id.as_str());
            }
            Command::UpsertEdge { edge } => {
                self.state
                    .topology
                    .upsert_edge(edge)
                    .map_err(Self::internal_error)?;
            }
            Command::RemoveEdge { edge_id } => {
                self.state.topology.remove_edge(edge_id.as_str());
            }
        }

        Ok(())
    }

    /// Apply one world command through the authoritative input boundary.
    pub(crate) fn command(&mut self, command: Command) -> RuntimeResult<()> {
        // resolve the requested command first
        let mode = self.state.trace.mode();
        let command = self.resolve_command(command)?;

        // apply the command before appending it to trace
        self.apply_command(command.clone())?;

        // append the authoritative input only after the command succeeds
        if mode == ExecutionMode::Record {
            self.record_command(command)?;
        }

        Ok(())
    }

    /// Remove one worker from the world.
    pub fn remove_worker(&mut self, worker_id: WorkerId) -> RuntimeResult<()> {
        self.command(Command::RemoveWorker { worker_id })
    }

    /// Create one world resource.
    pub fn create_resource(&mut self, resource: Resource) -> RuntimeResult<()> {
        self.command(Command::CreateResource { resource })
    }

    /// Destroy one world resource.
    pub fn destroy_resource(&mut self, resource_id: ResourceId) -> RuntimeResult<()> {
        self.command(Command::DestroyResource { resource_id })
    }

    /// Replace the active policy.
    pub fn set_policy(&mut self, policy: Policy) -> RuntimeResult<()> {
        self.command(Command::SetPolicy { policy })
    }

    /// Install one rule into the active policy.
    pub fn install_rule(&mut self, rule: Rule) -> RuntimeResult<()> {
        self.command(Command::InstallRule { rule })
    }

    /// Install many rules into the active policy in declaration order.
    pub fn install_rules(&mut self, rules: impl IntoIterator<Item = Rule>) -> RuntimeResult<()> {
        for rule in rules {
            self.install_rule(rule)?;
        }

        Ok(())
    }

    /// Remove one rule from the active policy.
    pub fn remove_rule(&mut self, rule_id: RuleId) -> RuntimeResult<()> {
        self.command(Command::RemoveRule { rule_id })
    }

    /// Enable one rule in the active policy.
    pub fn enable_rule(&mut self, rule_id: RuleId) -> RuntimeResult<()> {
        self.command(Command::EnableRule { rule_id })
    }

    /// Disable one rule in the active policy.
    pub fn disable_rule(&mut self, rule_id: RuleId) -> RuntimeResult<()> {
        self.command(Command::DisableRule { rule_id })
    }

    /// Replace one installed rule.
    pub fn replace_rule(&mut self, rule_id: RuleId, rule: Rule) -> RuntimeResult<()> {
        self.command(Command::ReplaceRule { rule_id, rule })
    }

    /// Define one world entity kind.
    pub fn define_entity_kind(&mut self, kind: EntityDefinition) -> RuntimeResult<()> {
        self.command(Command::DefineEntityKind { kind })
    }

    /// Define one world edge kind.
    pub fn define_edge_kind(&mut self, kind: EdgeDefinition) -> RuntimeResult<()> {
        self.command(Command::DefineEdgeKind { kind })
    }

    /// Upsert one world entity.
    pub fn upsert_entity(&mut self, entity: Entity) -> RuntimeResult<()> {
        self.command(Command::UpsertEntity { entity })
    }

    /// Remove one world entity.
    pub fn remove_entity(&mut self, entity_id: EntityId) -> RuntimeResult<()> {
        self.command(Command::RemoveEntity { entity_id })
    }

    /// Upsert one world edge.
    pub fn upsert_edge(&mut self, edge: Edge) -> RuntimeResult<()> {
        self.command(Command::UpsertEdge { edge })
    }

    /// Remove one world edge.
    pub fn remove_edge(&mut self, edge_id: EdgeId) -> RuntimeResult<()> {
        self.command(Command::RemoveEdge { edge_id })
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
        self.state
            .resources
            .retain(|resource_id, _| resource_id.worker_id != worker_id);
        self.state.topology.remove_worker(worker_id);
    }
}
