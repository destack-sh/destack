use std::collections::BTreeMap;
use std::fmt;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::policy::{Policy, PolicyState, Rule, RuleId};
use crate::runtime::{AgentId, WorldResource, WorldResourceId};
use destack_workspace::ExecutionMode;

use super::topology::Topology;
use super::{
    World, WorldCommand, WorldEdge, WorldEdgeId, WorldEdgeKindDefinition, WorldEntity,
    WorldEntityId, WorldEntityKindDefinition,
};

impl World {
    /// Apply one internal world control command and return the new revision.
    pub(crate) fn apply_control_command(&self, command: WorldCommand) -> RuntimeResult<u64> {
        self.apply_control_command_at_revision(None, command)
    }

    /// Apply one internal world control command at an expected revision.
    pub(crate) fn apply_control_command_at_revision(
        &self,
        expected_revision: Option<u64>,
        command: WorldCommand,
    ) -> RuntimeResult<u64> {
        self.apply_control_command_inner(expected_revision, command)
    }

    /// Remove one agent from the world.
    pub fn remove_agent(&self, agent_id: AgentId) -> RuntimeResult<u64> {
        self.apply_control_command(WorldCommand::RemoveAgent { agent_id })
    }

    /// Create one world resource.
    pub fn create_resource(&self, resource: WorldResource) -> RuntimeResult<u64> {
        self.apply_control_command(WorldCommand::CreateResource { resource })
    }

    /// Destroy one world resource.
    pub fn destroy_resource(&self, resource_id: WorldResourceId) -> RuntimeResult<u64> {
        self.apply_control_command(WorldCommand::DestroyResource { resource_id })
    }

    /// Replace the active policy.
    pub fn set_policy(&self, policy: Policy) -> RuntimeResult<u64> {
        self.apply_control_command(WorldCommand::SetPolicy { policy })
    }

    /// Install one rule into the active policy.
    pub fn install_rule(&self, rule: Rule) -> RuntimeResult<u64> {
        self.apply_control_command(WorldCommand::InstallRule { rule })
    }

    /// Install many rules into the active policy in declaration order.
    pub fn install_rules(&self, rules: impl IntoIterator<Item = Rule>) -> RuntimeResult<u64> {
        let mut revision = self.revision();

        for rule in rules {
            revision = self.install_rule(rule)?;
        }

        Ok(revision)
    }

    /// Remove one rule from the active policy.
    pub fn remove_rule(&self, rule_id: RuleId) -> RuntimeResult<u64> {
        self.apply_control_command(WorldCommand::RemoveRule { rule_id })
    }

    /// Enable one rule in the active policy.
    pub fn enable_rule(&self, rule_id: RuleId) -> RuntimeResult<u64> {
        self.apply_control_command(WorldCommand::EnableRule { rule_id })
    }

    /// Disable one rule in the active policy.
    pub fn disable_rule(&self, rule_id: RuleId) -> RuntimeResult<u64> {
        self.apply_control_command(WorldCommand::DisableRule { rule_id })
    }

    /// Replace one installed rule.
    pub fn replace_rule(&self, rule_id: RuleId, rule: Rule) -> RuntimeResult<u64> {
        self.apply_control_command(WorldCommand::ReplaceRule { rule_id, rule })
    }

    /// Define one world entity kind.
    pub fn define_entity_kind(&self, kind: WorldEntityKindDefinition) -> RuntimeResult<u64> {
        self.apply_control_command(WorldCommand::DefineEntityKind { kind })
    }

    /// Define one world edge kind.
    pub fn define_edge_kind(&self, kind: WorldEdgeKindDefinition) -> RuntimeResult<u64> {
        self.apply_control_command(WorldCommand::DefineEdgeKind { kind })
    }

    /// Upsert one world entity.
    pub fn upsert_entity(&self, entity: WorldEntity) -> RuntimeResult<u64> {
        self.apply_control_command(WorldCommand::UpsertEntity { entity })
    }

    /// Remove one world entity.
    pub fn remove_entity(&self, entity_id: WorldEntityId) -> RuntimeResult<u64> {
        self.apply_control_command(WorldCommand::RemoveEntity { entity_id })
    }

    /// Upsert one world edge.
    pub fn upsert_edge(&self, edge: WorldEdge) -> RuntimeResult<u64> {
        self.apply_control_command(WorldCommand::UpsertEdge { edge })
    }

    /// Remove one world edge.
    pub fn remove_edge(&self, edge_id: WorldEdgeId) -> RuntimeResult<u64> {
        self.apply_control_command(WorldCommand::RemoveEdge { edge_id })
    }

    /// Apply one internal world control command with optional revision gate.
    fn apply_control_command_inner(
        &self,
        expected_revision: Option<u64>,
        command: WorldCommand,
    ) -> RuntimeResult<u64> {
        // resolve replay state before mutating world state
        let mode = self.replay.mode();
        let command = self.replay.resolve_world_command(command)?;
        let command_for_replay = command.clone();

        // gate one command under the world control lock
        let mut control = self.control.write();
        if let Some(expected_revision) = expected_revision {
            let actual_revision = control.revision;
            if actual_revision != expected_revision {
                return Err(RuntimeError::Internal {
                    message: format!(
                        "world revision mismatch: expected {}, actual {}",
                        expected_revision, actual_revision
                    ),
                }
                .boxed());
            }
        }

        // apply one command directly to world state
        let mut topology = self.topology.write();
        let mut policy_state = self.policy.write();
        let mut resources = self.resources.write();
        Self::apply_control_command_in_place(
            &mut topology,
            &mut policy_state,
            &mut resources,
            command,
        )?;

        // record mode: append command only after successful world mutation
        if mode == ExecutionMode::Record {
            self.replay.record_world_command(&command_for_replay)?;
        }

        control.revision = control.revision.saturating_add(1);

        Ok(control.revision)
    }

    /// Apply one internal world control command directly to world state.
    fn apply_control_command_in_place(
        topology: &mut Topology,
        policy_state: &mut PolicyState,
        resources: &mut BTreeMap<WorldResourceId, WorldResource>,
        command: WorldCommand,
    ) -> RuntimeResult<()> {
        match command {
            WorldCommand::CreateRuntime {
                runtime_id,
                runtime_name,
                runtime_labels,
                primary_agent_id,
                primary_agent_name,
                primary_agent_labels,
            } => {
                topology
                    .add_runtime(
                        runtime_id,
                        runtime_name,
                        runtime_labels,
                        primary_agent_id,
                        primary_agent_name,
                        primary_agent_labels,
                    )
                    .map_err(Self::internal_error)?;
            }

            WorldCommand::CreateAgent {
                runtime_id,
                agent_id,
                agent_name,
                agent_labels,
            } => {
                topology
                    .add_agent(runtime_id, agent_id, agent_name, agent_labels)
                    .map_err(Self::internal_error)?;
            }

            WorldCommand::RemoveAgent { agent_id } => {
                resources.retain(|resource_id, _| resource_id.agent_id != agent_id);
                let _ = topology.remove_agent(agent_id);
            }

            WorldCommand::CreateResource { resource } => {
                topology
                    .attach_resource(
                        resource.id,
                        resource.kind.clone(),
                        resource.label.as_deref(),
                    )
                    .map_err(Self::internal_error)?;
                let _ = resources.insert(resource.id, resource);
            }

            WorldCommand::DestroyResource { resource_id } => {
                let _ = topology.detach_resource(resource_id);
                resources.remove(&resource_id);
            }

            WorldCommand::SetPolicy { policy } => {
                policy_state.set_policy(policy, topology)?;
            }

            WorldCommand::InstallRule { rule } => {
                policy_state.install_rule(rule, topology)?;
            }

            WorldCommand::RemoveRule { rule_id } => {
                policy_state.remove_rule(&rule_id)?;
            }

            WorldCommand::EnableRule { rule_id } => {
                policy_state.enable_rule(&rule_id)?;
            }

            WorldCommand::DisableRule { rule_id } => {
                policy_state.disable_rule(&rule_id)?;
            }

            WorldCommand::ReplaceRule { rule_id, rule } => {
                policy_state.replace_rule(&rule_id, rule, topology)?;
            }

            WorldCommand::DefineEntityKind { kind } => {
                topology
                    .define_entity_kind(kind)
                    .map_err(Self::internal_error)?;
            }

            WorldCommand::DefineEdgeKind { kind } => {
                topology
                    .define_edge_kind(kind)
                    .map_err(Self::internal_error)?;
            }

            WorldCommand::UpsertEntity { entity } => {
                topology
                    .upsert_entity(entity)
                    .map_err(Self::internal_error)?;
            }

            WorldCommand::RemoveEntity { entity_id } => {
                let _ = topology.remove_entity(entity_id.as_str());
            }

            WorldCommand::UpsertEdge { edge } => {
                topology.upsert_edge(edge).map_err(Self::internal_error)?;
            }

            WorldCommand::RemoveEdge { edge_id } => {
                let _ = topology.remove_edge(edge_id.as_str());
            }
        }

        Ok(())
    }

    /// Build one internal runtime error.
    fn internal_error(message: impl fmt::Display) -> Box<RuntimeError> {
        RuntimeError::Internal {
            message: message.to_string(),
        }
        .boxed()
    }
}
