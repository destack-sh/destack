use std::collections::BTreeMap;
use std::sync::Arc;

use destack_heap as heap;
use parking_lot::RwLock;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{PlatformError, ResourceId};
use crate::runtime::binding::BindingReplayPayload;
use crate::runtime::policy::{Policy, PolicyState};
use crate::runtime::random::{Random, RandomStreamId};
use crate::runtime::time::{Clock, HostClockSource, Instant, Nanos};
use crate::runtime::trace::{
    EnvironmentConfig, Observation, ObservationSequence, Observations, Outcome, Trace, TraceHeader,
    TraceSequence,
};
use crate::runtime::{Collector, CollectorMode, Runtime, WorkerId};
use crate::simulation::Simulation;
use destack_workspace::{RandomMode, ReplayPayloadMode, RuntimeOptions, TimeMode};

use super::lineage::{Lineage, ROOT_BRANCH};
use super::topology::Topology;
pub(crate) use super::topology::{
    Edge, EdgeDefinition, EdgeId, EdgeKind, Entity, EntityDefinition, EntityId, EntityKind,
    RuntimeId,
};
use super::{BranchId, Command, Resource, WorldImage, WorldState};

/// Number of bytes in one configured trace chunk mebibyte.
const TRACE_CHUNK_MEBIBYTE_BYTES: u64 = 1024 * 1024;

/// One interconnected runtime world.
#[derive(Debug)]
pub struct World {
    /// Live runtimes owned by this world.
    pub(crate) runtimes: BTreeMap<RuntimeId, Box<Runtime>>,
    /// Shared state used by runtimes and workers.
    pub(crate) state: WorldState,

    /// Lineage-root metadata for this live world.
    pub(crate) lineage: Arc<RwLock<Lineage>>,
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
        Self::new_at_branch(ROOT_BRANCH, options, host_clock_source)
    }

    /// Create one world for one explicit active branch.
    pub(crate) fn new_at_branch(
        branch_id: BranchId,
        options: &RuntimeOptions,
        host_clock_source: Option<Arc<dyn HostClockSource>>,
    ) -> RuntimeResult<Self> {
        // effective modes
        let execution_mode = options.execution_mode();
        let time_mode = options.time_mode();
        let random_mode = options.random_mode();
        let replay_payload = match options.replay_payload_mode() {
            ReplayPayloadMode::ResultsOnly => BindingReplayPayload::Results,
            ReplayPayloadMode::ArgumentsAndResults => BindingReplayPayload::ArgumentsAndResults,
        };

        // replay header
        let mut trace_header = TraceHeader {
            execution_mode,
            time_mode,
            random_mode,
            branch_id,
            replay_payload,
            ..TraceHeader::new(EnvironmentConfig::default())
        };
        if let Some(chunk_size_mb) = options.trace_chunk_size_mb() {
            let chunk_bytes = chunk_size_mb.saturating_mul(TRACE_CHUNK_MEBIBYTE_BYTES);
            if chunk_bytes > 0 {
                trace_header.max_chunk_bytes = chunk_bytes;
            }
        }

        // live state inputs
        let time_options = options.time_options();
        let clock = if let Some(host_clock_source) = host_clock_source {
            Clock::from_options_with_host_clock_source(&time_options, host_clock_source)
        } else {
            Clock::from_options(&time_options)
        };
        let random = Random::from_options(&options.random_options());
        let policy = Policy::default();
        let trace = Trace::new(execution_mode, trace_header);
        let topology = Topology::new();
        policy.validate_with_kind_catalog(&topology)?;

        // live world state
        let state = WorldState {
            branch_id,
            time_mode,
            random_mode,
            simulation: Simulation::default(),
            policy: PolicyState::new(policy),
            next_runtime_id: 1,
            next_worker_id: 1,
            topology,
            resources: BTreeMap::new(),
            clock,
            random,
            trace,
            observations: Observations::default(),
        };

        // lineage backing
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
            policy: state.policy.clone(),
            topology: state.topology.clone(),
            resources: state.resources.clone(),
            simulation: state.simulation.clone(),
            clock: state.clock.snapshot(),
            random: state.random.snapshot(),
            runtimes: BTreeMap::new(),
            workers: BTreeMap::new(),
        });
        let root_trace_image = Arc::new(state.trace.capture_image());
        let collector_mode = CollectorMode::from_scheduler_mode(options.scheduler.mode);
        let collector = Collector::new(collector_mode, format!("destack.collector.{branch_id:?}"))?;
        let lineage = Arc::new(RwLock::new(Lineage::new_root(
            allocator,
            collector,
            root_image.clock.virtual_wall,
            root_image.clock.virtual_mono,
            root_trace_image.next_sequence,
            root_image,
            root_trace_image,
        )));
        let world = Self {
            runtimes: BTreeMap::new(),
            state,
            lineage,
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
        self.state.policy.spec.clone()
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
        let topology = &self.state.topology;
        let entity = topology.entities().get(&runtime_id.entity_id()).ok_or(
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            },
        )?;

        Ok(entity.labels.clone())
    }

    /// Return labels for one live worker.
    pub fn worker_labels(&self, worker_id: WorkerId) -> RuntimeResult<BTreeMap<String, String>> {
        let topology = &self.state.topology;
        let entity = topology.entities().get(&worker_id.entity_id()).ok_or(
            RuntimeError::WorkerNotFound {
                worker_id: worker_id.0,
            },
        )?;

        Ok(entity.labels.clone())
    }

    /// Snapshot world edges.
    pub fn edges(&self) -> BTreeMap<EdgeId, Edge> {
        self.state.topology.edges().clone()
    }

    /// Snapshot world resources.
    pub fn resources(&self) -> BTreeMap<ResourceId, Resource> {
        self.state.resources.clone()
    }

    /// Borrow the shared world clock.
    pub fn clock(&self) -> &Clock {
        &self.state.clock
    }

    /// Return the effective world time mode.
    pub fn time_mode(&self) -> TimeMode {
        self.state.time_mode
    }

    /// Return the effective world random mode.
    pub fn random_mode(&self) -> RandomMode {
        self.state.random_mode
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
        match self.state.time_mode {
            TimeMode::Host => self.state.clock.host_wall(),
            TimeMode::Virtual => self.state.clock.virtual_wall(),
        }
    }

    /// Return the current world wall time in nanoseconds.
    pub fn wall_nanos(&self) -> u64 {
        self.wall().get()
    }

    /// Return the current world monotonic time.
    pub fn mono(&self) -> Nanos {
        match self.state.time_mode {
            TimeMode::Host => self.state.clock.host_mono(),
            TimeMode::Virtual => self.state.clock.virtual_mono(),
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
        if self.state.random_mode == RandomMode::Deterministic {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.random.secure.bytes",
            ))
            .boxed());
        }

        self.state.random.fill_secure_bytes(buffer)
    }

    /// Try to fill one buffer with secure world-routed random bytes without blocking.
    pub fn try_fill_secure_bytes(&self, buffer: &mut [u8]) -> RuntimeResult<()> {
        // deterministic worlds reject secure host entropy by default
        if self.state.random_mode == RandomMode::Deterministic {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.random.secure.bytesTry",
            ))
            .boxed());
        }

        self.state.random.try_fill_secure_bytes(buffer)
    }

    /// Return one world-routed random u64 from one stream.
    pub fn next_stream_u64(&self, stream_id: RandomStreamId) -> RuntimeResult<u64> {
        match self.state.random_mode {
            RandomMode::Host => self.state.random.next_secure_u64(),
            RandomMode::Deterministic => Ok(self.state.random.next_stream_u64(stream_id)),
        }
    }

    /// Fill one buffer with world-routed random bytes from one stream.
    pub fn fill_stream_bytes(
        &self,
        stream_id: RandomStreamId,
        buffer: &mut [u8],
    ) -> RuntimeResult<()> {
        match self.state.random_mode {
            RandomMode::Host => self.state.random.fill_secure_bytes(buffer),
            RandomMode::Deterministic => {
                self.state.random.fill_stream_bytes(stream_id, buffer);
                Ok(())
            }
        }
    }

    /// Borrow the shared trace controller.
    pub fn trace(&self) -> &Trace {
        &self.state.trace
    }

    /// Record one authoritative command at the world boundary.
    pub fn record_command(&self, command: Command) -> RuntimeResult<()> {
        self.state.trace.record_command(command)
    }

    /// Record one authoritative external outcome at the world boundary.
    pub fn record_outcome(&self, outcome: Outcome) -> RuntimeResult<()> {
        self.state.trace.record_outcome(outcome)
    }

    /// Append one explicit history label to the active trace.
    pub fn label(&self, label: impl Into<String>) -> RuntimeResult<TraceSequence> {
        self.state.trace.record_anchor(label.into())
    }

    /// Resolve one command against the replay boundary.
    pub(crate) fn resolve_command(&self, command: Command) -> RuntimeResult<Command> {
        self.state.trace.resolve_command(command)
    }
}
