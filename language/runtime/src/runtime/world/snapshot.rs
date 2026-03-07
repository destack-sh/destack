use std::collections::BTreeMap;
use std::sync::atomic::Ordering;

use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::ResourceSnapshot;
use crate::runtime::engine::EngineSnapshot;
use crate::runtime::policy::Policy;
use crate::runtime::random::{RandomStreamId, ScopedRandomStreamKey};
use crate::runtime::time::WorldInstant;
use crate::runtime::topology::Topology;
use crate::runtime::{AgentId, DropCounts};
use crate::simulation::Simulation;
use destack_workspace::RuntimeOptions;

use super::{RuntimeId, World, WorldResource, WorldResourceId};

/// Snapshot identifier for one durable world snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SnapshotId(u128);

impl SnapshotId {
    /// Create a new snapshot identifier.
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    /// Return the raw snapshot identifier value.
    pub const fn get(self) -> u128 {
        self.0
    }
}

/// Snapshot payload for one durable world restore point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    /// The snapshot identifier.
    pub id: SnapshotId,
    /// The durable snapshot payload.
    pub payload: SnapshotPayload,
}

/// Durable snapshot payload captured at one checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotPayload {
    /// Captured world-owned state.
    pub world: WorldSnapshot,
    /// Captured runtime metadata keyed by runtime id.
    pub runtimes: BTreeMap<RuntimeId, RuntimeSnapshot>,
    /// Captured agent metadata keyed by agent id.
    pub agents: BTreeMap<AgentId, AgentSnapshot>,
    /// Captured engine state keyed by agent id.
    pub engines: BTreeMap<AgentId, EngineSnapshot>,
}

/// Durable world-owned state captured at one checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldSnapshot {
    /// The next runtime id to allocate after restore.
    pub next_runtime_id: u64,
    /// The next agent id to allocate after restore.
    pub next_agent_id: u64,
    /// The world mutation revision at this snapshot.
    pub revision: u64,
    /// Captured dynamic policy state.
    pub policy: Policy,
    /// Captured topology metadata graph.
    pub topology: Topology,
    /// Captured logical world resources.
    pub resources: BTreeMap<WorldResourceId, WorldResource>,
    /// Captured simulation state.
    pub simulation: Simulation,
    /// Captured world clock state.
    pub clock: ClockSnapshot,
    /// Captured world random state.
    pub random: RandomSnapshot,
}

/// Durable clock state captured at one checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockSnapshot {
    /// Captured virtual wall-clock instant.
    pub virtual_wall: WorldInstant,
    /// Captured virtual monotonic instant.
    pub virtual_mono: WorldInstant,
}

/// Durable random state captured at one checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomSnapshot {
    /// Captured deterministic root seed.
    pub root_seed: u64,
    /// Captured deterministic default stream state.
    pub default_stream: Vec<u8>,
    /// Captured deterministic next stream identifier.
    pub next_stream_id: u64,
    /// Captured deterministic user stream states.
    pub streams: BTreeMap<RandomStreamId, Vec<u8>>,
    /// Captured scoped stream bindings.
    pub scoped_streams: BTreeMap<ScopedRandomStreamKey, RandomStreamId>,
}

/// Durable runtime metadata captured at one checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSnapshot {
    /// Runtime identifier in the world.
    pub runtime_id: RuntimeId,
    /// Primary agent identifier for this runtime.
    pub primary_agent_id: AgentId,
    /// Runtime display name.
    pub name: String,
    /// Runtime launch arguments.
    pub platform_args: Vec<String>,
    /// Runtime options captured for reconstruction.
    pub options: RuntimeOptions,
    /// Runtime drop counts.
    pub drop_counts: DropCounts,
}

/// Durable agent metadata captured at one checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSnapshot {
    /// Agent identifier in the world.
    pub agent_id: AgentId,
    /// Owning runtime identifier.
    pub runtime_id: RuntimeId,
    /// Agent display name.
    pub name: String,
    /// Agent launch arguments.
    pub platform_args: Vec<String>,
    /// Agent options captured for reconstruction.
    pub options: RuntimeOptions,
    /// Agent drop counts.
    pub drop_counts: DropCounts,
    /// Captured durable resource payloads for this agent.
    pub resources: Vec<ResourceSnapshot>,
}

impl World {
    /// Return metadata for one stored snapshot.
    pub fn snapshot_info(&self, snapshot_id: SnapshotId) -> RuntimeResult<Snapshot> {
        let lineage = self.lineage.read();
        let snapshot = lineage.snapshots.get(&snapshot_id).ok_or_else(|| {
            RuntimeError::Internal {
                message: format!("snapshot {} does not exist", snapshot_id.get()),
            }
            .boxed()
        })?;

        Ok(snapshot.as_ref().clone())
    }

    /// Return identifiers for all stored snapshots in stable order.
    pub fn snapshot_ids(&self) -> Vec<SnapshotId> {
        self.lineage.read().snapshots.keys().copied().collect()
    }

    /// Capture one durable snapshot payload while the world is checkpoint-ready.
    pub(crate) fn capture_snapshot(&self) -> RuntimeResult<SnapshotPayload> {
        self.require_checkpoint_ready()?;
        let _next_runtime_id = self.next_runtime_id.load(Ordering::SeqCst);
        let _next_agent_id = self.next_agent_id.load(Ordering::SeqCst);
        let _revision = self.revision.load(Ordering::SeqCst);

        Err(RuntimeError::Internal {
            message: "world snapshot capture is not implemented yet".to_string(),
        }
        .boxed())
    }

    /// Restore one durable snapshot payload while the world is checkpoint-ready.
    pub(crate) fn restore_snapshot(&self, snapshot: &Snapshot) -> RuntimeResult<()> {
        self.require_checkpoint_ready()?;
        let _ = snapshot;

        Err(RuntimeError::Internal {
            message: "world snapshot restore is not implemented yet".to_string(),
        }
        .boxed())
    }
}
