use super::{
    Input, Mutation, World, WorldEdge, WorldEdgeId, WorldEdgeKindDefinition, WorldEntity,
    WorldEntityId, WorldEntityKindDefinition,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::policy::{Policy, Rule, RuleId};
use crate::runtime::{AgentId, WorldResource, WorldResourceId};
use destack_workspace::ExecutionMode;
use std::fmt;

impl World {
    /// Apply one resolved world input without touching authoritative trace.
    pub(crate) fn apply_input(&mut self, input: Input) -> RuntimeResult<()> {
        match input {
            // scheduler progress
            Input::Tick => {
                let _ = self.tick_inner()?;
            }

            // runtime lifecycle
            Input::RemoveRuntime { runtime_id } => {
                let _ = self.remove_runtime_inner(runtime_id)?;
            }

            // entry execution
            Input::RunEntrypoint {
                runtime_id,
                entry,
                args,
            } => {
                let _ = self.run_replayable_entrypoint_inner(runtime_id, &entry, &args)?;
            }

            // structural mutation
            Input::Mutation(mutation) => {
                self.apply_mutation(mutation)?;
            }
        }

        Ok(())
    }

    /// Apply one world mutation through the authoritative input boundary.
    pub(crate) fn mutate(&mut self, mutation: Mutation) -> RuntimeResult<()> {
        // resolve the requested mutation first
        let mode = self.trace.mode();
        let input = Input::Mutation(mutation);
        let input = self.resolve_input(input)?;

        // apply the mutation before appending it to trace
        self.apply_input(input.clone())?;

        // append the authoritative input only after the mutation succeeds
        if mode == ExecutionMode::Record {
            self.ingest(input)?;
        }

        Ok(())
    }

    /// Apply one mutation directly to live world state.
    pub(crate) fn apply_mutation(&mut self, mutation: Mutation) -> RuntimeResult<()> {
        self.apply_world_mutation(mutation)
    }

    /// Remove one agent from the world.
    pub fn remove_agent(&mut self, agent_id: AgentId) -> RuntimeResult<()> {
        self.mutate(Mutation::RemoveAgent { agent_id })
    }

    /// Create one world resource.
    pub fn create_resource(&mut self, resource: WorldResource) -> RuntimeResult<()> {
        self.mutate(Mutation::CreateResource { resource })
    }

    /// Destroy one world resource.
    pub fn destroy_resource(&mut self, resource_id: WorldResourceId) -> RuntimeResult<()> {
        self.mutate(Mutation::DestroyResource { resource_id })
    }

    /// Replace the active policy.
    pub fn set_policy(&mut self, policy: Policy) -> RuntimeResult<()> {
        self.mutate(Mutation::SetPolicy { policy })
    }

    /// Install one rule into the active policy.
    pub fn install_rule(&mut self, rule: Rule) -> RuntimeResult<()> {
        self.mutate(Mutation::InstallRule { rule })
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

    /// Replace one installed rule.
    pub fn replace_rule(&mut self, rule_id: RuleId, rule: Rule) -> RuntimeResult<()> {
        self.mutate(Mutation::ReplaceRule { rule_id, rule })
    }

    /// Define one world entity kind.
    pub fn define_entity_kind(&mut self, kind: WorldEntityKindDefinition) -> RuntimeResult<()> {
        self.mutate(Mutation::DefineEntityKind { kind })
    }

    /// Define one world edge kind.
    pub fn define_edge_kind(&mut self, kind: WorldEdgeKindDefinition) -> RuntimeResult<()> {
        self.mutate(Mutation::DefineEdgeKind { kind })
    }

    /// Upsert one world entity.
    pub fn upsert_entity(&mut self, entity: WorldEntity) -> RuntimeResult<()> {
        self.mutate(Mutation::UpsertEntity { entity })
    }

    /// Remove one world entity.
    pub fn remove_entity(&mut self, entity_id: WorldEntityId) -> RuntimeResult<()> {
        self.mutate(Mutation::RemoveEntity { entity_id })
    }

    /// Upsert one world edge.
    pub fn upsert_edge(&mut self, edge: WorldEdge) -> RuntimeResult<()> {
        self.mutate(Mutation::UpsertEdge { edge })
    }

    /// Remove one world edge.
    pub fn remove_edge(&mut self, edge_id: WorldEdgeId) -> RuntimeResult<()> {
        self.mutate(Mutation::RemoveEdge { edge_id })
    }

    /// Apply one internal world mutation directly to live state.
    fn apply_world_mutation(&mut self, mutation: Mutation) -> RuntimeResult<()> {
        match mutation {
            Mutation::RemoveAgent { agent_id } => {
                self.resources
                    .retain(|resource_id, _| resource_id.agent_id != agent_id);
                self.topology.remove_agent(agent_id);
            }

            Mutation::CreateResource { resource } => {
                self.topology
                    .attach_resource(
                        resource.id,
                        resource.kind.clone(),
                        resource.label.as_deref(),
                    )
                    .map_err(Self::internal_error)?;
                self.resources.insert(resource.id, resource);
            }

            Mutation::DestroyResource { resource_id } => {
                self.topology.detach_resource(resource_id);
                self.resources.remove(&resource_id);
            }

            Mutation::SetPolicy { policy } => {
                self.policy.set_policy(policy, &self.topology)?;
            }

            Mutation::InstallRule { rule } => {
                self.policy.install_rule(rule, &self.topology)?;
            }

            Mutation::RemoveRule { rule_id } => {
                self.policy.remove_rule(&rule_id)?;
            }

            Mutation::EnableRule { rule_id } => {
                self.policy.enable_rule(&rule_id)?;
            }

            Mutation::DisableRule { rule_id } => {
                self.policy.disable_rule(&rule_id)?;
            }

            Mutation::ReplaceRule { rule_id, rule } => {
                self.policy.replace_rule(&rule_id, rule, &self.topology)?;
            }

            Mutation::DefineEntityKind { kind } => {
                self.topology
                    .define_entity_kind(kind)
                    .map_err(Self::internal_error)?;
            }

            Mutation::DefineEdgeKind { kind } => {
                self.topology
                    .define_edge_kind(kind)
                    .map_err(Self::internal_error)?;
            }

            Mutation::UpsertEntity { entity } => {
                self.topology
                    .upsert_entity(entity)
                    .map_err(Self::internal_error)?;
            }

            Mutation::RemoveEntity { entity_id } => {
                self.topology.remove_entity(entity_id.as_str());
            }

            Mutation::UpsertEdge { edge } => {
                self.topology
                    .upsert_edge(edge)
                    .map_err(Self::internal_error)?;
            }

            Mutation::RemoveEdge { edge_id } => {
                self.topology.remove_edge(edge_id.as_str());
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
