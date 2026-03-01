use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::runtime::policy::{InstalledPolicy, Policy, TriggerScopeKey};
use crate::runtime::random::Random;
use crate::runtime::time::Clock;
use crate::simulation::Simulation;

/// Global world id sequence for stable world identity.
static NEXT_WORLD_ID: AtomicU64 = AtomicU64::new(1);

/// Stable world identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorldId(pub u64);

/// Shared deterministic runtime world.
#[derive(Debug, Clone)]
pub struct World {
    /// Shared world state.
    world: Arc<WorldState>,
}

/// Mutable and shared world state.
#[derive(Debug)]
struct WorldState {
    /// Stable world identity.
    id: WorldId,
    /// Shared simulation state for all agents using this world.
    simulation: RwLock<Simulation>,
    /// Shared world clock.
    clock: Clock,
    /// Shared world randomness state.
    random: Random,
    /// Installed policy plus world-local runtime state.
    policy: RwLock<InstalledPolicy>,
}

impl World {
    /// Create a world with empty policy rules.
    pub fn new() -> Self {
        Self::new_with_state(Policy::default(), Clock::default(), Random::default())
    }

    /// Create a world for runtime bootstrap domains.
    pub(crate) fn for_runtime(policy: Policy, clock: Clock, random: Random) -> Self {
        Self::new_with_state(policy, clock, random)
    }

    /// Create a world with explicit policy and domain state.
    fn new_with_state(policy: Policy, clock: Clock, random: Random) -> Self {
        let id = WorldId(NEXT_WORLD_ID.fetch_add(1, Ordering::Relaxed));
        let state = WorldState {
            id,
            simulation: RwLock::new(Simulation::default()),
            clock,
            random,
            policy: RwLock::new(InstalledPolicy::new(policy)),
        };

        Self {
            world: Arc::new(state),
        }
    }

    /// Return this world identifier.
    pub fn id(&self) -> WorldId {
        self.world.id
    }

    /// Borrow one read guard for simulation.
    pub fn read_simulation(&self) -> RwLockReadGuard<'_, Simulation> {
        self.world.simulation.read()
    }

    /// Borrow one write guard for simulation.
    pub fn write_simulation(&self) -> RwLockWriteGuard<'_, Simulation> {
        self.world.simulation.write()
    }

    /// Snapshot the simulation.
    pub fn simulation(&self) -> Simulation {
        self.world.simulation.read().clone()
    }

    /// Borrow the shared world clock.
    pub fn clock(&self) -> &Clock {
        &self.world.clock
    }

    /// Borrow the shared world randomness state.
    pub fn random(&self) -> &Random {
        &self.world.random
    }

    /// Snapshot the active policy.
    pub fn policy(&self) -> Policy {
        self.world.policy.read().policy.clone()
    }

    /// Return the active policy revision.
    pub fn policy_revision(&self) -> u64 {
        self.world.policy.read().revision
    }

    /// Replace the active policy.
    pub fn set_policy(&self, policy: Policy) {
        self.world.policy.write().replace_policy(policy);
    }

    /// Ensure trigger counters are initialized for one active policy revision.
    pub fn prepare_policy_trigger_state(&self, policy_revision: u64, rule_count: usize) {
        self.world
            .policy
            .write()
            .prepare_trigger_state_for_revision(policy_revision, rule_count);
    }

    /// Record one policy-matched call in world-global scope.
    pub fn record_policy_call(&self) {
        self.world.policy.write().record_call();
    }

    /// Record matched effect rule indexes for one agent scope.
    pub fn record_policy_matches_for_agent(&self, agent_id: u64, indexes: &[usize]) {
        self.world
            .policy
            .write()
            .record_matches(TriggerScopeKey::Agent(agent_id), indexes);
    }

    /// Snapshot matched effect counters for the active policy revision.
    #[cfg(test)]
    pub(crate) fn policy_matched_effects_seen(&self) -> Vec<u64> {
        self.world.policy.read().matched_effects_seen_totals()
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}
