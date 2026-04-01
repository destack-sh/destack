use std::cell::{Cell, Ref, RefCell, RefMut};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use destack_heap as heap;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::runtime::bindings::BindingReplayPayload;
use crate::runtime::policy::{Policy, PolicyState};
use crate::runtime::random::{Random, RandomStreamId};
use crate::runtime::time::{Clock, HostClockSource, Nanos};
use crate::runtime::trace::{Outcome, Trace, TraceHeader, TraceSequence};
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
    AccessState, BranchId, INITIAL_AGENT_ID, INITIAL_RUNTIME_ID, Image, Input, ObservationLog,
    WorldResource, WorldResourceId,
};

/// Number of bytes in a megabyte for replay chunk sizing.
const BYTES_PER_MB: u64 = 1024 * 1024;

/// Shared deterministic runtime world.
#[derive(Debug)]
pub struct World {
    /// Active branch identifier for this live world instance.
    pub(crate) branch_id: BranchId,
    /// Live runtimes owned by this world.
    pub(crate) runtimes: RefCell<BTreeMap<RuntimeId, Box<Runtime>>>,
    /// Shared simulation state for all agents using this world.
    pub(crate) simulation: RefCell<Simulation>,

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
    pub(crate) observations: ObservationLog,
    /// Active policy state.
    pub(crate) policy: RefCell<PolicyState>,
    /// One explicit reentrancy guard for structural mutation.
    pub(crate) mutation_active: Cell<bool>,
    /// The next runtime id to allocate.
    pub(crate) next_runtime_id: Cell<u64>,
    /// The next agent id to allocate.
    pub(crate) next_agent_id: Cell<u64>,
    /// World-owned lineage and durable restore metadata.
    pub(crate) lineage: Rc<RefCell<Lineage>>,
    /// World-owned shared and exclusive access coordinator state.
    pub(crate) access_state: Cell<AccessState>,
    /// Topology registry for world metadata.
    pub(crate) topology: RefCell<Topology>,
    /// Logical resource records keyed by world resource identifier.
    pub(crate) resources: RefCell<BTreeMap<WorldResourceId, WorldResource>>,
    /// World-owned shared memory visible across agents.
    pub(crate) shared: RefCell<heap::SharedSpace>,
    /// Exact hard limits for world-owned shared memory.
    pub(crate) shared_limits: heap::SharedLimits,
}

#[allow(clippy::arc_with_non_send_sync)]
impl World {
    /// Create one world from runtime options.
    pub fn from_options(options: &RuntimeOptions) -> RuntimeResult<Arc<Self>> {
        Self::new(options, None)
    }

    /// Create one world from runtime options and optional host clock source.
    #[allow(clippy::arc_with_non_send_sync)]
    pub(crate) fn new(
        options: &RuntimeOptions,
        host_clock_source: Option<Arc<dyn HostClockSource>>,
    ) -> RuntimeResult<Arc<Self>> {
        Self::build_with_branch(ROOT_BRANCH_ID, options, host_clock_source)
    }

    /// Create one world for one explicit active branch.
    #[allow(clippy::arc_with_non_send_sync)]
    pub(crate) fn build_with_branch(
        branch_id: BranchId,
        options: &RuntimeOptions,
        host_clock_source: Option<Arc<dyn HostClockSource>>,
    ) -> RuntimeResult<Arc<Self>> {
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
            ..TraceHeader::default()
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
        let mut shared = heap::SharedSpace::with_page_bytes(options.heap.page_bytes);
        let shared_limits = heap::SharedLimits {
            max_bytes: options.heap.max_shared_bytes,
        };
        policy.validate_with_kind_catalog(&topology)?;

        // final world state
        let lineage = Rc::new(RefCell::new(Lineage::new_root(
            Rc::new(Image {
                id: ROOT_IMAGE_ID,
                next_runtime_id: INITIAL_RUNTIME_ID,
                next_agent_id: INITIAL_AGENT_ID,
                policy: PolicyState::new(policy.clone()),
                topology: topology.clone(),
                resources: BTreeMap::new(),
                simulation: Simulation::default(),
                clock: clock.snapshot(),
                random: random.snapshot(),
                shared: shared.image(None),
                runtimes: BTreeMap::new(),
                agents: BTreeMap::new(),
            }),
            Rc::new(trace.capture_image()),
        )));

        let world = Arc::new(Self {
            branch_id,
            runtimes: RefCell::new(BTreeMap::new()),
            simulation: RefCell::new(Simulation::default()),
            time_mode,
            random_mode,
            clock,
            random,
            trace,
            observations: ObservationLog::default(),
            policy: RefCell::new(PolicyState::new(policy)),
            mutation_active: Cell::new(false),
            next_runtime_id: Cell::new(INITIAL_RUNTIME_ID),
            next_agent_id: Cell::new(INITIAL_AGENT_ID),
            lineage,
            access_state: Cell::new(AccessState::Shared {
                active_operations: 0,
            }),
            topology: RefCell::new(topology),
            resources: RefCell::new(BTreeMap::new()),
            shared: RefCell::new(shared),
            shared_limits,
        });

        Ok(world)
    }

    /// Borrow one read guard for simulation.
    pub fn read_simulation(&self) -> Ref<'_, Simulation> {
        self.simulation.borrow()
    }

    /// Borrow one write guard for simulation.
    pub fn write_simulation(&self) -> RefMut<'_, Simulation> {
        self.simulation.borrow_mut()
    }

    /// Snapshot world policy state.
    pub fn policy(&self) -> Policy {
        self.policy.borrow().spec.clone()
    }

    /// Snapshot world entity kind definitions.
    pub fn entity_kinds(&self) -> BTreeMap<WorldEntityKind, WorldEntityKindDefinition> {
        self.topology.borrow().entity_kinds().clone()
    }

    /// Snapshot world edge kind definitions.
    pub fn edge_kinds(&self) -> BTreeMap<WorldEdgeKind, WorldEdgeKindDefinition> {
        self.topology.borrow().edge_kinds().clone()
    }

    /// Snapshot world entities.
    pub fn entities(&self) -> BTreeMap<WorldEntityId, WorldEntity> {
        self.topology.borrow().entities().clone()
    }

    /// Return labels for one live runtime.
    pub fn runtime_labels(&self, runtime_id: RuntimeId) -> RuntimeResult<BTreeMap<String, String>> {
        let entity_id = format!("runtime.{}", runtime_id.0);
        let topology = self.topology.borrow();
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
        let topology = self.topology.borrow();
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
        self.topology.borrow().edges().clone()
    }

    /// Snapshot logical world resources.
    pub fn resources(&self) -> BTreeMap<WorldResourceId, WorldResource> {
        self.resources.borrow().clone()
    }

    /// Allocate one runtime identifier.
    pub(crate) fn allocate_runtime_id(&self) -> RuntimeId {
        let runtime_id = self.next_runtime_id.get();
        self.next_runtime_id.set(runtime_id.saturating_add(1));

        RuntimeId(runtime_id)
    }

    /// Allocate one agent identifier.
    pub(crate) fn allocate_agent_id(&self) -> AgentId {
        let agent_id = self.next_agent_id.get();
        self.next_agent_id.set(agent_id.saturating_add(1));

        AgentId(agent_id)
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
    pub fn observations(&self) -> &ObservationLog {
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

impl Default for World {
    /// Create a world with empty policy rules.
    fn default() -> Self {
        let options = RuntimeOptions::default();
        match Self::new(&options, None) {
            Ok(world) => match Arc::try_unwrap(world) {
                Ok(world) => world,
                Err(_) => panic!("default world should not retain extra references"),
            },
            Err(error) => {
                panic!("default world construction should succeed: {error:?}");
            }
        }
    }
}
