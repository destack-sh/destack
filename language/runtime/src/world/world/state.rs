use std::collections::BTreeMap;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::random::Random;
use crate::runtime::time::{Clock, Instant, Nanos};
use crate::runtime::{RuntimeId, WorkerId};
use crate::simulation::Simulation;
use crate::world::policy::PolicyState;
use crate::world::scenario::{FaultRule, FaultRuleId, Scenario, ScenarioId};
use crate::world::trace::{Observation, ObservationSequence, Observations, Trace};

use crate::host::ResourceId;

use super::{BranchId, Resource, Topology};

/// Shared world state used by runtimes and workers.
#[derive(Debug)]
pub(crate) struct WorldState {
    /// Active branch identifier for this live world.
    pub(crate) branch_id: BranchId,
    /// Shared simulation state for all runtimes in this world.
    pub(crate) simulation: Simulation,
    /// Active policy state.
    pub(crate) policy: PolicyState,
    /// Active scenario scripts.
    pub(crate) scenarios: Vec<Scenario>,
    /// The next scenario id to allocate.
    pub(crate) next_scenario_id: u64,
    /// The next runtime id to allocate.
    pub(crate) next_runtime_id: u64,
    /// The next worker id to allocate.
    pub(crate) next_worker_id: u64,
    /// Topology registry for world metadata.
    pub(crate) topology: Topology,
    /// Resource records keyed by resource identifier.
    pub(crate) resources: BTreeMap<ResourceId, Resource>,
    /// Shared world clock.
    pub(crate) clock: Clock,
    /// Shared world randomness state.
    pub(crate) random: Random,
    /// Trace of world events.
    pub(crate) trace: Trace,
    /// Emitted observations.
    pub(crate) observations: Observations,
}

impl WorldState {
    /// Borrow the live simulation state.
    #[inline]
    pub(crate) fn simulation(&self) -> &Simulation {
        &self.simulation
    }

    /// Borrow the live policy state.
    #[inline]
    pub(crate) fn policy(&self) -> &PolicyState {
        &self.policy
    }

    /// Borrow the live topology.
    #[inline]
    pub(crate) fn topology(&self) -> &Topology {
        &self.topology
    }

    /// Register one runtime in world topology.
    pub(crate) fn register_runtime_topology(
        &mut self,
        runtime_id: RuntimeId,
        runtime_name: String,
        runtime_labels: BTreeMap<String, String>,
    ) -> RuntimeResult<()> {
        let result = self
            .topology
            .add_runtime(runtime_id, runtime_name, runtime_labels);

        result.map_err(|message| {
            RuntimeError::Internal {
                message: message.to_string(),
            }
            .boxed()
        })
    }

    /// Register one worker in one existing runtime.
    pub(crate) fn register_worker_topology(
        &mut self,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        worker_name: String,
        worker_labels: BTreeMap<String, String>,
    ) -> RuntimeResult<()> {
        let result = self
            .topology
            .add_worker(runtime_id, worker_id, worker_name, worker_labels);

        result.map_err(|message| {
            RuntimeError::Internal {
                message: message.to_string(),
            }
            .boxed()
        })
    }

    /// Attach one resource to the world topology and resource table.
    pub(crate) fn attach_resource(&mut self, resource: Resource) -> RuntimeResult<()> {
        let result = self.topology.attach_resource(&resource);
        result.map_err(|message| {
            RuntimeError::Internal {
                message: message.to_string(),
            }
            .boxed()
        })?;

        self.resources.insert(resource.id, resource);

        Ok(())
    }

    /// Detach one resource from the world topology and resource table.
    pub(crate) fn detach_resource(&mut self, resource_id: ResourceId) {
        let resource = self.resources.remove(&resource_id);
        if let Some(resource) = resource {
            self.topology.detach_resource(&resource);
        }
    }

    /// Borrow the shared world clock.
    #[inline]
    pub(crate) fn clock(&self) -> &Clock {
        &self.clock
    }

    /// Borrow the shared world randomness state.
    #[inline]
    pub(crate) fn random(&self) -> &Random {
        &self.random
    }

    /// Borrow the shared trace controller.
    #[inline]
    pub(crate) fn trace(&self) -> &Trace {
        &self.trace
    }

    /// Borrow the emitted observations.
    #[inline]
    pub(crate) fn observations(&self) -> &Observations {
        &self.observations
    }

    /// Return the current live execution coordinate.
    pub(crate) fn moment(&self) -> super::Moment {
        super::Moment::new(self.branch_id, self.trace().log().next_sequence())
    }

    /// Emit one observation at the current execution coordinate.
    pub(crate) fn observe(&self, observation: Observation) -> ObservationSequence {
        self.observations().record_at(self.moment(), observation)
    }

    /// Allocate one runtime identifier.
    pub(crate) fn allocate_runtime_id(&mut self) -> RuntimeId {
        let runtime_id = self.next_runtime_id;
        self.next_runtime_id = runtime_id + 1;

        RuntimeId(runtime_id)
    }

    /// Allocate one worker identifier.
    pub(crate) fn allocate_worker_id(&mut self) -> WorkerId {
        let worker_id = self.next_worker_id;
        self.next_worker_id = worker_id + 1;

        WorkerId(worker_id)
    }

    /// Return the next scenario identifier without consuming it.
    pub(crate) fn next_scenario_id(&self) -> ScenarioId {
        ScenarioId(self.next_scenario_id)
    }

    /// Add one scenario script.
    pub(crate) fn add_scenario(&mut self, scenario: Scenario) -> RuntimeResult<()> {
        if self
            .scenarios
            .iter()
            .any(|existing| existing.id == scenario.id)
        {
            return Err(Self::scenario_error(format!(
                "runtime scenario requires unique scenario ids: {}",
                scenario.id.0
            )));
        }

        scenario.validate_with_kind_catalog(&self.topology)?;
        self.next_scenario_id = self.next_scenario_id.max(scenario.id.0 + 1);
        self.scenarios.push(scenario);

        Ok(())
    }

    /// Remove one scenario script.
    pub(crate) fn remove_scenario(&mut self, scenario_id: ScenarioId) -> RuntimeResult<()> {
        let before_len = self.scenarios.len();
        self.scenarios.retain(|scenario| scenario.id != scenario_id);
        if self.scenarios.len() < before_len {
            return Ok(());
        }

        Err(Self::unknown_scenario_error(scenario_id))
    }

    /// Enable one scenario script.
    pub(crate) fn enable_scenario(&mut self, scenario_id: ScenarioId) -> RuntimeResult<()> {
        self.set_scenario_enabled(scenario_id, true)
    }

    /// Disable one scenario script.
    pub(crate) fn disable_scenario(&mut self, scenario_id: ScenarioId) -> RuntimeResult<()> {
        self.set_scenario_enabled(scenario_id, false)
    }

    /// Add one rule into one scenario.
    pub(crate) fn add_scenario_rule(
        &mut self,
        scenario_id: ScenarioId,
        rule: FaultRule,
    ) -> RuntimeResult<()> {
        let topology = &self.topology;
        let scenario = Self::scenario_mut(&mut self.scenarios, scenario_id)?;

        scenario.add_rule(rule, topology)
    }

    /// Remove one rule from one scenario.
    pub(crate) fn remove_scenario_rule(
        &mut self,
        scenario_id: ScenarioId,
        rule_id: &FaultRuleId,
    ) -> RuntimeResult<()> {
        Self::scenario_mut(&mut self.scenarios, scenario_id)?.remove_rule(rule_id)
    }

    /// Enable one rule in one scenario.
    pub(crate) fn enable_scenario_rule(
        &mut self,
        scenario_id: ScenarioId,
        rule_id: &FaultRuleId,
    ) -> RuntimeResult<()> {
        Self::scenario_mut(&mut self.scenarios, scenario_id)?.enable_rule(rule_id)
    }

    /// Disable one rule in one scenario.
    pub(crate) fn disable_scenario_rule(
        &mut self,
        scenario_id: ScenarioId,
        rule_id: &FaultRuleId,
    ) -> RuntimeResult<()> {
        Self::scenario_mut(&mut self.scenarios, scenario_id)?.disable_rule(rule_id)
    }

    /// Replace one rule in one scenario.
    pub(crate) fn replace_scenario_rule(
        &mut self,
        scenario_id: ScenarioId,
        rule_id: &FaultRuleId,
        rule: FaultRule,
    ) -> RuntimeResult<()> {
        let topology = &self.topology;
        let scenario = Self::scenario_mut(&mut self.scenarios, scenario_id)?;

        scenario.replace_rule(rule_id, rule, topology)
    }

    /// Set one scenario enabled state.
    fn set_scenario_enabled(
        &mut self,
        scenario_id: ScenarioId,
        is_enabled: bool,
    ) -> RuntimeResult<()> {
        let scenario = Self::scenario_mut(&mut self.scenarios, scenario_id)?;
        scenario.enabled = is_enabled;

        Ok(())
    }

    /// Borrow one scenario by id.
    fn scenario_mut(
        scenarios: &mut [Scenario],
        scenario_id: ScenarioId,
    ) -> RuntimeResult<&mut Scenario> {
        scenarios
            .iter_mut()
            .find(|scenario| scenario.id == scenario_id)
            .ok_or_else(|| Self::unknown_scenario_error(scenario_id))
    }

    /// Return one unknown-scenario error.
    fn unknown_scenario_error(scenario_id: ScenarioId) -> Box<RuntimeError> {
        Self::scenario_error(format!(
            "runtime scenario mutation requires one known scenario id: {}",
            scenario_id.0
        ))
    }

    /// Return one invalid-scenario error.
    fn scenario_error(message: impl Into<String>) -> Box<RuntimeError> {
        RuntimeError::Internal {
            message: message.into(),
        }
        .boxed()
    }

    /// Return the current world wall time.
    pub(crate) fn wall(&self) -> Nanos {
        self.clock().wall()
    }

    /// Return the current world wall time in nanoseconds.
    pub(crate) fn wall_nanos(&self) -> u64 {
        self.wall().get()
    }

    /// Return the current world monotonic time.
    pub(crate) fn mono(&self) -> Nanos {
        self.clock().mono()
    }

    /// Return the current world monotonic time in nanoseconds.
    pub(crate) fn mono_nanos(&self) -> u64 {
        self.mono().get()
    }

    /// Return the earliest deadline across worker-local and world-local timed work.
    pub(crate) fn next_deadline<I>(&self, worker_deadlines: I) -> Option<Instant>
    where
        I: IntoIterator<Item = Option<Instant>>,
    {
        let mut next_deadline = self.simulation().next_deadline();

        for worker_deadline in worker_deadlines {
            next_deadline = match (next_deadline, worker_deadline) {
                (Some(current), Some(candidate)) => Some(current.min(candidate)),
                (Some(current), None) => Some(current),
                (None, Some(candidate)) => Some(candidate),
                (None, None) => None,
            };
        }

        next_deadline
    }
}
