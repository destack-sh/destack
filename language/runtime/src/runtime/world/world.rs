use std::collections::BTreeMap;
use std::sync::Arc;

use parking_lot::{RwLock, RwLockReadGuard};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::AgentId;
use crate::runtime::bindings::{BindingDescriptor, BindingEngine, BindingReplayPayload};
use crate::runtime::policy::{
    BindingDispatchDecision, HookEvent, Policy, PolicyDecision, PolicyState, Rule, RuleId,
    RuleSubject,
};
use crate::runtime::random::Random;
use crate::runtime::replay::{ReplayController, ReplayHeader};
use crate::runtime::time::{Clock, HostClockSource};
use crate::simulation::Simulation;
use destack_workspace::{
    ExecutionMode, RandomMode, ReplayPayloadMode, RuntimeAccess, RuntimeOptions, RuntimeWorld,
    TimeMode,
};

use super::topology::Topology;
pub use super::topology::{
    RuntimeId, WorldEdge, WorldEdgeId, WorldEdgeKind, WorldEdgeKindDefinition, WorldEntity,
    WorldEntityId, WorldEntityKind, WorldEntityKindDefinition,
};
use super::{WorldCommand, WorldResource, WorldResourceId};

/// Number of bytes in a megabyte for replay chunk sizing.
const BYTES_PER_MB: u64 = 1024 * 1024;

/// Shared deterministic runtime world.
#[derive(Debug)]
pub struct World {
    /// Shared simulation state for all agents using this world.
    simulation: RwLock<Simulation>,
    /// Shared world clock.
    clock: Clock,
    /// Shared world randomness state.
    random: Random,
    /// Replay controller for deterministic world event history.
    replay: ReplayController,
    /// Active policy state.
    policy: RwLock<PolicyState>,
    /// Topology registry for runtime and simulation identity.
    topology: RwLock<Topology>,
    /// Logical resource records keyed by world resource identifier.
    resources: RwLock<BTreeMap<WorldResourceId, WorldResource>>,
}

impl World {
    /// Create one world from runtime options and optional host clock source.
    pub(crate) fn new(
        options: &RuntimeOptions,
        host_clock_source: Option<Arc<dyn HostClockSource>>,
    ) -> RuntimeResult<Self> {
        // execution mode: replay forces virtual time and deterministic random
        let is_replay = options.execution == ExecutionMode::Replay;
        let time_mode = if is_replay {
            TimeMode::Virtual
        } else {
            options.time.mode
        };
        let random_mode = if is_replay {
            RandomMode::Deterministic
        } else {
            options.random.mode
        };
        let replay_payload = match options.replay.payload {
            ReplayPayloadMode::ResultsOnly => BindingReplayPayload::Results,
            ReplayPayloadMode::ArgumentsAndResults => BindingReplayPayload::ArgumentsAndResults,
        };

        // replay header: options with chunk-size override
        let mut replay_header = ReplayHeader {
            execution_mode: options.execution,
            replay_payload,
            ..ReplayHeader::default()
        };
        if let Some(chunk_size_mb) = options.replay.chunk_size_mb {
            let chunk_bytes = chunk_size_mb.saturating_mul(BYTES_PER_MB);
            if chunk_bytes > 0 {
                replay_header.max_chunk_bytes = chunk_bytes;
            }
        }

        // world state
        let clock = if let Some(host_clock_source) = host_clock_source {
            Clock::from_mode_and_options_with_host_clock_source(
                time_mode,
                &options.time,
                host_clock_source,
            )
        } else {
            Clock::from_mode_and_options(time_mode, &options.time)
        };
        let random = Random::new(options.random.seed.unwrap_or(0), random_mode);
        let policy = Policy::from_workspace_rules(&options.rules);
        let replay = ReplayController::new(options.execution, replay_payload, replay_header);
        let topology = Topology::with_builtin_kinds()
            .map_err(|message| RuntimeError::Internal { message }.boxed())?;
        policy.validate_with_kind_catalog(&topology)?;

        // final world state
        Ok(Self {
            simulation: RwLock::new(Simulation::default()),
            clock,
            random,
            replay,
            policy: RwLock::new(PolicyState::new(policy)),
            topology: RwLock::new(topology),
            resources: RwLock::new(BTreeMap::new()),
        })
    }

    /// Borrow one read guard for simulation.
    pub fn read_simulation(&self) -> RwLockReadGuard<'_, Simulation> {
        self.simulation.read()
    }

    /// Snapshot world policy state.
    pub fn policy(&self) -> Policy {
        self.policy.read().spec.clone()
    }

    /// Snapshot world entity kind definitions.
    pub fn entity_kinds(&self) -> BTreeMap<WorldEntityKind, WorldEntityKindDefinition> {
        self.topology.read().entity_kinds().clone()
    }

    /// Snapshot world edge kind definitions.
    pub fn edge_kinds(&self) -> BTreeMap<WorldEdgeKind, WorldEdgeKindDefinition> {
        self.topology.read().edge_kinds().clone()
    }

    /// Snapshot world entities.
    pub fn entities(&self) -> BTreeMap<WorldEntityId, WorldEntity> {
        self.topology.read().entities().clone()
    }

    /// Snapshot world edges.
    pub fn edges(&self) -> BTreeMap<WorldEdgeId, WorldEdge> {
        self.topology.read().edges().clone()
    }

    /// Snapshot logical world resources.
    pub fn resources(&self) -> BTreeMap<WorldResourceId, WorldResource> {
        self.resources.read().clone()
    }

    /// Return the current world command revision.
    pub fn revision(&self) -> u64 {
        self.topology.read().control_revision()
    }

    /// Allocate one runtime identifier.
    pub(crate) fn allocate_runtime_id(&self) -> RuntimeId {
        self.topology.write().allocate_runtime_id()
    }

    /// Allocate one agent identifier.
    pub(crate) fn allocate_agent_id(&self) -> AgentId {
        self.topology.write().allocate_agent_id()
    }

    /// Borrow the shared world clock.
    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    /// Borrow the shared world randomness state.
    pub fn random(&self) -> &Random {
        &self.random
    }

    /// Borrow the shared replay controller.
    pub fn replay(&self) -> &ReplayController {
        &self.replay
    }

    /// Apply one world command and return the new world revision.
    pub fn apply(&self, command: WorldCommand) -> RuntimeResult<u64> {
        self.apply_command(command, None)
    }

    /// Apply one world command at an expected revision and return the new revision.
    pub fn apply_at_revision(
        &self,
        expected_revision: u64,
        command: WorldCommand,
    ) -> RuntimeResult<u64> {
        self.apply_command(command, Some(expected_revision))
    }

    /// Create one runtime and its primary agent.
    pub fn create_runtime(
        &self,
        runtime_id: RuntimeId,
        runtime_name: String,
        runtime_labels: BTreeMap<String, String>,
        primary_agent_id: AgentId,
        primary_agent_name: String,
        primary_agent_labels: BTreeMap<String, String>,
    ) -> RuntimeResult<u64> {
        self.apply(WorldCommand::CreateRuntime {
            runtime_id,
            runtime_name,
            runtime_labels,
            primary_agent_id,
            primary_agent_name,
            primary_agent_labels,
        })
    }

    /// Create one agent in one existing runtime.
    pub fn create_agent(
        &self,
        runtime_id: RuntimeId,
        agent_id: AgentId,
        agent_name: String,
        agent_labels: BTreeMap<String, String>,
    ) -> RuntimeResult<u64> {
        self.apply(WorldCommand::CreateAgent {
            runtime_id,
            agent_id,
            agent_name,
            agent_labels,
        })
    }

    /// Remove one agent from the world.
    pub fn remove_agent(&self, agent_id: AgentId) -> RuntimeResult<u64> {
        self.apply(WorldCommand::RemoveAgent { agent_id })
    }

    /// Create one world resource.
    pub fn create_resource(&self, resource: WorldResource) -> RuntimeResult<u64> {
        self.apply(WorldCommand::CreateResource { resource })
    }

    /// Destroy one world resource.
    pub fn destroy_resource(&self, resource_id: WorldResourceId) -> RuntimeResult<u64> {
        self.apply(WorldCommand::DestroyResource { resource_id })
    }

    /// Replace the active policy.
    pub fn set_policy(&self, policy: Policy) -> RuntimeResult<u64> {
        self.apply(WorldCommand::SetPolicy { policy })
    }

    /// Install one rule into the active policy.
    pub fn install_rule(&self, rule: Rule) -> RuntimeResult<u64> {
        self.apply(WorldCommand::InstallRule { rule })
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
        self.apply(WorldCommand::RemoveRule { rule_id })
    }

    /// Enable one rule in the active policy.
    pub fn enable_rule(&self, rule_id: RuleId) -> RuntimeResult<u64> {
        self.apply(WorldCommand::EnableRule { rule_id })
    }

    /// Disable one rule in the active policy.
    pub fn disable_rule(&self, rule_id: RuleId) -> RuntimeResult<u64> {
        self.apply(WorldCommand::DisableRule { rule_id })
    }

    /// Replace one installed rule.
    pub fn replace_rule(&self, rule_id: RuleId, rule: Rule) -> RuntimeResult<u64> {
        self.apply(WorldCommand::ReplaceRule { rule_id, rule })
    }

    /// Define one world entity kind.
    pub fn define_entity_kind(&self, kind: WorldEntityKindDefinition) -> RuntimeResult<u64> {
        self.apply(WorldCommand::DefineEntityKind { kind })
    }

    /// Define one world edge kind.
    pub fn define_edge_kind(&self, kind: WorldEdgeKindDefinition) -> RuntimeResult<u64> {
        self.apply(WorldCommand::DefineEdgeKind { kind })
    }

    /// Upsert one world entity.
    pub fn upsert_entity(&self, entity: WorldEntity) -> RuntimeResult<u64> {
        self.apply(WorldCommand::UpsertEntity { entity })
    }

    /// Remove one world entity.
    pub fn remove_entity(&self, entity_id: WorldEntityId) -> RuntimeResult<u64> {
        self.apply(WorldCommand::RemoveEntity { entity_id })
    }

    /// Upsert one world edge.
    pub fn upsert_edge(&self, edge: WorldEdge) -> RuntimeResult<u64> {
        self.apply(WorldCommand::UpsertEdge { edge })
    }

    /// Remove one world edge.
    pub fn remove_edge(&self, edge_id: WorldEdgeId) -> RuntimeResult<u64> {
        self.apply(WorldCommand::RemoveEdge { edge_id })
    }

    /// Apply one world command with optional revision gate.
    fn apply_command(
        &self,
        command: WorldCommand,
        expected_revision: Option<u64>,
    ) -> RuntimeResult<u64> {
        // resolve replay state before mutating world state
        let mode = self.replay.mode();
        let command = self.replay.resolve_world_command(command)?;
        let command_for_replay = command.clone();

        // apply one command directly to world state
        let mut topology = self.topology.write();
        if let Some(expected_revision) = expected_revision {
            let actual_revision = topology.control_revision();
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

        let mut policy_state = self.policy.write();
        let mut resources = self.resources.write();
        Self::apply_command_in_place(&mut topology, &mut policy_state, &mut resources, command)?;

        // record mode: append command only after successful world mutation
        if mode == ExecutionMode::Record {
            self.replay.record_world_command(&command_for_replay)?;
        }

        topology.bump_control_revision();

        Ok(topology.control_revision())
    }

    /// Apply one command directly to world state.
    fn apply_command_in_place(
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
                    .register_runtime(runtime_id, runtime_name, runtime_labels)
                    .map_err(Self::internal_error)?;
                topology
                    .register_agent(
                        runtime_id,
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
                    .register_agent(runtime_id, agent_id, agent_name, agent_labels)
                    .map_err(Self::internal_error)?;
            }
            WorldCommand::RemoveAgent { agent_id } => {
                let _ = topology.deregister_agent(agent_id);
            }
            WorldCommand::CreateResource { resource } => {
                let previous_resource = resources.insert(resource.id, resource.clone());
                let create_result = topology.create_resource_for_agent(
                    resource.id.agent_id,
                    resource.id.resource_id,
                    resource.kind.as_str(),
                    resource.label.as_deref(),
                );
                if let Err(error) = create_result {
                    if let Some(previous_resource) = previous_resource {
                        resources.insert(previous_resource.id, previous_resource);
                    } else {
                        resources.remove(&resource.id);
                    }
                    return Err(Self::internal_error(error));
                }
            }
            WorldCommand::DestroyResource { resource_id } => {
                resources.remove(&resource_id);
                let _ = topology
                    .destroy_resource_for_agent(resource_id.agent_id, resource_id.resource_id);
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
    fn internal_error(message: impl Into<String>) -> Box<RuntimeError> {
        RuntimeError::Internal {
            message: message.into(),
        }
        .boxed()
    }

    /// Resolve binding dispatch decisions for one binding call.
    pub(crate) fn resolve_binding_dispatch(
        &self,
        mode: ExecutionMode,
        runtime_id: RuntimeId,
        agent_id: AgentId,
        descriptor: BindingDescriptor,
        engine: Option<BindingEngine>,
        default_access: RuntimeAccess,
        default_world: RuntimeWorld,
        default_replay_payload: BindingReplayPayload,
    ) -> RuntimeResult<BindingDispatchDecision> {
        let topology = self.topology.read();
        let subject = Self::resolve_rule_subject(&topology, runtime_id, agent_id, mode)?;

        // evaluate dispatch decision against active policy
        let decision = self.policy.read().resolve_binding_dispatch_for_subject(
            subject,
            descriptor,
            engine,
            default_access,
            default_world,
            default_replay_payload,
        );

        Ok(decision)
    }

    /// Evaluate one policy event for one agent under one world lock.
    pub(crate) fn evaluate_policy_event(
        &self,
        mode: ExecutionMode,
        runtime_id: RuntimeId,
        agent_id: AgentId,
        event: &HookEvent,
    ) -> RuntimeResult<Vec<PolicyDecision>> {
        let topology = self.topology.read();
        let subject = Self::resolve_rule_subject(&topology, runtime_id, agent_id, mode)?;

        // evaluate one policy event with world-randomness context
        let mut policy = self.policy.write();
        let decisions = policy.on_event_for_subject(event, subject, &self.random);

        Ok(decisions)
    }

    /// Resolve one rule subject from runtime and agent world topology identities.
    fn resolve_rule_subject<'a>(
        topology: &'a Topology,
        runtime_id: RuntimeId,
        agent_id: AgentId,
        mode: ExecutionMode,
    ) -> RuntimeResult<RuleSubject<'a>> {
        let (runtime_name, runtime_labels) =
            topology.runtime_identity(runtime_id).ok_or_else(|| {
                RuntimeError::Internal {
                    message: format!(
                        "runtime {} is not registered or missing selector labels in world topology",
                        runtime_id.0
                    ),
                }
                .boxed()
            })?;
        let (agent_name, agent_labels) = topology.agent_identity(agent_id).ok_or_else(|| {
            RuntimeError::Internal {
                message: format!(
                    "agent {} is not registered or missing selector labels in world topology",
                    agent_id.0
                ),
            }
            .boxed()
        })?;

        Ok(RuleSubject::new(
            runtime_name,
            runtime_labels,
            agent_name,
            agent_labels,
            mode,
        ))
    }
}

impl Default for World {
    /// Create a world with empty policy rules.
    fn default() -> Self {
        let options = RuntimeOptions::default();
        match Self::new(&options, None) {
            Ok(world) => world,
            Err(error) => {
                panic!("default world construction should succeed: {error:?}");
            }
        }
    }
}
