use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::policy::{Policy, PolicyState, Rule, RuleId};
use crate::runtime::{AgentId, WorldResource, WorldResourceId};
use destack_workspace::ExecutionMode;
use std::collections::BTreeMap;
use std::fmt;

use super::topology::Topology;
use super::{
    Input, Mutation, Observation, ObservationCategory, World, WorldEdge, WorldEdgeId,
    WorldEdgeKindDefinition, WorldEntity, WorldEntityId, WorldEntityKindDefinition,
};

impl World {
    /// Apply one resolved world input without touching authoritative trace.
    pub(crate) fn apply_input(&self, input: Input) -> RuntimeResult<()> {
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
    pub(crate) fn mutate(&self, mutation: Mutation) -> RuntimeResult<()> {
        // resolve the requested mutation first
        let mode = self.trace.mode();
        let mutation_summary = format!("{mutation:?}");
        let input = Input::Mutation(mutation);
        let input = self.resolve_input(input)?;

        // apply the mutation before appending it to trace
        self.apply_input(input.clone())?;

        // append the authoritative input only after the mutation succeeds
        if mode == ExecutionMode::Record {
            self.ingest(input)?;
        }

        // emit one derived observation after the authoritative input lands
        self.observe(Observation::world_summary(
            ObservationCategory::Diagnostic,
            "world.control",
            mutation_summary,
        ));

        Ok(())
    }

    /// Apply one mutation directly to live world state.
    pub(crate) fn apply_mutation(&self, mutation: Mutation) -> RuntimeResult<()> {
        // keep structural mutation serialized
        let _activity = self.enter_activity()?;
        let _mutation = self.enter_mutation()?;

        // mutate the topology, policy, and resource state together
        let mut topology = self.topology.borrow_mut();
        let mut policy_state = self.policy.borrow_mut();
        let mut resources = self.resources.borrow_mut();
        commit_mutation(&mut topology, &mut policy_state, &mut resources, mutation)
    }

    /// Remove one agent from the world.
    pub fn remove_agent(&self, agent_id: AgentId) -> RuntimeResult<()> {
        self.mutate(Mutation::RemoveAgent { agent_id })
    }

    /// Create one world resource.
    pub fn create_resource(&self, resource: WorldResource) -> RuntimeResult<()> {
        self.mutate(Mutation::CreateResource { resource })
    }

    /// Destroy one world resource.
    pub fn destroy_resource(&self, resource_id: WorldResourceId) -> RuntimeResult<()> {
        self.mutate(Mutation::DestroyResource { resource_id })
    }

    /// Replace the active policy.
    pub fn set_policy(&self, policy: Policy) -> RuntimeResult<()> {
        self.mutate(Mutation::SetPolicy { policy })
    }

    /// Install one rule into the active policy.
    pub fn install_rule(&self, rule: Rule) -> RuntimeResult<()> {
        self.mutate(Mutation::InstallRule { rule })
    }

    /// Install many rules into the active policy in declaration order.
    pub fn install_rules(&self, rules: impl IntoIterator<Item = Rule>) -> RuntimeResult<()> {
        for rule in rules {
            self.install_rule(rule)?;
        }

        Ok(())
    }

    /// Remove one rule from the active policy.
    pub fn remove_rule(&self, rule_id: RuleId) -> RuntimeResult<()> {
        self.mutate(Mutation::RemoveRule { rule_id })
    }

    /// Enable one rule in the active policy.
    pub fn enable_rule(&self, rule_id: RuleId) -> RuntimeResult<()> {
        self.mutate(Mutation::EnableRule { rule_id })
    }

    /// Disable one rule in the active policy.
    pub fn disable_rule(&self, rule_id: RuleId) -> RuntimeResult<()> {
        self.mutate(Mutation::DisableRule { rule_id })
    }

    /// Replace one installed rule.
    pub fn replace_rule(&self, rule_id: RuleId, rule: Rule) -> RuntimeResult<()> {
        self.mutate(Mutation::ReplaceRule { rule_id, rule })
    }

    /// Define one world entity kind.
    pub fn define_entity_kind(&self, kind: WorldEntityKindDefinition) -> RuntimeResult<()> {
        self.mutate(Mutation::DefineEntityKind { kind })
    }

    /// Define one world edge kind.
    pub fn define_edge_kind(&self, kind: WorldEdgeKindDefinition) -> RuntimeResult<()> {
        self.mutate(Mutation::DefineEdgeKind { kind })
    }

    /// Upsert one world entity.
    pub fn upsert_entity(&self, entity: WorldEntity) -> RuntimeResult<()> {
        self.mutate(Mutation::UpsertEntity { entity })
    }

    /// Remove one world entity.
    pub fn remove_entity(&self, entity_id: WorldEntityId) -> RuntimeResult<()> {
        self.mutate(Mutation::RemoveEntity { entity_id })
    }

    /// Upsert one world edge.
    pub fn upsert_edge(&self, edge: WorldEdge) -> RuntimeResult<()> {
        self.mutate(Mutation::UpsertEdge { edge })
    }

    /// Remove one world edge.
    pub fn remove_edge(&self, edge_id: WorldEdgeId) -> RuntimeResult<()> {
        self.mutate(Mutation::RemoveEdge { edge_id })
    }
}

/// Apply one internal world mutation directly to world state.
fn commit_mutation(
    topology: &mut Topology,
    policy_state: &mut PolicyState,
    resources: &mut BTreeMap<WorldResourceId, WorldResource>,
    mutation: Mutation,
) -> RuntimeResult<()> {
    match mutation {
        Mutation::RemoveAgent { agent_id } => {
            resources.retain(|resource_id, _| resource_id.agent_id != agent_id);
            topology.remove_agent(agent_id);
        }

        Mutation::CreateResource { resource } => {
            topology
                .attach_resource(
                    resource.id,
                    resource.kind.clone(),
                    resource.label.as_deref(),
                )
                .map_err(internal_error)?;
            resources.insert(resource.id, resource);
        }

        Mutation::DestroyResource { resource_id } => {
            topology.detach_resource(resource_id);
            resources.remove(&resource_id);
        }

        Mutation::SetPolicy { policy } => {
            policy_state.set_policy(policy, topology)?;
        }

        Mutation::InstallRule { rule } => {
            policy_state.install_rule(rule, topology)?;
        }

        Mutation::RemoveRule { rule_id } => {
            policy_state.remove_rule(&rule_id)?;
        }

        Mutation::EnableRule { rule_id } => {
            policy_state.enable_rule(&rule_id)?;
        }

        Mutation::DisableRule { rule_id } => {
            policy_state.disable_rule(&rule_id)?;
        }

        Mutation::ReplaceRule { rule_id, rule } => {
            policy_state.replace_rule(&rule_id, rule, topology)?;
        }

        Mutation::DefineEntityKind { kind } => {
            topology.define_entity_kind(kind).map_err(internal_error)?;
        }

        Mutation::DefineEdgeKind { kind } => {
            topology.define_edge_kind(kind).map_err(internal_error)?;
        }

        Mutation::UpsertEntity { entity } => {
            topology.upsert_entity(entity).map_err(internal_error)?;
        }

        Mutation::RemoveEntity { entity_id } => {
            topology.remove_entity(entity_id.as_str());
        }

        Mutation::UpsertEdge { edge } => {
            topology.upsert_edge(edge).map_err(internal_error)?;
        }

        Mutation::RemoveEdge { edge_id } => {
            topology.remove_edge(edge_id.as_str());
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
