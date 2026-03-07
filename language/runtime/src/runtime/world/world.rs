use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::runtime::bindings::BindingReplayPayload;
use crate::runtime::policy::{Policy, PolicyState};
use crate::runtime::random::{Random, RandomStreamId};
use crate::runtime::replay::{Replay, ReplayHeader};
use crate::runtime::time::{Clock, HostClockSource, Nanos};
use crate::runtime::{AgentId, Runtime};
use crate::simulation::Simulation;
use destack_workspace::{ExecutionMode, RandomMode, ReplayPayloadMode, RuntimeOptions, TimeMode};

use super::lineage::{Lineage, ROOT_BRANCH_ID};
use super::topology::Topology;
pub use super::topology::{
    RuntimeId, WorldEdge, WorldEdgeId, WorldEdgeKind, WorldEdgeKindDefinition, WorldEntity,
    WorldEntityId, WorldEntityKind, WorldEntityKindDefinition,
};
use super::{
    BranchId, CheckpointState, INITIAL_AGENT_ID, INITIAL_CONTROL_REVISION, INITIAL_RUNTIME_ID,
    WorldResource, WorldResourceId,
};

/// Number of bytes in a megabyte for replay chunk sizing.
const BYTES_PER_MB: u64 = 1024 * 1024;

/// Shared deterministic runtime world.
#[derive(Debug)]
pub struct World {
    /// Active branch identifier for this live world instance.
    pub(super) branch_id: BranchId,
    /// Live runtimes owned by this world.
    pub(super) runtimes: RwLock<BTreeMap<RuntimeId, Box<Runtime>>>,
    /// Shared simulation state for all agents using this world.
    pub(super) simulation: RwLock<Simulation>,

    /// Effective world time mode after execution-mode resolution.
    pub(super) time_mode: TimeMode,
    /// Effective world random mode after execution-mode resolution.
    pub(super) random_mode: RandomMode,
    /// Shared world clock.
    pub(super) clock: Clock,
    /// Shared world randomness state.
    pub(super) random: Random,
    /// Replay controller for deterministic world event history.
    pub(super) replay: Replay,
    /// Active policy state.
    pub(super) policy: RwLock<PolicyState>,
    /// One global mutation gate for replayable world control changes.
    pub(super) mutation_lock: Mutex<()>,
    /// The next runtime id to allocate.
    pub(super) next_runtime_id: AtomicU64,
    /// The next agent id to allocate.
    pub(super) next_agent_id: AtomicU64,
    /// The current world command revision.
    pub(super) revision: AtomicU64,
    /// World-owned lineage and durable restore metadata.
    pub(super) lineage: Arc<RwLock<Lineage>>,
    /// World-owned checkpoint readiness state.
    pub(super) checkpoint_state: RwLock<CheckpointState>,
    /// Topology registry for world metadata.
    pub(super) topology: RwLock<Topology>,
    /// Logical resource records keyed by world resource identifier.
    pub(super) resources: RwLock<BTreeMap<WorldResourceId, WorldResource>>,
}

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
        let mut replay_header = ReplayHeader {
            execution_mode: options.execution,
            branch_id: ROOT_BRANCH_ID,
            replay_payload,
            ..ReplayHeader::default()
        };
        if let Some(chunk_size_mb) = options.replay.chunk_size_mb {
            let chunk_bytes = chunk_size_mb.saturating_mul(BYTES_PER_MB);
            if chunk_bytes > 0 {
                replay_header.max_chunk_bytes = chunk_bytes;
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
        let replay = Replay::new(options.execution, replay_header);
        let topology = Topology::new();
        policy.validate_with_kind_catalog(&topology)?;

        // final world state
        Ok(Arc::new(Self {
            branch_id: ROOT_BRANCH_ID,
            runtimes: RwLock::new(BTreeMap::new()),
            simulation: RwLock::new(Simulation::default()),
            time_mode,
            random_mode,
            clock,
            random,
            replay,
            policy: RwLock::new(PolicyState::new(policy)),
            mutation_lock: Mutex::new(()),
            next_runtime_id: AtomicU64::new(INITIAL_RUNTIME_ID),
            next_agent_id: AtomicU64::new(INITIAL_AGENT_ID),
            revision: AtomicU64::new(INITIAL_CONTROL_REVISION),
            lineage: Arc::new(RwLock::new(Lineage::default())),
            checkpoint_state: RwLock::new(CheckpointState::Ready),
            topology: RwLock::new(topology),
            resources: RwLock::new(BTreeMap::new()),
        }))
    }

    /// Borrow one read guard for simulation.
    pub fn read_simulation(&self) -> RwLockReadGuard<'_, Simulation> {
        self.simulation.read()
    }

    /// Borrow one write guard for simulation.
    pub fn write_simulation(&self) -> RwLockWriteGuard<'_, Simulation> {
        self.simulation.write()
    }

    /// Snapshot world policy state.
    pub fn policy(&self) -> Policy {
        self.policy.read().spec.clone()
    }

    /// Snapshot world entity kind definitions.
    pub fn entity_kinds(&self) -> BTreeMap<WorldEntityKind, WorldEntityKindDefinition> {
        self.topology.read().entity_kinds().clone()
    }

    /// Snapshot world edge kind definitions.
    pub fn edge_kinds(&self) -> BTreeMap<WorldEdgeKind, WorldEdgeKindDefinition> {
        self.topology.read().edge_kinds().clone()
    }

    /// Snapshot world entities.
    pub fn entities(&self) -> BTreeMap<WorldEntityId, WorldEntity> {
        self.topology.read().entities().clone()
    }

    /// Snapshot world edges.
    pub fn edges(&self) -> BTreeMap<WorldEdgeId, WorldEdge> {
        self.topology.read().edges().clone()
    }

    /// Snapshot logical world resources.
    pub fn resources(&self) -> BTreeMap<WorldResourceId, WorldResource> {
        self.resources.read().clone()
    }

    /// Return the current world command revision.
    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::SeqCst)
    }

    /// Allocate one runtime identifier.
    pub(crate) fn allocate_runtime_id(&self) -> RuntimeId {
        let runtime_id = self.next_runtime_id.fetch_add(1, Ordering::SeqCst);

        RuntimeId(runtime_id)
    }

    /// Allocate one agent identifier.
    pub(crate) fn allocate_agent_id(&self) -> AgentId {
        let agent_id = self.next_agent_id.fetch_add(1, Ordering::SeqCst);

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

    /// Borrow the shared replay controller.
    pub fn replay(&self) -> &Replay {
        &self.replay
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
