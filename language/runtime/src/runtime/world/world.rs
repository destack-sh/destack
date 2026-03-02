use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::diagnostic::RuntimeResult;
use crate::runtime::AgentId;
use crate::runtime::bindings::{BindingDescriptor, BindingEngine, BindingReplayPayload};
use crate::runtime::policy::{
    HookEvent, Policy, PolicyCommand, PolicyDecision, PolicyIdentity, PolicyState,
};
use crate::runtime::random::Random;
use crate::runtime::replay::ReplayController;
use crate::runtime::time::Clock;
use crate::simulation::Simulation;
use destack_workspace::{ExecutionMode, RuntimeAccess, RuntimeWorld};

/// Shared deterministic runtime world.
#[derive(Debug)]
pub struct World {
    /// Shared simulation state for all agents using this world.
    simulation: RwLock<Simulation>,
    /// Next world-scoped agent identifier.
    next_agent_id: AtomicU64,
    /// Shared world clock.
    clock: Clock,
    /// Shared world randomness state.
    random: Random,
    /// Replay controller for deterministic world event history.
    replay: ReplayController,
    /// Control plane state.
    policy: RwLock<PolicyState>,
}

impl World {
    /// Create a world for runtime bootstrap domains.
    pub(crate) fn for_runtime(
        policy: Policy,
        clock: Clock,
        random: Random,
        replay: ReplayController,
    ) -> Self {
        Self::new(policy, clock, random, replay)
    }

    /// Create a world with explicit policy and domain state.
    fn new(policy: Policy, clock: Clock, random: Random, replay: ReplayController) -> Self {
        Self {
            simulation: RwLock::new(Simulation::default()),
            next_agent_id: AtomicU64::new(1),
            clock,
            random,
            replay,
            policy: RwLock::new(PolicyState::new(policy)),
        }
    }

    /// Allocate one world-scoped agent identifier.
    pub(crate) fn allocate_agent_id(&self) -> AgentId {
        AgentId(self.next_agent_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Borrow one read guard for simulation.
    pub fn read_simulation(&self) -> RwLockReadGuard<'_, Simulation> {
        self.simulation.read()
    }

    /// Borrow one write guard for simulation.
    pub fn write_simulation(&self) -> RwLockWriteGuard<'_, Simulation> {
        self.simulation.write()
    }

    /// Snapshot the simulation.
    pub fn simulation(&self) -> Simulation {
        self.simulation.read().clone()
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

    /// Replace the active policy.
    pub fn set_policy(&self, policy: Policy) -> RuntimeResult<()> {
        self.policy.write().set_policy(policy)
    }

    /// Apply one policy command to the active world policy.
    pub fn apply_policy_command(&self, command: PolicyCommand) -> RuntimeResult<()> {
        self.policy.write().apply_policy_command(command)
    }

    /// Resolve one access decision for one binding call.
    pub(crate) fn resolve_binding_access(
        &self,
        mode: ExecutionMode,
        identity: &PolicyIdentity,
        descriptor: BindingDescriptor,
        engine: Option<BindingEngine>,
        default_access: RuntimeAccess,
    ) -> RuntimeAccess {
        self.policy.read().resolve_binding_access(
            identity,
            mode,
            descriptor,
            engine,
            default_access,
        )
    }

    /// Resolve one world decision for one binding call.
    pub(crate) fn resolve_binding_world(
        &self,
        mode: ExecutionMode,
        identity: &PolicyIdentity,
        descriptor: BindingDescriptor,
        engine: Option<BindingEngine>,
        default_world: RuntimeWorld,
    ) -> RuntimeWorld {
        self.policy
            .read()
            .resolve_binding_world(identity, mode, descriptor, engine, default_world)
    }

    /// Resolve one replay payload decision for one binding call.
    pub(crate) fn resolve_binding_replay_payload(
        &self,
        mode: ExecutionMode,
        identity: &PolicyIdentity,
        descriptor: BindingDescriptor,
        engine: Option<BindingEngine>,
        default_replay_payload: BindingReplayPayload,
    ) -> BindingReplayPayload {
        self.policy.read().resolve_binding_replay_payload(
            identity,
            mode,
            descriptor,
            engine,
            default_replay_payload,
        )
    }

    /// Evaluate one policy event for one agent under one world lock.
    pub(crate) fn evaluate_policy_event(
        &self,
        mode: ExecutionMode,
        identity: &PolicyIdentity,
        event: &HookEvent,
    ) -> Vec<PolicyDecision> {
        let mut policy = self.policy.write();

        policy.on_event(event, identity, mode, &self.random)
    }

    /// Snapshot matched effect counters for the active policy revision.
    #[cfg(test)]
    pub(crate) fn policy_matched_effects_seen(&self) -> Vec<u64> {
        self.policy.read().matched_decisions_seen_totals()
    }
}

impl Default for World {
    /// Create a world with empty policy rules.
    fn default() -> Self {
        Self::new(
            Policy::default(),
            Clock::default(),
            Random::default(),
            ReplayController::default(),
        )
    }
}
