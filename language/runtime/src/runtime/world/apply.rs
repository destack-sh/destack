use super::{
    Command, World, WorldEdge, WorldEdgeId, WorldEdgeKindDefinition, WorldEntity, WorldEntityId,
    WorldEntityKindDefinition,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::policy::{Policy, Rule, RuleId};
use crate::runtime::{WorkerId, WorldResource, WorldResourceId};
use destack_workspace::ExecutionMode;
use std::fmt;

impl World {
    /// Apply one resolved world command without touching authoritative trace.
    pub(crate) fn apply_command(&mut self, command: Command) -> RuntimeResult<()> {
        match command {
            // scheduler progress
            Command::Tick => {
                let _ = self.tick_inner()?;
            }

            // runtime lifecycle
            Command::RemoveRuntime { runtime_id } => {
                let _ = self.remove_runtime_inner(runtime_id)?;
            }

            // entry execution
            Command::RunEntrypoint {
                runtime_id,
                entry,
                args,
            } => {
                let _ = self.run_entrypoint_inner(runtime_id, &entry, &args)?;
            }

            // structural commands
            Command::RemoveWorker { worker_id } => {
                self.mark_roots.leave_root_scan(worker_id);
                self.mark_roots.leave_edge_scan(worker_id);
                self.mark_roots.remove_direct_roots(worker_id);
                self.resources
                    .retain(|resource_id, _| resource_id.worker_id != worker_id);
                self.topology.remove_worker(worker_id);
            }
            Command::CreateResource { resource } => {
                self.topology
                    .attach_resource(
                        resource.id,
                        resource.kind.clone(),
                        resource.label.as_deref(),
                    )
                    .map_err(Self::internal_error)?;
                self.resources.insert(resource.id, resource);
            }
            Command::DestroyResource { resource_id } => {
                self.topology.detach_resource(resource_id);
                self.resources.remove(&resource_id);
            }
            Command::SetPolicy { policy } => {
                self.policy.set_policy(policy, &self.topology)?;
            }
            Command::InstallRule { rule } => {
                self.policy.install_rule(rule, &self.topology)?;
            }
            Command::RemoveRule { rule_id } => {
                self.policy.remove_rule(&rule_id)?;
            }
            Command::EnableRule { rule_id } => {
                self.policy.enable_rule(&rule_id)?;
            }
            Command::DisableRule { rule_id } => {
                self.policy.disable_rule(&rule_id)?;
            }
            Command::ReplaceRule { rule_id, rule } => {
                self.policy.replace_rule(&rule_id, rule, &self.topology)?;
            }
            Command::DefineEntityKind { kind } => {
                self.topology
                    .define_entity_kind(kind)
                    .map_err(Self::internal_error)?;
            }
            Command::DefineEdgeKind { kind } => {
                self.topology
                    .define_edge_kind(kind)
                    .map_err(Self::internal_error)?;
            }
            Command::UpsertEntity { entity } => {
                self.topology
                    .upsert_entity(entity)
                    .map_err(Self::internal_error)?;
            }
            Command::RemoveEntity { entity_id } => {
                self.topology.remove_entity(entity_id.as_str());
            }
            Command::UpsertEdge { edge } => {
                self.topology
                    .upsert_edge(edge)
                    .map_err(Self::internal_error)?;
            }
            Command::RemoveEdge { edge_id } => {
                self.topology.remove_edge(edge_id.as_str());
            }
        }

        Ok(())
    }

    /// Apply one world command through the authoritative input boundary.
    pub(crate) fn command(&mut self, command: Command) -> RuntimeResult<()> {
        // resolve the requested command first
        let mode = self.trace.mode();
        let command = self.resolve_command(command)?;

        // apply the command before appending it to trace
        self.apply_command(command.clone())?;

        // append the authoritative input only after the command succeeds
        if mode == ExecutionMode::Record {
            self.ingest(command)?;
        }

        Ok(())
    }

    /// Remove one worker from the world.
    pub fn remove_worker(&mut self, worker_id: WorkerId) -> RuntimeResult<()> {
        self.command(Command::RemoveWorker { worker_id })
    }

    /// Create one world resource.
    pub fn create_resource(&mut self, resource: WorldResource) -> RuntimeResult<()> {
        self.command(Command::CreateResource { resource })
    }

    /// Destroy one world resource.
    pub fn destroy_resource(&mut self, resource_id: WorldResourceId) -> RuntimeResult<()> {
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
    pub fn define_entity_kind(&mut self, kind: WorldEntityKindDefinition) -> RuntimeResult<()> {
        self.command(Command::DefineEntityKind { kind })
    }

    /// Define one world edge kind.
    pub fn define_edge_kind(&mut self, kind: WorldEdgeKindDefinition) -> RuntimeResult<()> {
        self.command(Command::DefineEdgeKind { kind })
    }

    /// Upsert one world entity.
    pub fn upsert_entity(&mut self, entity: WorldEntity) -> RuntimeResult<()> {
        self.command(Command::UpsertEntity { entity })
    }

    /// Remove one world entity.
    pub fn remove_entity(&mut self, entity_id: WorldEntityId) -> RuntimeResult<()> {
        self.command(Command::RemoveEntity { entity_id })
    }

    /// Upsert one world edge.
    pub fn upsert_edge(&mut self, edge: WorldEdge) -> RuntimeResult<()> {
        self.command(Command::UpsertEdge { edge })
    }

    /// Remove one world edge.
    pub fn remove_edge(&mut self, edge_id: WorldEdgeId) -> RuntimeResult<()> {
        self.command(Command::RemoveEdge { edge_id })
    }

    /// Build one internal runtime error.
    fn internal_error(message: impl fmt::Display) -> Box<RuntimeError> {
        RuntimeError::Internal {
            message: message.to_string(),
        }
        .boxed()
    }
}
