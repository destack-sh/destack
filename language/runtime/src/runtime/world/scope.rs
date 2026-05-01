use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::observe::{Observation, ObservationSequence, Observations};
use crate::runtime::policy::{HookEvent, PolicyDecision, PolicyState, RuleSubject};
use crate::runtime::random::Random;
use crate::runtime::time::Clock;
use crate::runtime::trace::Trace;
use crate::runtime::{RuntimeId, WorkerId};
use crate::simulation::Simulation;
use destack_workspace::{RandomMode, TimeMode};
use std::collections::BTreeMap;

use super::{BranchId, Topology, WorldResource, WorldResourceId};

/// Execution-scoped mutable world handle.
#[derive(Debug)]
pub(crate) struct WorldScope {
    /// Active branch identifier for this live execution scope.
    pub(crate) branch_id: BranchId,
    /// Effective world time mode after execution-mode resolution.
    pub(crate) time_mode: TimeMode,
    /// Effective world random mode after execution-mode resolution.
    pub(crate) random_mode: RandomMode,
    /// Shared simulation state for all workers using this world.
    simulation: *mut Simulation,
    /// Active policy state.
    policy: *mut PolicyState,
    /// The next runtime id to allocate.
    next_runtime_id: *mut u64,
    /// The next worker id to allocate.
    next_worker_id: *mut u64,
    /// Topology registry for world metadata.
    topology: *mut Topology,
    /// Logical resource records keyed by world resource identifier.
    resources: *mut BTreeMap<WorldResourceId, WorldResource>,
    /// Shared world clock.
    clock: *const Clock,
    /// Shared world randomness state.
    random: *const Random,
    /// Trace of world events.
    trace: *const Trace,
    /// Emitted observations.
    observations: *const Observations,
}

impl WorldScope {
    /// Create one execution-scoped world handle from split world fields.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        branch_id: BranchId,
        time_mode: TimeMode,
        random_mode: RandomMode,
        simulation: &mut Simulation,
        policy: &mut PolicyState,
        next_runtime_id: &mut u64,
        next_worker_id: &mut u64,
        topology: &mut Topology,
        resources: &mut BTreeMap<WorldResourceId, WorldResource>,
        clock: &Clock,
        random: &Random,
        trace: &Trace,
        observations: &Observations,
    ) -> Self {
        Self {
            branch_id,
            time_mode,
            random_mode,
            simulation,
            policy,
            next_runtime_id,
            next_worker_id,
            topology,
            resources,
            clock,
            random,
            trace,
            observations,
        }
    }

    /// Borrow the live simulation state.
    #[inline]
    pub(crate) fn simulation(&self) -> &Simulation {
        // safety: the execution scope owns the live simulation borrow
        unsafe { &*self.simulation }
    }

    /// Borrow the live policy state.
    #[inline]
    pub(crate) fn policy(&self) -> &PolicyState {
        // safety: the execution scope owns the live policy borrow
        unsafe { &*self.policy }
    }

    /// Apply one policy event against the live policy state.
    pub(crate) fn apply_policy_event(
        &self,
        event: &HookEvent,
        subject: RuleSubject<'_>,
    ) -> Vec<PolicyDecision> {
        // policy
        let policy = unsafe { &mut *self.policy };

        policy.on_event_for_subject(event, subject, self.random())
    }

    /// Borrow the live topology.
    #[inline]
    pub(crate) fn topology(&self) -> &Topology {
        // safety: the execution scope owns the live topology borrow
        unsafe { &*self.topology }
    }

    /// Register one runtime and its primary worker in world topology.
    pub(crate) fn register_runtime_topology(
        &self,
        runtime_id: RuntimeId,
        runtime_name: String,
        runtime_labels: BTreeMap<String, String>,
        primary_worker_id: WorkerId,
        primary_worker_name: String,
        primary_worker_labels: BTreeMap<String, String>,
    ) -> RuntimeResult<()> {
        // topology
        let topology = unsafe { &mut *self.topology };
        let result = topology.add_runtime(
            runtime_id,
            runtime_name,
            runtime_labels,
            primary_worker_id,
            primary_worker_name,
            primary_worker_labels,
        );

        result.map_err(|message| {
            RuntimeError::Internal {
                message: message.to_string(),
            }
            .boxed()
        })
    }

    /// Register one worker in one existing runtime.
    pub(crate) fn register_worker_topology(
        &self,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        worker_name: String,
        worker_labels: BTreeMap<String, String>,
    ) -> RuntimeResult<()> {
        // topology
        let topology = unsafe { &mut *self.topology };
        let result = topology.add_worker(runtime_id, worker_id, worker_name, worker_labels);

        result.map_err(|message| {
            RuntimeError::Internal {
                message: message.to_string(),
            }
            .boxed()
        })
    }

    /// Attach one resource to the world topology and resource table.
    pub(crate) fn attach_resource(&self, resource: WorldResource) -> RuntimeResult<()> {
        // topology
        let topology = unsafe { &mut *self.topology };
        let result = topology.attach_resource(
            resource.id,
            resource.kind.clone(),
            resource.label.as_deref(),
        );
        result.map_err(|message| {
            RuntimeError::Internal {
                message: message.to_string(),
            }
            .boxed()
        })?;

        // resource table
        let resources = unsafe { &mut *self.resources };
        resources.insert(resource.id, resource);

        Ok(())
    }

    /// Detach one resource from the world topology and resource table.
    pub(crate) fn detach_resource(&self, resource_id: WorldResourceId) {
        // topology
        let topology = unsafe { &mut *self.topology };
        topology.detach_resource(resource_id);

        // resource table
        let resources = unsafe { &mut *self.resources };
        resources.remove(&resource_id);
    }

    /// Borrow the shared world clock.
    #[inline]
    pub(crate) fn clock(&self) -> &Clock {
        // safety: the execution scope owns the live world borrow
        unsafe { &*self.clock }
    }

    /// Borrow the shared world randomness state.
    #[inline]
    pub(crate) fn random(&self) -> &Random {
        // safety: the execution scope owns the live world borrow
        unsafe { &*self.random }
    }

    /// Borrow the shared trace controller.
    #[inline]
    pub(crate) fn trace(&self) -> &Trace {
        // safety: the execution scope owns the live world borrow
        unsafe { &*self.trace }
    }

    /// Borrow the emitted observations.
    #[inline]
    pub(crate) fn observations(&self) -> &Observations {
        // safety: the execution scope owns the live world borrow
        unsafe { &*self.observations }
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
    pub(crate) fn allocate_runtime_id(&self) -> RuntimeId {
        // id cursor
        let runtime_id = unsafe { *self.next_runtime_id };
        unsafe {
            *self.next_runtime_id = runtime_id + 1;
        }

        RuntimeId(runtime_id)
    }

    /// Allocate one worker identifier.
    pub(crate) fn allocate_worker_id(&self) -> WorkerId {
        // id cursor
        let worker_id = unsafe { *self.next_worker_id };
        unsafe {
            *self.next_worker_id = worker_id + 1;
        }

        WorkerId(worker_id)
    }
}
