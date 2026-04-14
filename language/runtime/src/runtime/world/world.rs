use std::collections::BTreeMap;
use std::sync::Arc;

use destack_heap as heap;
use parking_lot::RwLock;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::runtime::bindings::BindingReplayPayload;
use crate::runtime::policy::{Policy, PolicyState};
use crate::runtime::random::{Random, RandomStreamId};
use crate::runtime::time::{Clock, HostClockSource, Nanos};
use crate::runtime::trace::{EnvironmentConfig, Outcome, Trace, TraceHeader, TraceSequence};
use crate::runtime::{AgentId, Runtime};
use crate::simulation::Simulation;
use destack_workspace::{ExecutionMode, RandomMode, ReplayPayloadMode, RuntimeOptions, TimeMode};

use super::lineage::{Lineage, ROOT_BRANCH_ID, ROOT_IMAGE_ID};
use super::topology::Topology;
pub(crate) use super::topology::{
    RuntimeId, WorldEdge, WorldEdgeId, WorldEdgeKind, WorldEdgeKindDefinition, WorldEntity,
    WorldEntityId, WorldEntityKind, WorldEntityKindDefinition,
};
use super::{
    BranchId, INITIAL_AGENT_ID, INITIAL_RUNTIME_ID, Image, ImageStore, Input, Observation,
    ObservationSequence, Observations, WorldResource, WorldResourceId,
};

/// Number of bytes in a megabyte for replay chunk sizing.
const BYTES_PER_MB: u64 = 1024 * 1024;

/// Execution-scoped mutable world reference.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WorldRef {
    /// Active branch identifier for this live execution scope.
    pub(crate) branch_id: BranchId,
    /// Effective world time mode after execution-mode resolution.
    pub(crate) time_mode: TimeMode,
    /// Effective world random mode after execution-mode resolution.
    pub(crate) random_mode: RandomMode,
    /// Shared page arena for every branchable world allocation.
    arena: *const Arc<heap::Arena>,
    /// Exact hard limits for world-owned shared memory.
    pub(crate) shared_limits: heap::SharedLimits,
    /// Shared simulation state for all agents using this world.
    simulation: *mut Simulation,
    /// Active policy state.
    policy: *mut PolicyState,
    /// The next runtime id to allocate.
    next_runtime_id: *mut u64,
    /// The next agent id to allocate.
    next_agent_id: *mut u64,
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
    /// World-owned shared memory visible across agents.
    shared: *mut heap::SharedSpace,
}

impl WorldRef {
    /// Create one execution-scoped world reference from split world fields.
    pub(crate) fn new(
        branch_id: BranchId,
        time_mode: TimeMode,
        random_mode: RandomMode,
        arena: &Arc<heap::Arena>,
        shared_limits: heap::SharedLimits,
        simulation: &mut Simulation,
        policy: &mut PolicyState,
        next_runtime_id: &mut u64,
        next_agent_id: &mut u64,
        topology: &mut Topology,
        resources: &mut BTreeMap<WorldResourceId, WorldResource>,
        clock: &Clock,
        random: &Random,
        trace: &Trace,
        observations: &Observations,
        shared: &mut heap::SharedSpace,
    ) -> Self {
        Self {
            branch_id,
            time_mode,
            random_mode,
            arena,
            shared_limits,
            simulation,
            policy,
            next_runtime_id,
            next_agent_id,
            topology,
            resources,
            clock,
            random,
            trace,
            observations,
            shared,
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

    /// Return the shared page arena for this execution scope.
    #[inline]
    pub(crate) fn arena(&self) -> Arc<heap::Arena> {
        // safety: the execution scope owns the live world borrow
        unsafe { (&*self.arena).clone() }
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

    /// Borrow the shared world memory mutably.
    #[inline]
    pub(crate) fn shared_mut(&self) -> &mut heap::SharedSpace {
        // safety: the execution scope owns the live shared-space borrow
        unsafe { &mut *self.shared }
    }

    /// Allocate one runtime identifier.
    pub(crate) fn allocate_runtime_id(&self) -> RuntimeId {
        let runtime_id = unsafe { *self.next_runtime_id };
        unsafe {
            *self.next_runtime_id = runtime_id.saturating_add(1);
        }

        RuntimeId(runtime_id)
    }

    /// Allocate one agent identifier.
    pub(crate) fn allocate_agent_id(&self) -> AgentId {
        let agent_id = unsafe { *self.next_agent_id };
        unsafe {
            *self.next_agent_id = agent_id.saturating_add(1);
        }

        AgentId(agent_id)
    }

    /// Register one runtime and its primary agent in world topology.
    pub(crate) fn register_runtime_topology(
        &self,
        runtime_id: RuntimeId,
        runtime_name: String,
        runtime_labels: BTreeMap<String, String>,
        primary_agent_id: AgentId,
        primary_agent_name: String,
        primary_agent_labels: BTreeMap<String, String>,
    ) -> RuntimeResult<()> {
        self.topology_mut()
            .add_runtime(
                runtime_id,
                runtime_name,
                runtime_labels,
                primary_agent_id,
                primary_agent_name,
                primary_agent_labels,
            )
            .map_err(|message| {
                RuntimeError::Internal {
                    message: message.to_string(),
                }
                .boxed()
            })
    }

    /// Register one agent in one existing runtime.
    pub(crate) fn register_agent_topology(
        &self,
        runtime_id: RuntimeId,
        agent_id: AgentId,
        agent_name: String,
        agent_labels: BTreeMap<String, String>,
    ) -> RuntimeResult<()> {
        self.topology_mut()
            .add_agent(runtime_id, agent_id, agent_name, agent_labels)
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

    /// Return the earliest deadline across agent-local and world-local timed work.
    pub(crate) fn next_deadline<I>(
        &self,
        agent_deadlines: I,
    ) -> Option<crate::runtime::time::WorldInstant>
    where
        I: IntoIterator<Item = Option<crate::runtime::time::WorldInstant>>,
    {
        let mut next_deadline = self.simulation().next_deadline();

        for agent_deadline in agent_deadlines {
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

/// Shared deterministic runtime world.
#[derive(Debug)]
pub struct World {
    /// Active branch identifier for this live world instance.
    pub(crate) branch_id: BranchId,
    /// Live runtimes owned by this world.
    pub(crate) runtimes: BTreeMap<RuntimeId, Box<Runtime>>,
    /// Shared simulation state for all agents using this world.
    pub(crate) simulation: Simulation,
    /// Active policy state.
    pub(crate) policy: PolicyState,
    /// The next runtime id to allocate.
    pub(crate) next_runtime_id: u64,
    /// The next agent id to allocate.
    pub(crate) next_agent_id: u64,
    /// Topology registry for world metadata.
    pub(crate) topology: Topology,
    /// Logical resource records keyed by world resource identifier.
    pub(crate) resources: BTreeMap<WorldResourceId, WorldResource>,

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
    /// Shared page arena for every branchable world allocation.
    pub(crate) arena: Arc<heap::Arena>,
    /// World-owned lineage metadata.
    pub(crate) lineage: Arc<RwLock<Lineage>>,
    /// World-owned retained image payloads.
    pub(crate) images: Arc<RwLock<ImageStore>>,
    /// World-owned shared memory visible across agents.
    pub(crate) shared: heap::SharedSpace,
    /// Exact hard limits for world-owned shared memory.
    pub(crate) shared_limits: heap::SharedLimits,
}

impl World {
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
        // execution mode: replay forces virtual time and deterministic random
        let is_replay = options.execution == ExecutionMode::Replay;
        let time_mode = if is_replay {
            TimeMode::Virtual
        } else {
            options.time.mode
        };
        let random_mode = if is_replay {
            RandomMode::Deterministic
        } else {
            options.random.mode
        };
        let replay_payload = match options.replay.payload {
            ReplayPayloadMode::ResultsOnly => BindingReplayPayload::Results,
            ReplayPayloadMode::ArgumentsAndResults => BindingReplayPayload::ArgumentsAndResults,
        };

        // replay header: options with chunk-size override
        let mut trace_header = TraceHeader {
            execution_mode: options.execution,
            time_mode,
            random_mode,
            branch_id,
            replay_payload,
            ..TraceHeader::new(EnvironmentConfig::default())
        };
        if let Some(chunk_size_mb) = options.replay.chunk_size_mb {
            let chunk_bytes = chunk_size_mb.saturating_mul(BYTES_PER_MB);
            if chunk_bytes > 0 {
                trace_header.max_chunk_bytes = chunk_bytes;
            }
        }

        // world state
        let clock = if let Some(host_clock_source) = host_clock_source {
            Clock::from_options_with_host_clock_source(&options.time, host_clock_source)
        } else {
            Clock::from_options(&options.time)
        };
        let random = Random::new(options.random.seed.unwrap_or(0));
        let policy = Policy::from_workspace_rules(&options.rules);
        let trace = Trace::new(options.execution, trace_header);
        let topology = Topology::new();
        let arena = Arc::new(heap::Arena::with_page_bytes(options.heap.page_bytes));
        let shared = heap::SharedSpace::with_arena(arena.clone());
        let shared_limits = heap::SharedLimits {
            max_bytes: options.heap.max_shared_bytes,
        };
        policy.validate_with_kind_catalog(&topology)?;

        // final world state
        let root_image = Arc::new(Image {
            id: ROOT_IMAGE_ID,
            next_runtime_id: INITIAL_RUNTIME_ID,
            next_agent_id: INITIAL_AGENT_ID,
            policy: PolicyState::new(policy.clone()),
            topology: topology.clone(),
            resources: BTreeMap::new(),
            simulation: Simulation::default(),
            clock: clock.snapshot(),
            random: random.snapshot(),
            shared: shared.image(),
            runtimes: BTreeMap::new(),
            agents: BTreeMap::new(),
        });
        let root_trace_image = Arc::new(trace.capture_image());
        let lineage = Arc::new(RwLock::new(Lineage::new_root(
            root_image.clock.virtual_wall,
            root_image.clock.virtual_mono,
            root_trace_image.next_sequence,
        )));
        let images = Arc::new(RwLock::new(ImageStore::new_root(
            root_image,
            root_trace_image,
        )));

        let world = Self {
            branch_id,
            runtimes: BTreeMap::new(),
            simulation: Simulation::default(),
            policy: PolicyState::new(policy),
            next_runtime_id: INITIAL_RUNTIME_ID,
            next_agent_id: INITIAL_AGENT_ID,
            topology,
            resources: BTreeMap::new(),
            time_mode,
            random_mode,
            clock,
            random,
            trace,
            observations: Observations::default(),
            arena,
            lineage,
            images,
            shared,
            shared_limits,
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
    pub fn next_simulation_deadline(&self) -> Option<crate::runtime::time::WorldInstant> {
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

    /// Return labels for one live agent.
    pub fn agent_labels(&self, agent_id: AgentId) -> RuntimeResult<BTreeMap<String, String>> {
        let entity_id = format!("agent.{}", agent_id.0);
        let topology = &self.topology;
        let entity =
            topology
                .entities()
                .get(entity_id.as_str())
                .ok_or(RuntimeError::AgentNotFound {
                    agent_id: agent_id.0,
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
            &self.arena,
            self.shared_limits,
            &mut self.simulation,
            &mut self.policy,
            &mut self.next_runtime_id,
            &mut self.next_agent_id,
            &mut self.topology,
            &mut self.resources,
            &self.clock,
            &self.random,
            &self.trace,
            &self.observations,
            &mut self.shared,
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

    /// Ingest one authoritative input at the world boundary.
    pub fn ingest(&self, input: Input) -> RuntimeResult<()> {
        self.trace.record_input(input)
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

    /// Resolve one input against the replay boundary.
    pub(crate) fn resolve_input(&self, input: Input) -> RuntimeResult<Input> {
        self.trace.resolve_input(input)
    }
}
