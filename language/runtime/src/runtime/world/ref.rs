use crate::runtime::observe::{Observation, ObservationSequence, Observations};
use crate::runtime::policy::PolicyState;
use crate::runtime::random::Random;
use crate::runtime::time::Clock;
use crate::runtime::trace::Trace;
use crate::runtime::{RuntimeId, WorkerId};
use crate::simulation::Simulation;
use destack_workspace::{RandomMode, TimeMode};
use std::collections::BTreeMap;

use super::{BranchId, Topology, WorldResource, WorldResourceId};

/// Execution-scoped mutable world reference.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WorldRef {
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

impl WorldRef {
    /// Create one execution-scoped world reference from split world fields.
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

    /// Borrow the live policy state mutably.
    #[inline]
    pub(crate) fn policy_mut(&self) -> &mut PolicyState {
        // safety: the execution scope owns the live policy borrow
        unsafe { &mut *self.policy }
    }

    /// Borrow the live topology.
    #[inline]
    pub(crate) fn topology(&self) -> &Topology {
        // safety: the execution scope owns the live topology borrow
        unsafe { &*self.topology }
    }

    /// Borrow the live topology mutably.
    #[inline]
    pub(crate) fn topology_mut(&self) -> &mut Topology {
        // safety: the execution scope owns the live topology borrow
        unsafe { &mut *self.topology }
    }

    /// Borrow the live world resources mutably.
    #[inline]
    pub(crate) fn resources_mut(&self) -> &mut BTreeMap<WorldResourceId, WorldResource> {
        // safety: the execution scope owns the live resource borrow
        unsafe { &mut *self.resources }
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
        let runtime_id = unsafe { *self.next_runtime_id };
        unsafe {
            *self.next_runtime_id = runtime_id.saturating_add(1);
        }

        RuntimeId(runtime_id)
    }

    /// Allocate one worker identifier.
    pub(crate) fn allocate_worker_id(&self) -> WorkerId {
        let worker_id = unsafe { *self.next_worker_id };
        unsafe {
            *self.next_worker_id = worker_id.saturating_add(1);
        }

        WorkerId(worker_id)
    }
}
