use tspp_program as program;

use crate::debugger::Debugger;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::ResourceId;
use crate::runtime::RuntimeId;
use crate::worker::WorkerId;
use crate::world::observation::{Observation, ObservationLog, ObservationSequence};
use crate::world::policy::Policy;
use crate::world::random::Random;
use crate::world::time::{Clock, Nanos};
use crate::world::trace::TraceLog;

use super::{BranchId, Entity, Moment, MomentSequence, Topology};

/// Shared world state used by runtimes and workers.
#[derive(Debug)]
pub(crate) struct WorldState {
    /// Active branch identifier for this live world.
    pub(crate) branch_id: BranchId,
    /// Current branch-local moment sequence.
    pub(crate) moment: MomentSequence,
    /// Active policy state.
    pub(crate) policy: Policy,
    /// Active debugger configuration.
    pub(crate) debugger: Debugger,
    /// The next runtime id to allocate.
    pub(crate) next_runtime_id: u64,
    /// The next worker id to allocate.
    pub(crate) next_worker_id: u64,
    /// Topology registry for world metadata.
    pub(crate) topology: Topology,
    /// Shared world clock.
    pub(crate) clock: Clock,
    /// Shared world randomness state.
    pub(crate) random: Random,
    /// Trace of world events.
    pub(crate) trace: TraceLog,
    /// Emitted observations.
    pub(crate) observations: ObservationLog,
}

impl WorldState {
    /// Register one runtime in world topology.
    pub(crate) fn register_runtime_topology(
        &mut self,
        runtime_id: RuntimeId,
        runtime_entity: Entity,
    ) -> RuntimeResult<()> {
        let result = self.topology.add_runtime(runtime_id, runtime_entity);

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
        worker_entity: Entity,
    ) -> RuntimeResult<()> {
        let result = self
            .topology
            .add_worker(runtime_id, worker_id, worker_entity);

        result.map_err(|message| {
            RuntimeError::Internal {
                message: message.to_string(),
            }
            .boxed()
        })
    }

    /// Attach one resource to the world topology and resource table.
    pub(crate) fn attach_resource(
        &mut self,
        resource_id: ResourceId,
        entity: Entity,
    ) -> RuntimeResult<()> {
        let result = self.topology.attach_resource(resource_id, entity);
        result.map_err(|message| {
            RuntimeError::Internal {
                message: message.to_string(),
            }
            .boxed()
        })?;

        Ok(())
    }

    /// Detach one resource from the world topology and resource table.
    pub(crate) fn detach_resource(&mut self, resource_id: ResourceId) {
        self.topology.detach_resource(resource_id);
    }

    /// Return the current live execution coordinate.
    pub(crate) fn moment(&self) -> Moment {
        Moment::new(self.branch_id, self.moment)
    }

    /// Advance the branch-local moment sequence.
    pub(crate) fn advance_moment(&mut self) -> RuntimeResult<Moment> {
        self.moment = self.moment.next()?;

        Ok(self.moment())
    }

    /// Emit one observation at the current execution coordinate.
    pub(crate) fn observe(&self, observation: Observation) -> RuntimeResult<ObservationSequence> {
        self.observations.record_at(self.moment(), observation)
    }

    /// Process one instrumentable Program execution event.
    pub(crate) fn probe(
        &mut self,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        fiber_id: Option<program::FiberId>,
        event: program::Event,
    ) -> RuntimeResult<()> {
        let branch_id = self.branch_id;
        let sequence = &mut self.moment;
        let observations = &self.observations;
        let mut observation_moment = None;

        self.debugger
            .probe(runtime_id, worker_id, fiber_id, event, |probe_id, count| {
                // allocate one shared Moment only when this event emits
                let moment = match observation_moment {
                    Some(moment) => moment,
                    None => {
                        *sequence = sequence.next()?;
                        let moment = Moment::new(branch_id, *sequence);
                        observation_moment = Some(moment);

                        moment
                    }
                };

                // record this exact Probe hit
                observations
                    .record_at(
                        moment,
                        Observation::ProbeHit {
                            probe_id,
                            count,
                            runtime_id,
                            worker_id,
                            fiber_id,
                            event,
                        },
                    )
                    .map(|_| ())
            })
    }

    /// Allocate one runtime identifier.
    pub(crate) fn allocate_runtime_id(&mut self) -> RuntimeResult<RuntimeId> {
        let runtime_id = self.next_runtime_id;
        self.next_runtime_id = runtime_id.checked_add(1).ok_or_else(|| {
            RuntimeError::Internal {
                message: "runtime identifier space exhausted".to_string(),
            }
            .boxed()
        })?;

        Ok(RuntimeId(runtime_id))
    }

    /// Allocate one worker identifier.
    pub(crate) fn allocate_worker_id(&mut self) -> RuntimeResult<WorkerId> {
        let worker_id = self.next_worker_id;
        self.next_worker_id = worker_id.checked_add(1).ok_or_else(|| {
            RuntimeError::Internal {
                message: "worker identifier space exhausted".to_string(),
            }
            .boxed()
        })?;

        Ok(WorkerId(worker_id))
    }

    /// Return the current world wall time.
    pub(crate) fn wall(&self) -> Nanos {
        self.clock.wall()
    }

    /// Return the current world wall time in nanoseconds.
    pub(crate) fn wall_nanos(&self) -> u64 {
        self.wall().get()
    }

    /// Return the current world monotonic time.
    pub(crate) fn mono(&self) -> Nanos {
        self.clock.mono()
    }

    /// Return the current world monotonic time in nanoseconds.
    pub(crate) fn mono_nanos(&self) -> u64 {
        self.mono().get()
    }
}
