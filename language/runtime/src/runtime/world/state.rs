use std::collections::BTreeMap;

use destack_workspace::{RandomMode, TimeMode};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::runtime::policy::{HookEvent, PolicyDecision, PolicyState, RuleSubject};
use crate::runtime::random::{Random, RandomStreamId};
use crate::runtime::time::{Clock, Instant, Nanos};
use crate::runtime::trace::{Observation, ObservationSequence, Observations, Trace};
use crate::runtime::{RuntimeId, WorkerId};
use crate::simulation::Simulation;

use crate::platform::ResourceId;

use super::{BranchId, Resource, Topology};

/// Shared world state used by runtimes and workers.
#[derive(Debug)]
pub(crate) struct WorldState {
    /// Active branch identifier for this live world.
    pub(crate) branch_id: BranchId,
    /// Effective world time mode after execution-mode resolution.
    pub(crate) time_mode: TimeMode,
    /// Effective world random mode after execution-mode resolution.
    pub(crate) random_mode: RandomMode,
    /// Shared simulation state for all runtimes in this world.
    pub(crate) simulation: Simulation,
    /// Active policy state.
    pub(crate) policy: PolicyState,
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

    /// Apply one policy event against the live policy state.
    pub(crate) fn apply_policy_event(
        &mut self,
        event: &HookEvent,
        subject: RuleSubject<'_>,
    ) -> Vec<PolicyDecision> {
        self.policy
            .on_event_for_subject(event, subject, &self.random)
    }

    /// Borrow the live topology.
    #[inline]
    pub(crate) fn topology(&self) -> &Topology {
        &self.topology
    }

    /// Register one runtime and its default worker in world topology.
    pub(crate) fn register_runtime_topology(
        &mut self,
        runtime_id: RuntimeId,
        runtime_name: String,
        runtime_labels: BTreeMap<String, String>,
        default_worker_id: WorkerId,
        default_worker_name: String,
        default_worker_labels: BTreeMap<String, String>,
    ) -> RuntimeResult<()> {
        let result = self.topology.add_runtime(
            runtime_id,
            runtime_name,
            runtime_labels,
            default_worker_id,
            default_worker_name,
            default_worker_labels,
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
        let result = self.topology.attach_resource(
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

        self.resources.insert(resource.id, resource);

        Ok(())
    }

    /// Detach one resource from the world topology and resource table.
    pub(crate) fn detach_resource(&mut self, resource_id: ResourceId) {
        self.topology.detach_resource(resource_id);
        self.resources.remove(&resource_id);
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

    /// Return the current world wall time.
    pub(crate) fn wall(&self) -> Nanos {
        match self.time_mode {
            TimeMode::Host => self.clock().host_wall(),
            TimeMode::Virtual => self.clock().virtual_wall(),
        }
    }

    /// Return the current world wall time in nanoseconds.
    pub(crate) fn wall_nanos(&self) -> u64 {
        self.wall().get()
    }

    /// Return the current world monotonic time.
    pub(crate) fn mono(&self) -> Nanos {
        match self.time_mode {
            TimeMode::Host => self.clock().host_mono(),
            TimeMode::Virtual => self.clock().virtual_mono(),
        }
    }

    /// Return the current world monotonic time in nanoseconds.
    pub(crate) fn mono_nanos(&self) -> u64 {
        self.mono().get()
    }

    /// Fill one buffer with secure world-routed random bytes.
    pub(crate) fn fill_secure_bytes(&self, buffer: &mut [u8]) -> RuntimeResult<()> {
        if self.random_mode == RandomMode::Deterministic {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.random.secure.bytes",
            ))
            .boxed());
        }

        self.random().fill_secure_bytes(buffer)
    }

    /// Try to fill one buffer with secure world-routed random bytes without blocking.
    pub(crate) fn try_fill_secure_bytes(&self, buffer: &mut [u8]) -> RuntimeResult<()> {
        if self.random_mode == RandomMode::Deterministic {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.random.secure.bytesTry",
            ))
            .boxed());
        }

        self.random().try_fill_secure_bytes(buffer)
    }

    /// Return one world-routed random u64 from one stream.
    pub(crate) fn next_stream_u64(&self, stream_id: RandomStreamId) -> RuntimeResult<u64> {
        match self.random_mode {
            RandomMode::Host => self.random().next_secure_u64(),
            RandomMode::Deterministic => Ok(self.random().next_stream_u64(stream_id)),
        }
    }

    /// Fill one buffer with world-routed random bytes from one stream.
    pub(crate) fn fill_stream_bytes(
        &self,
        stream_id: RandomStreamId,
        buffer: &mut [u8],
    ) -> RuntimeResult<()> {
        match self.random_mode {
            RandomMode::Host => self.random().fill_secure_bytes(buffer),
            RandomMode::Deterministic => {
                self.random().fill_stream_bytes(stream_id, buffer);
                Ok(())
            }
        }
    }

    /// Return the effective world time mode.
    pub(crate) const fn time_mode(&self) -> TimeMode {
        self.time_mode
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
