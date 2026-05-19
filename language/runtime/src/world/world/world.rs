use std::collections::BTreeMap;
use std::sync::Arc;

use destack_heap as heap;
use parking_lot::RwLock;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::binding::BindingReplayPayload;
use crate::host::core::HostQueue;
use crate::host::poller::{HostPoller, create_host_poller};
use crate::host::time::HostClockSource;
use crate::host::{Host, HostError, ResourceId, compile_target_host};
use crate::runtime::random::{Random, RandomStreamId};
use crate::runtime::time::{Clock, Instant, Nanos};
use crate::runtime::{Runtime, SharedCollector, SharedCollectorMode, WorkerId};
use crate::simulation::Simulation;
use crate::world::policy::Policy;
use crate::world::scenario::Scenario;
use crate::world::trace::{
    EntrypointCall, Observation, ObservationSequence, Observations, Outcome, Trace, TraceHeader,
    TraceSequence,
};
use destack_workspace::{
    Environment, PollerBackend, RandomSource, ReplayPayloadMode, RuntimeOptions,
};

use super::topology::Topology;
pub(crate) use super::topology::{
    Edge, EdgeDefinition, EdgeId, EdgeKind, Entity, EntityDefinition, EntityId, EntityKind,
    RuntimeId,
};
use super::{BranchId, History, Mutation, ROOT_BRANCH, Resource, WorldImage, WorldState};

/// One interconnected runtime world.
pub struct World {
    /// Host integration shared by live runtimes in this world.
    pub(crate) host: Arc<dyn Host>,
    /// Host events accepted into this world.
    pub(crate) host_queue: HostQueue,
    /// Shared host poller for external resources.
    pub(crate) poller: Box<dyn HostPoller>,
    /// Host poller backend used for fork reconstruction.
    pub(crate) poller_backend: PollerBackend,
    /// Live runtimes owned by this world.
    pub(crate) runtimes: BTreeMap<RuntimeId, Box<Runtime>>,
    /// Shared state used by runtimes and workers.
    pub(crate) state: WorldState,
    /// History-root metadata for this live world.
    pub(crate) history: Arc<RwLock<History>>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("host", &self.host)
            .field("host_queue", &self.host_queue)
            .field("poller", &"<host poller>")
            .field("poller_backend", &self.poller_backend)
            .field("runtimes", &self.runtimes)
            .field("state", &self.state)
            .field("history", &self.history)
            .finish()
    }
}

impl World {
    /// Suspend runtime shared GC while one quiescent world operation runs.
    pub(crate) fn quiesce_shared_gc(&self) {
        for runtime in self.runtimes.values() {
            runtime.shared.quiesce();
        }
    }

    /// Resume runtime shared GC after one quiescent world operation.
    pub(crate) fn resume_shared_gc(&self) {
        for runtime in self.runtimes.values() {
            runtime.shared.resume();
        }
    }

    /// Create one world from runtime options.
    pub fn from_options(options: &RuntimeOptions) -> RuntimeResult<Self> {
        Self::new(options, None)
    }

    /// Create one world from runtime options and optional host clock source.
    pub(crate) fn new(
        options: &RuntimeOptions,
        host_clock_source: Option<Arc<dyn HostClockSource>>,
    ) -> RuntimeResult<Self> {
        Self::empty(ROOT_BRANCH, options, host_clock_source)
    }

    /// Create one empty world shell for restore or replay.
    pub(crate) fn empty(
        branch_id: BranchId,
        options: &RuntimeOptions,
        host_clock_source: Option<Arc<dyn HostClockSource>>,
    ) -> RuntimeResult<Self> {
        // execution configuration
        let execution_mode = options.execution_mode();
        let clock_source = options.clock_source();
        let random_source = options.random_source();
        let replay_payload = match options.replay_payload_mode() {
            ReplayPayloadMode::ResultsOnly => BindingReplayPayload::Results,
            ReplayPayloadMode::ArgumentsAndResults => BindingReplayPayload::ArgumentsAndResults,
        };

        // replay header
        let mut trace_header = TraceHeader {
            execution_mode,
            clock_source,
            random_source,
            branch_id,
            replay_payload,
            ..TraceHeader::new(Environment::default())
        };
        if let Some(chunk_size_mb) = options.trace_chunk_size_mb() {
            let chunk_bytes = chunk_size_mb.saturating_mul(1024 * 1024);
            if chunk_bytes > 0 {
                trace_header.max_chunk_bytes = chunk_bytes;
            }
        }

        // live state inputs
        let time_options = options.time_options();
        let host = compile_target_host(host_clock_source);
        let clock = Clock::from_options(&time_options);
        let random = Random::from_options(&options.random_options());
        let policy = Policy::default();
        let scenarios: Vec<Scenario> = Vec::new();
        let trace = Trace::new(execution_mode, trace_header);
        let topology = Topology::new();

        // live world state
        let state = WorldState {
            branch_id,
            simulation: Simulation::default(),
            policy,
            scenarios,
            next_runtime_id: 1,
            next_worker_id: 1,
            next_scenario_id: 1,
            topology,
            resources: BTreeMap::new(),
            clock,
            random,
            trace,
            observations: Observations::default(),
        };

        // history backing
        let allocator = Arc::new(
            heap::Allocator::try_new(
                options.heap.layout.page_bytes,
                options.heap.layout.chunk_bytes,
            )
            .map_err(Box::<RuntimeError>::from)?,
        );

        // root image mirrors the initial live state
        let root_image = Arc::new(WorldImage {
            next_runtime_id: state.next_runtime_id,
            next_worker_id: state.next_worker_id,
            next_scenario_id: state.next_scenario_id,
            policy: state.policy.clone(),
            scenarios: state.scenarios.clone(),
            topology: state.topology.clone(),
            resources: state.resources.clone(),
            simulation: state.simulation.clone(),
            clock: state.clock.snapshot(),
            random: state.random.snapshot(),
            runtimes: BTreeMap::new(),
            workers: BTreeMap::new(),
        });
        let root_trace_image = Arc::new(state.trace.capture_image());
        let collector_mode = SharedCollectorMode::from_scheduler_mode(options.scheduler.mode);
        let collector =
            SharedCollector::new(collector_mode, format!("destack.collector.{branch_id:?}"))?;
        let history = Arc::new(RwLock::new(History::new_root(
            allocator,
            collector,
            root_image.clock.wall,
            root_image.clock.monotonic,
            root_trace_image.next_sequence(),
            root_image,
            root_trace_image,
        )));
        let poller_backend = options.scheduler.poller_backend;
        let poller = create_host_poller(poller_backend)?;
        let world = Self {
            host,
            host_queue: HostQueue::new(),
            poller,
            poller_backend,
            runtimes: BTreeMap::new(),
            state,
            history,
        };

        Ok(world)
    }

    /// Borrow simulation state.
    pub fn simulation(&self) -> &Simulation {
        &self.state.simulation
    }

    /// Borrow mutable simulation state.
    pub fn simulation_mut(&mut self) -> &mut Simulation {
        &mut self.state.simulation
    }

    /// Return the earliest deadline contributed by simulation state.
    pub fn next_simulation_deadline(&self) -> Option<Instant> {
        self.state.simulation.next_deadline()
    }

    /// Snapshot world policy state.
    pub fn policy(&self) -> Policy {
        self.state.policy.clone()
    }

    /// Snapshot world entity kind definitions.
    pub fn entity_kinds(&self) -> BTreeMap<EntityKind, EntityDefinition> {
        self.state.topology.entity_kinds().clone()
    }

    /// Snapshot world edge kind definitions.
    pub fn edge_kinds(&self) -> BTreeMap<EdgeKind, EdgeDefinition> {
        self.state.topology.edge_kinds().clone()
    }

    /// Snapshot world entities.
    pub fn entities(&self) -> BTreeMap<EntityId, Entity> {
        self.state.topology.entities().clone()
    }

    /// Return labels for one live runtime.
    pub fn runtime_labels(&self, runtime_id: RuntimeId) -> RuntimeResult<BTreeMap<String, String>> {
        let entity = self.runtime_entity(runtime_id)?;

        Ok(entity.labels)
    }

    /// Return the topology name for one live runtime.
    pub fn runtime_name(&self, runtime_id: RuntimeId) -> RuntimeResult<&str> {
        let topology = &self.state.topology;
        let entity = topology.entities().get(&runtime_id.entity_id()).ok_or(
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            },
        )?;

        Ok(entity.name.as_str())
    }

    /// Return the topology entity for one live runtime.
    pub(crate) fn runtime_entity(&self, runtime_id: RuntimeId) -> RuntimeResult<Entity> {
        let topology = &self.state.topology;
        let entity = topology.entities().get(&runtime_id.entity_id()).ok_or(
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            },
        )?;

        Ok(entity.clone())
    }

    /// Return labels for one live worker.
    pub fn worker_labels(&self, worker_id: WorkerId) -> RuntimeResult<BTreeMap<String, String>> {
        let entity = self.worker_entity(worker_id)?;

        Ok(entity.labels)
    }

    /// Return the topology name for one live worker.
    pub fn worker_name(&self, worker_id: WorkerId) -> RuntimeResult<&str> {
        let topology = &self.state.topology;
        let entity = topology.entities().get(&worker_id.entity_id()).ok_or(
            RuntimeError::WorkerNotFound {
                worker_id: worker_id.0,
            },
        )?;

        Ok(entity.name.as_str())
    }

    /// Return the topology entity for one live worker.
    pub(crate) fn worker_entity(&self, worker_id: WorkerId) -> RuntimeResult<Entity> {
        let topology = &self.state.topology;
        let entity = topology.entities().get(&worker_id.entity_id()).ok_or(
            RuntimeError::WorkerNotFound {
                worker_id: worker_id.0,
            },
        )?;

        Ok(entity.clone())
    }

    /// Snapshot world edges.
    pub fn edges(&self) -> BTreeMap<EdgeId, Edge> {
        self.state.topology.edges().clone()
    }

    /// Snapshot world resources.
    pub fn resources(&self) -> BTreeMap<ResourceId, Resource> {
        self.state.resources.clone()
    }

    /// Snapshot active scenario scripts.
    pub fn scenarios(&self) -> Vec<Scenario> {
        self.state.scenarios.clone()
    }

    /// Borrow the shared world clock.
    pub fn clock(&self) -> &Clock {
        &self.state.clock
    }

    /// Return the effective world clock source.
    pub fn clock_source(&self) -> destack_workspace::ClockSource {
        self.state.clock.source()
    }

    /// Return the effective world random source.
    pub fn random_source(&self) -> RandomSource {
        self.state.random.source()
    }

    /// Return the emitted observation log for this world.
    pub fn observations(&self) -> &Observations {
        &self.state.observations
    }

    /// Emit one observation at the current world moment.
    pub fn observe(&mut self, observation: Observation) -> ObservationSequence {
        let moment = self.moment();

        self.state.observations.record_at(moment, observation)
    }

    /// Return the current world wall time.
    pub fn wall(&self) -> Nanos {
        match self.state.clock.source() {
            destack_workspace::ClockSource::Host => Nanos::new(self.host.wall_nanos()),
            destack_workspace::ClockSource::Virtual => self.state.clock.wall(),
        }
    }

    /// Return the current world wall time in nanoseconds.
    pub fn wall_nanos(&self) -> u64 {
        self.wall().get()
    }

    /// Return the current world monotonic time.
    pub fn mono(&self) -> Nanos {
        match self.state.clock.source() {
            destack_workspace::ClockSource::Host => Nanos::new(self.host.mono_nanos()),
            destack_workspace::ClockSource::Virtual => self.state.clock.mono(),
        }
    }

    /// Return the current world monotonic time in nanoseconds.
    pub fn mono_nanos(&self) -> u64 {
        self.mono().get()
    }

    /// Borrow the shared world randomness state.
    pub fn random(&self) -> &Random {
        &self.state.random
    }

    /// Fill one buffer with secure world-routed random bytes.
    pub fn fill_secure_bytes(&self, buffer: &mut [u8]) -> RuntimeResult<()> {
        // deterministic worlds reject secure host entropy by default
        if self.state.random.source() == RandomSource::Deterministic {
            return Err(RuntimeError::from(HostError::not_supported(
                "destack.random.secure.bytes",
            ))
            .boxed());
        }

        self.host.fill_random_bytes(buffer)
    }

    /// Try to fill one buffer with secure world-routed random bytes without blocking.
    pub fn try_fill_secure_bytes(&self, buffer: &mut [u8]) -> RuntimeResult<()> {
        // deterministic worlds reject secure host entropy by default
        if self.state.random.source() == RandomSource::Deterministic {
            return Err(RuntimeError::from(HostError::not_supported(
                "destack.random.secure.bytesTry",
            ))
            .boxed());
        }

        self.host.try_fill_random_bytes(buffer)
    }

    /// Return one world-routed random u64 from one stream.
    pub fn next_stream_u64(&self, stream_id: RandomStreamId) -> RuntimeResult<u64> {
        match self.state.random.source() {
            RandomSource::Host => self.host.random_u64(),
            RandomSource::Deterministic => Ok(self.state.random.next_stream_u64(stream_id)),
        }
    }

    /// Fill one buffer with world-routed random bytes from one stream.
    pub fn fill_stream_bytes(
        &self,
        stream_id: RandomStreamId,
        buffer: &mut [u8],
    ) -> RuntimeResult<()> {
        match self.state.random.source() {
            RandomSource::Host => self.host.fill_random_bytes(buffer),
            RandomSource::Deterministic => {
                self.state.random.fill_stream_bytes(stream_id, buffer);
                Ok(())
            }
        }
    }

    /// Borrow the shared trace controller.
    pub fn trace(&self) -> &Trace {
        &self.state.trace
    }

    /// Record one authoritative mutation at the world boundary.
    pub fn record_mutation(&self, mutation: Mutation) -> RuntimeResult<()> {
        self.state.trace.record_mutation(mutation)
    }

    /// Record one runtime entrypoint call.
    pub fn record_entrypoint(&self, invocation: EntrypointCall) -> RuntimeResult<()> {
        self.state.trace.record_entrypoint(invocation)
    }

    /// Record one authoritative external outcome at the world boundary.
    pub fn record_outcome(&self, outcome: Outcome) -> RuntimeResult<()> {
        self.state.trace.record_outcome(outcome)
    }

    /// Append one explicit history label to the active trace.
    pub fn label(&self, label: impl Into<String>) -> RuntimeResult<TraceSequence> {
        self.state.trace.record_label(label.into())
    }

    /// Resolve one mutation against the replay boundary.
    pub(crate) fn resolve_mutation(&self, mutation: Mutation) -> RuntimeResult<Mutation> {
        self.state.trace.resolve_mutation(mutation)
    }

    /// Resolve one runtime entrypoint call.
    pub(crate) fn resolve_entrypoint(
        &self,
        invocation: EntrypointCall,
    ) -> RuntimeResult<EntrypointCall> {
        self.state.trace.resolve_entrypoint(invocation)
    }
}
