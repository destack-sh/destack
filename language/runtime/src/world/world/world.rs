use std::collections::BTreeMap;
use std::sync::Arc;

use parking_lot::RwLock;

use tspp_heap as heap;
use tspp_memory::MemoryMap;
use tspp_repository::{Environment, WorldOptions};

use crate::binding::ReplayPayload;
use crate::debugger::Debugger;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::heap::{WorldCollector, WorldCollectorMode};
use crate::host::poller::HostPoller;
use crate::host::time::HostClockSource;
use crate::host::{Host, HostError, HostEvent, HostQueue, compile_target};
use crate::runtime::{Runtime, RuntimeId};
use crate::worker::WorkerId;
use crate::world::observation::{Observation, ObservationLog, ObservationSequence};
use crate::world::policy::Policy;
use crate::world::random::{Random, RandomSource, RandomStreamId};
use crate::world::time::{Clock, ClockSource, Nanos};
use crate::world::topology::LabelSet;
use crate::world::trace::{EntrypointCall, TraceHeader, TraceLog};

use super::constants::WORLD_MEMORY_MAP_SIZE_BYTES;
use super::topology::Topology;
pub(crate) use super::topology::{
    Edge, EdgeDefinition, EdgeId, EdgeKind, Entity, EntityDefinition, EntityId, EntityKind,
};
use super::{BranchId, Lineage, MomentSequence, Mutation, ROOT_BRANCH, WorldImage, WorldState};

/// One interconnected runtime world.
pub struct World {
    /// Host integration shared by live runtimes in this world.
    pub(crate) host: Arc<dyn Host>,
    /// Host events accepted into this world.
    pub(crate) host_queue: HostQueue,
    /// Shared host poller for external resources.
    pub(crate) poller: HostPoller,
    /// Live runtimes owned by this world.
    pub(crate) runtimes: BTreeMap<RuntimeId, Runtime>,
    /// The next runtime slot to schedule first.
    pub(crate) next_runtime_cursor: usize,
    /// Forkable virtual memory for every runtime and worker in this world.
    pub(crate) memory: Arc<MemoryMap>,
    /// Shared GC scheduler for live runtimes.
    pub(crate) collector: Arc<WorldCollector>,
    /// Shared state used by runtimes and workers.
    pub(crate) state: WorldState,
    /// Lineage-root metadata for this live world.
    pub(crate) lineage: Arc<RwLock<Lineage>>,
}

impl std::fmt::Debug for World {
    /// Format one world without traversing process-local host state.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("host", &self.host)
            .field("host_queue", &self.host_queue)
            .field("poller", &"<host poller>")
            .field("runtimes", &self.runtimes)
            .field("next_runtime_cursor", &self.next_runtime_cursor)
            .field("memory", &self.memory)
            .field("collector", &self.collector)
            .field("state", &self.state)
            .field("lineage", &self.lineage)
            .finish()
    }
}

impl World {
    /// Suspend runtime shared GC while one quiescent world operation runs.
    pub(crate) fn quiesce_shared_gc(&self) {
        for runtime in self.runtimes.values() {
            runtime.shared_collection.quiesce();
        }
    }

    /// Resume runtime shared GC after one quiescent world operation.
    pub(crate) fn resume_shared_gc(&self) {
        for runtime in self.runtimes.values() {
            runtime
                .shared_collection
                .resume(&runtime.shared_heap, &runtime.program);
        }
    }

    /// Create one world from explicit seed state.
    pub fn new(
        options: &WorldOptions,
        environment: impl Into<Arc<Environment>>,
    ) -> RuntimeResult<Self> {
        Self::empty(ROOT_BRANCH, options, environment, None)
    }

    /// Create one empty world shell for restore or replay.
    pub(crate) fn empty(
        branch_id: BranchId,
        options: &WorldOptions,
        environment: impl Into<Arc<Environment>>,
        host_clock_source: Option<Arc<dyn HostClockSource>>,
    ) -> RuntimeResult<Self> {
        let environment = environment.into();

        // execution configuration
        let execution_mode = options.mode;
        let clock_source = ClockSource::from_execution_mode(execution_mode);
        let random_source = RandomSource::from_execution_mode(execution_mode);
        let replay_payload = ReplayPayload::from(options.replay_payload);

        // replay header
        let trace_header = TraceHeader {
            execution_mode,
            clock_source,
            random_source,
            branch_id,
            replay_payload,
            ..TraceHeader::new(environment)
        };

        // live state inputs
        let host = compile_target(host_clock_source);
        let default_clock_epoch_nanos = match clock_source {
            ClockSource::Host => 0,
            ClockSource::Runtime => host.wall_nanos(),
        };
        let clock = Clock::from_options(clock_source, &options.clock, default_clock_epoch_nanos);
        let random = Random::from_options(random_source, &options.random);
        let policy = Policy::default();
        let debugger = Debugger::default();
        let trace = TraceLog::new(execution_mode, trace_header);
        let topology = Topology::new();
        let moment_sequence = MomentSequence::new(0);

        // live world state
        let state = WorldState {
            branch_id,
            moment: moment_sequence,
            policy,
            debugger,
            next_runtime_id: 1,
            next_worker_id: 1,
            topology,
            clock,
            random,
            trace,
            observations: ObservationLog::default(),
        };

        // reserve one world-relative address map
        let memory = Arc::new(
            MemoryMap::reserve(
                WORLD_MEMORY_MAP_SIZE_BYTES,
                heap::DEFAULT_HEAP_PAGE_SIZE_BYTES,
            )
            .map_err(Box::<RuntimeError>::from)?,
        );

        // root image mirrors the initial live state
        let root_memory = memory.capture().map_err(Box::<RuntimeError>::from)?;
        let root_image = Arc::new(WorldImage {
            memory: root_memory,
            moment: state.moment,
            next_runtime_id: state.next_runtime_id,
            next_worker_id: state.next_worker_id,
            next_runtime_cursor: 0,
            policy: state.policy.clone(),
            debugger: state.debugger.clone(),
            topology: state.topology.clone(),
            clock: state.clock.image(),
            random: state.random.image(),
            runtimes: BTreeMap::new(),
            workers: BTreeMap::new(),
        });
        let root_trace_image = Arc::new(state.trace.capture_image());

        // create the shared collector
        let collector_mode = WorldCollectorMode::from_execution_mode(execution_mode);
        let collector = WorldCollector::new(
            collector_mode,
            format!("tspp.collector.{}", branch_id.get()),
        )?;

        let lineage = Arc::new(RwLock::new(Lineage::new_root(
            root_image.clock.wall,
            root_image.clock.monotonic,
            root_image.moment,
            root_trace_image.next_sequence()?,
            root_image,
            root_trace_image,
        )));
        let poller = HostPoller::open()?;
        let world = Self {
            host,
            host_queue: HostQueue::new(),
            poller,
            runtimes: BTreeMap::new(),
            next_runtime_cursor: 0,
            memory,
            collector,
            state,
            lineage,
        };

        Ok(world)
    }

    /// Drain the host bootstrap events so spawned runtimes see a settled host.
    pub fn bootstrap_host(&mut self) -> RuntimeResult<()> {
        self.host_queue.poll(self.host.as_ref(), Some(0))?;

        Ok(())
    }

    /// Snapshot world policy state.
    pub fn policy(&self) -> Policy {
        self.state.policy.clone()
    }

    /// Borrow the active world debugger configuration.
    pub fn debugger(&self) -> &Debugger {
        &self.state.debugger
    }

    /// Snapshot world entity kind definitions.
    pub fn entity_kinds(&self) -> BTreeMap<EntityKind, EntityDefinition> {
        self.state.topology.entity_kinds()
    }

    /// Snapshot world edge kind definitions.
    pub fn edge_kinds(&self) -> BTreeMap<EdgeKind, EdgeDefinition> {
        self.state.topology.edge_kinds()
    }

    /// Snapshot world entities.
    pub fn entities(&self) -> BTreeMap<EntityId, Entity> {
        self.state.topology.entities().clone()
    }

    /// Return labels for one live runtime.
    pub fn runtime_labels(&self, runtime_id: RuntimeId) -> RuntimeResult<LabelSet> {
        let entity = self.runtime_entity(runtime_id)?;

        Ok(entity.labels)
    }

    /// Return the topology name for one live runtime.
    pub fn runtime_name(&self, runtime_id: RuntimeId) -> RuntimeResult<&str> {
        let topology = &self.state.topology;
        let entity = topology
            .entities()
            .get(&runtime_id.entity_id())
            .ok_or(RuntimeError::runtime_not_found(runtime_id.0))?;

        Ok(entity.name.as_str())
    }

    /// Return the topology entity for one live runtime.
    pub(crate) fn runtime_entity(&self, runtime_id: RuntimeId) -> RuntimeResult<Entity> {
        let topology = &self.state.topology;
        let entity = topology
            .entities()
            .get(&runtime_id.entity_id())
            .ok_or(RuntimeError::runtime_not_found(runtime_id.0))?;

        Ok(entity.clone())
    }

    /// Return labels for one live worker.
    pub fn worker_labels(&self, worker_id: WorkerId) -> RuntimeResult<LabelSet> {
        let entity = self.worker_entity(worker_id)?;

        Ok(entity.labels)
    }

    /// Return the topology name for one live worker.
    pub fn worker_name(&self, worker_id: WorkerId) -> RuntimeResult<&str> {
        let topology = &self.state.topology;
        let entity = topology
            .entities()
            .get(&worker_id.entity_id())
            .ok_or(RuntimeError::worker_not_found(worker_id.0))?;

        Ok(entity.name.as_str())
    }

    /// Return the topology entity for one live worker.
    pub(crate) fn worker_entity(&self, worker_id: WorkerId) -> RuntimeResult<Entity> {
        let topology = &self.state.topology;
        let entity = topology
            .entities()
            .get(&worker_id.entity_id())
            .ok_or(RuntimeError::worker_not_found(worker_id.0))?;

        Ok(entity.clone())
    }

    /// Snapshot world edges.
    pub fn edges(&self) -> BTreeMap<EdgeId, Edge> {
        self.state.topology.edges().clone()
    }

    /// Borrow the shared world clock.
    pub fn clock(&self) -> &Clock {
        &self.state.clock
    }

    /// Return the effective world clock source.
    pub fn clock_source(&self) -> ClockSource {
        self.state.clock.source()
    }

    /// Return the effective world random source.
    pub fn random_source(&self) -> RandomSource {
        self.state.random.source()
    }

    /// Return the emitted observation log for this world.
    pub fn observations(&self) -> &ObservationLog {
        &self.state.observations
    }

    /// Emit one observation at the current world moment.
    pub fn observe(&mut self, observation: Observation) -> RuntimeResult<ObservationSequence> {
        let moment = self.moment();

        self.state.observations.record_at(moment, observation)
    }

    /// Send one typed host Event into this World.
    pub fn send(&self, event: HostEvent) {
        self.host_queue.enqueue(vec![event]);
    }

    /// Return the current world wall time.
    pub fn wall(&self) -> Nanos {
        match self.state.clock.source() {
            ClockSource::Host => Nanos::new(self.host.wall_nanos()),
            ClockSource::Runtime => self.state.clock.wall(),
        }
    }

    /// Return the current world wall time in nanoseconds.
    pub fn wall_nanos(&self) -> u64 {
        self.wall().get()
    }

    /// Return the current world monotonic time.
    pub fn mono(&self) -> Nanos {
        match self.state.clock.source() {
            ClockSource::Host => Nanos::new(self.host.mono_nanos()),
            ClockSource::Runtime => self.state.clock.mono(),
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
        // runtime-owned randomness rejects secure host entropy by default
        if self.state.random.source() == RandomSource::Deterministic {
            return Err(
                RuntimeError::from(HostError::not_supported("tspp.random.secure.bytes")).boxed(),
            );
        }

        self.host.fill_random_bytes(buffer)
    }

    /// Try to fill one buffer with secure world-routed random bytes without blocking.
    pub fn try_fill_secure_bytes(&self, buffer: &mut [u8]) -> RuntimeResult<()> {
        // runtime-owned randomness rejects secure host entropy by default
        if self.state.random.source() == RandomSource::Deterministic {
            return Err(RuntimeError::from(HostError::not_supported(
                "tspp.random.secure.bytesTry",
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
    pub fn trace(&self) -> &TraceLog {
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
