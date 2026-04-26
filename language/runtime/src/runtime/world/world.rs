use std::collections::BTreeMap;
use std::sync::Arc;

use destack_heap as heap;
use parking_lot::RwLock;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::runtime::bindings::BindingReplayPayload;
use crate::runtime::observe::{Observation, ObservationSequence, Observations};
use crate::runtime::policy::{Policy, PolicyState};
use crate::runtime::random::{Random, RandomStreamId};
use crate::runtime::time::{Clock, HostClockSource, Nanos, WorldInstant};
use crate::runtime::trace::{EnvironmentConfig, Outcome, Trace, TraceHeader, TraceSequence};
use crate::runtime::{Collector, CollectorMode, Runtime, WorkerId};
use crate::simulation::Simulation;
use destack_workspace::{RandomMode, ReplayPayloadMode, RuntimeOptions, TimeMode};

use super::lineage::{Lineage, ROOT_BRANCH_ID};
use super::topology::Topology;
pub(crate) use super::topology::{
    RuntimeId, WorldEdge, WorldEdgeId, WorldEdgeKind, WorldEdgeKindDefinition, WorldEntity,
    WorldEntityId, WorldEntityKind, WorldEntityKindDefinition,
};
use super::{
    BranchId, Command, INITIAL_RUNTIME_ID, INITIAL_WORKER_ID, WorldImage, WorldRef, WorldResource,
    WorldResourceId,
};

/// Number of bytes in a megabyte for replay chunk sizing.
const BYTES_PER_MB: u64 = 1024 * 1024;

impl WorldRef {
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
        self.topology_mut()
            .add_runtime(
                runtime_id,
                runtime_name,
                runtime_labels,
                primary_worker_id,
                primary_worker_name,
                primary_worker_labels,
            )
            .map_err(|message| {
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
        self.topology_mut()
            .add_worker(runtime_id, worker_id, worker_name, worker_labels)
            .map_err(|message| {
                RuntimeError::Internal {
                    message: message.to_string(),
                }
                .boxed()
            })
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
    pub(crate) fn next_deadline<I>(&self, worker_deadlines: I) -> Option<WorldInstant>
    where
        I: IntoIterator<Item = Option<WorldInstant>>,
    {
        let mut next_deadline = self.simulation().next_deadline();

        for agent_deadline in worker_deadlines {
            next_deadline = match (next_deadline, agent_deadline) {
                (Some(current), Some(candidate)) => Some(current.min(candidate)),
                (Some(current), None) => Some(current),
                (None, Some(candidate)) => Some(candidate),
                (None, None) => None,
            };
        }

        next_deadline
    }
}

/// One interconnected runtime world.
#[derive(Debug)]
pub struct World {
    /// Active branch identifier for this live world instance.
    pub(crate) branch_id: BranchId,
    /// Live runtimes owned by this world.
    pub(crate) runtimes: BTreeMap<RuntimeId, Box<Runtime>>,
    /// Shared simulation state for all workers using this world.
    pub(crate) simulation: Simulation,
    /// Active policy state.
    pub(crate) policy: PolicyState,
    /// The next runtime id to allocate.
    pub(crate) next_runtime_id: u64,
    /// The next worker id to allocate.
    pub(crate) next_worker_id: u64,
    /// Topology registry for world metadata.
    pub(crate) topology: Topology,
    /// Logical resource records keyed by world resource identifier.
    pub(crate) resources: BTreeMap<WorldResourceId, WorldResource>,

    /// Lineage-root metadata for this live world.
    pub(crate) lineage: Arc<RwLock<Lineage>>,
    /// Effective world time mode after execution-mode resolution.
    pub(crate) time_mode: TimeMode,
    /// Effective world random mode after execution-mode resolution.
    pub(crate) random_mode: RandomMode,
    /// Shared world clock.
    pub(crate) clock: Clock,
    /// Shared world randomness state.
    pub(crate) random: Random,
    /// Trace of world events.
    pub(crate) trace: Trace,
    /// Emitted observation log (separate from causal trace).
    pub(crate) observations: Observations,
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
        host_clock_source: Option<std::sync::Arc<dyn HostClockSource>>,
    ) -> RuntimeResult<Self> {
        Self::for_branch(ROOT_BRANCH_ID, options, host_clock_source)
    }

    /// Create one world for one explicit active branch.
    pub(crate) fn for_branch(
        branch_id: BranchId,
        options: &RuntimeOptions,
        host_clock_source: Option<std::sync::Arc<dyn HostClockSource>>,
    ) -> RuntimeResult<Self> {
        // collapsed execution summaries
        let execution_mode = options.execution_mode();
        let time_mode = options.time_mode();
        let random_mode = options.random_mode();
        let replay_payload = match options.replay_payload_mode() {
            ReplayPayloadMode::ResultsOnly => BindingReplayPayload::Results,
            ReplayPayloadMode::ArgumentsAndResults => BindingReplayPayload::ArgumentsAndResults,
        };

        // replay header: options with chunk-size override
        let mut trace_header = TraceHeader {
            execution_mode,
            time_mode,
            random_mode,
            branch_id,
            replay_payload,
            ..TraceHeader::new(EnvironmentConfig::default())
        };
        if let Some(chunk_size_mb) = options.trace_chunk_size_mb() {
            let chunk_bytes = chunk_size_mb.saturating_mul(BYTES_PER_MB);
            if chunk_bytes > 0 {
                trace_header.max_chunk_bytes = chunk_bytes;
            }
        }

        // world state
        let time_options = options.time_options();
        let clock = if let Some(host_clock_source) = host_clock_source {
            Clock::from_options_with_host_clock_source(&time_options, host_clock_source)
        } else {
            Clock::from_options(&time_options)
        };
        let random = Random::new(options.random_options().seed.unwrap_or(0));
        let policy = Policy::from_workspace_rules(&options.effect.rules);
        let trace = Trace::new(execution_mode, trace_header);
        let topology = Topology::new();
        let allocator = Arc::new(
            heap::Allocator::try_new(
                options.heap.layout.page_bytes,
                options.heap.layout.arena_bytes,
            )
            .map_err(Box::<RuntimeError>::from)?,
        );

        // final world state
        let root_image = Arc::new(WorldImage {
            next_runtime_id: INITIAL_RUNTIME_ID,
            next_worker_id: INITIAL_WORKER_ID,
            policy: PolicyState::new(policy.clone()),
            topology: topology.clone(),
            resources: BTreeMap::new(),
            simulation: Simulation::default(),
            clock: clock.snapshot(),
            random: random.snapshot(),
            runtimes: BTreeMap::new(),
            workers: BTreeMap::new(),
        });
        let root_trace_image = Arc::new(trace.capture_image());
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
        policy.validate_with_kind_catalog(&topology)?;

        let world = Self {
            branch_id,
            runtimes: BTreeMap::new(),
            simulation: Simulation::default(),
            policy: PolicyState::new(policy),
            next_runtime_id: INITIAL_RUNTIME_ID,
            next_worker_id: INITIAL_WORKER_ID,
            topology,
            resources: BTreeMap::new(),
            time_mode,
            random_mode,
            clock,
            random,
            trace,
            observations: Observations::default(),
            lineage,
        };

        Ok(world)
    }

    /// Borrow one read guard for simulation.
    pub fn simulation(&self) -> &Simulation {
        &self.simulation
    }

    /// Borrow one write guard for simulation.
    pub fn simulation_mut(&mut self) -> &mut Simulation {
        &mut self.simulation
    }

    /// Return the earliest deadline contributed by simulation state.
    pub fn next_simulation_deadline(&self) -> Option<WorldInstant> {
        self.simulation.next_deadline()
    }

    /// Snapshot world policy state.
    pub fn policy(&self) -> Policy {
        self.policy.spec.clone()
    }

    /// Snapshot world entity kind definitions.
    pub fn entity_kinds(&self) -> BTreeMap<WorldEntityKind, WorldEntityKindDefinition> {
        self.topology.entity_kinds().clone()
    }

    /// Snapshot world edge kind definitions.
    pub fn edge_kinds(&self) -> BTreeMap<WorldEdgeKind, WorldEdgeKindDefinition> {
        self.topology.edge_kinds().clone()
    }

    /// Snapshot world entities.
    pub fn entities(&self) -> BTreeMap<WorldEntityId, WorldEntity> {
        self.topology.entities().clone()
    }

    /// Return labels for one live runtime.
    pub fn runtime_labels(&self, runtime_id: RuntimeId) -> RuntimeResult<BTreeMap<String, String>> {
        let entity_id = format!("runtime.{}", runtime_id.0);
        let topology = &self.topology;
        let entity =
            topology
                .entities()
                .get(entity_id.as_str())
                .ok_or(RuntimeError::RuntimeNotFound {
                    runtime_id: runtime_id.0,
                })?;

        Ok(entity.labels.clone())
    }

    /// Return labels for one live worker.
    pub fn worker_labels(&self, worker_id: WorkerId) -> RuntimeResult<BTreeMap<String, String>> {
        let entity_id = format!("worker.{}", worker_id.0);
        let topology = &self.topology;
        let entity =
            topology
                .entities()
                .get(entity_id.as_str())
                .ok_or(RuntimeError::WorkerNotFound {
                    worker_id: worker_id.0,
                })?;

        Ok(entity.labels.clone())
    }

    /// Snapshot world edges.
    pub fn edges(&self) -> BTreeMap<WorldEdgeId, WorldEdge> {
        self.topology.edges().clone()
    }

    /// Snapshot logical world resources.
    pub fn resources(&self) -> BTreeMap<WorldResourceId, WorldResource> {
        self.resources.clone()
    }

    /// Borrow the live branch state.
    pub(crate) fn world_ref(&mut self) -> WorldRef {
        WorldRef::new(
            self.branch_id,
            self.time_mode,
            self.random_mode,
            &mut self.simulation,
            &mut self.policy,
            &mut self.next_runtime_id,
            &mut self.next_worker_id,
            &mut self.topology,
            &mut self.resources,
            &self.clock,
            &self.random,
            &self.trace,
            &self.observations,
        )
    }

    /// Borrow the shared world clock.
    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    /// Return the effective world time mode.
    pub fn time_mode(&self) -> TimeMode {
        self.time_mode
    }

    /// Return the effective world random mode.
    pub fn random_mode(&self) -> RandomMode {
        self.random_mode
    }

    /// Return the emitted observation log for this world.
    pub fn observations(&self) -> &Observations {
        &self.observations
    }

    /// Emit one observation at the current world moment.
    pub fn observe(&mut self, observation: Observation) -> ObservationSequence {
        let moment = self.moment();

        self.observations.record_at(moment, observation)
    }

    /// Return the current world wall time.
    pub fn wall(&self) -> Nanos {
        match self.time_mode {
            TimeMode::Host => self.clock.host_wall(),
            TimeMode::Virtual => self.clock.virtual_wall(),
        }
    }

    /// Return the current world wall time in nanoseconds.
    pub fn wall_nanos(&self) -> u64 {
        self.wall().get()
    }

    /// Return the current world monotonic time.
    pub fn mono(&self) -> Nanos {
        match self.time_mode {
            TimeMode::Host => self.clock.host_mono(),
            TimeMode::Virtual => self.clock.virtual_mono(),
        }
    }

    /// Return the current world monotonic time in nanoseconds.
    pub fn mono_nanos(&self) -> u64 {
        self.mono().get()
    }

    /// Borrow the shared world randomness state.
    pub fn random(&self) -> &Random {
        &self.random
    }

    /// Fill one buffer with secure world-routed random bytes.
    pub fn fill_secure_bytes(&self, buffer: &mut [u8]) -> RuntimeResult<()> {
        // deterministic worlds reject secure host entropy by default
        if self.random_mode == RandomMode::Deterministic {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.random.secure.bytes",
            ))
            .boxed());
        }

        self.random.fill_secure_bytes(buffer)
    }

    /// Try to fill one buffer with secure world-routed random bytes without blocking.
    pub fn try_fill_secure_bytes(&self, buffer: &mut [u8]) -> RuntimeResult<()> {
        // deterministic worlds reject secure host entropy by default
        if self.random_mode == RandomMode::Deterministic {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.random.secure.bytesTry",
            ))
            .boxed());
        }

        self.random.try_fill_secure_bytes(buffer)
    }

    /// Return one world-routed random u64 from one stream.
    pub fn next_stream_u64(&self, stream_id: RandomStreamId) -> RuntimeResult<u64> {
        match self.random_mode {
            RandomMode::Host => self.random.next_secure_u64(),
            RandomMode::Deterministic => Ok(self.random.next_stream_u64(stream_id)),
        }
    }

    /// Fill one buffer with world-routed random bytes from one stream.
    pub fn fill_stream_bytes(
        &self,
        stream_id: RandomStreamId,
        buffer: &mut [u8],
    ) -> RuntimeResult<()> {
        match self.random_mode {
            RandomMode::Host => self.random.fill_secure_bytes(buffer),
            RandomMode::Deterministic => {
                self.random.fill_stream_bytes(stream_id, buffer);
                Ok(())
            }
        }
    }

    /// Borrow the shared trace controller.
    pub fn trace(&self) -> &Trace {
        &self.trace
    }

    /// Ingest one authoritative command at the world boundary.
    pub fn ingest(&self, command: Command) -> RuntimeResult<()> {
        self.trace.record_command(command)
    }

    /// Accept one authoritative external outcome at the world boundary.
    pub fn accept(&self, outcome: Outcome) -> RuntimeResult<()> {
        self.trace.record_outcome(outcome)
    }

    /// Append one authoritative history anchor at the world boundary.
    pub fn anchor(&self, label: impl Into<String>) -> RuntimeResult<TraceSequence> {
        self.trace.record_anchor(label.into())
    }

    /// Append one explicit history label to the active trace.
    pub fn label(&self, label: impl Into<String>) -> RuntimeResult<TraceSequence> {
        self.anchor(label)
    }

    /// Resolve one command against the replay boundary.
    pub(crate) fn resolve_command(&self, command: Command) -> RuntimeResult<Command> {
        self.trace.resolve_command(command)
    }
}
