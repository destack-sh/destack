use destack_artifact::ConditionSet;
use destack_core::Blob;
use destack_program as program;
use destack_repository::{Environment, RuntimeOptions};
use destack_rpc::service;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::machine::Entry;
use crate::worker::WorkerId;
use crate::world::{
    Branch, BranchId, Checkpoint, CheckpointId, Instant, Moment, Run, RunOutcome, RuntimeId,
};

/// RPC operations over hosted Worlds.
#[service(name = "destack.world.World")]
pub trait WorldService {
    // =============================================================================
    // World
    // =============================================================================

    /// Read one World's current Moment.
    #[rpc(name = "ReadMoment", idempotency = "no_side_effects")]
    fn read_moment(request: ReadMomentRequest) -> Moment;

    // =============================================================================
    // Runtime
    // =============================================================================

    /// Spawn one Runtime from an encoded Program Blob.
    #[rpc(name = "SpawnRuntime")]
    fn spawn_runtime(request: SpawnRuntimeRequest) -> RuntimeId;

    /// List the Runtime identifiers in one World.
    #[rpc(name = "ListRuntimes", idempotency = "no_side_effects")]
    fn list_runtimes(request: ListRuntimesRequest) -> Vec<RuntimeId>;

    /// Remove one Runtime from its World.
    #[rpc(name = "RemoveRuntime", idempotency = "idempotent")]
    fn remove_runtime(request: RemoveRuntimeRequest) -> ();

    // =============================================================================
    // Execution
    // =============================================================================

    /// Invoke one Runtime entrypoint.
    #[rpc(name = "Invoke")]
    fn invoke(request: InvokeRequest) -> program::Value;

    /// Run one unit of World execution.
    #[rpc(name = "Run")]
    fn run(request: RunRequest) -> RunOutcome;

    /// Resume one exact debugger-stopped Worker.
    #[rpc(name = "Resume")]
    fn resume(request: ResumeRequest) -> RunOutcome;

    /// Advance one runtime-controlled World clock.
    #[rpc(name = "Advance")]
    fn advance(request: AdvanceRequest) -> Instant;

    // =============================================================================
    // Branch
    // =============================================================================

    /// Read one Branch in a World's lineage.
    #[rpc(name = "ReadBranch", idempotency = "no_side_effects")]
    fn read_branch(request: ReadBranchRequest) -> Branch;

    /// List the Branches in a World's lineage.
    #[rpc(name = "ListBranches", idempotency = "no_side_effects")]
    fn list_branches(request: ListBranchesRequest) -> Vec<Branch>;

    /// Rewind one World to a committed Moment.
    #[rpc(name = "Rewind")]
    fn rewind(request: RewindRequest) -> ();

    /// Fork one hosted World from a committed Moment.
    #[rpc(name = "Fork")]
    fn fork(request: ForkRequest) -> WorldId;

    // =============================================================================
    // Checkpoint
    // =============================================================================

    /// Read one Checkpoint in a World's lineage.
    #[rpc(name = "ReadCheckpoint", idempotency = "no_side_effects")]
    fn read_checkpoint(request: ReadCheckpointRequest) -> Checkpoint;

    /// List the Checkpoints in a World's lineage.
    #[rpc(name = "ListCheckpoints", idempotency = "no_side_effects")]
    fn list_checkpoints(request: ListCheckpointsRequest) -> Vec<Checkpoint>;

    /// Create one Checkpoint for a World.
    #[rpc(name = "CreateCheckpoint")]
    fn create_checkpoint(request: CreateCheckpointRequest) -> CheckpointId;

    // =============================================================================
    // Snapshot
    // =============================================================================

    /// Store one Checkpoint snapshot as a Blob.
    #[rpc(name = "Snapshot")]
    fn snapshot(request: SnapshotRequest) -> Blob;

    /// Restore one World from a snapshot Blob.
    #[rpc(name = "Restore")]
    fn restore(request: RestoreRequest) -> WorldId;
}

/// Request to read one World's current Moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReadMomentRequest {
    /// Hosted World.
    pub world_id: WorldId,
}

/// Request to spawn one Runtime in a World.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SpawnRuntimeRequest {
    /// World receiving the Runtime.
    pub world_id: WorldId,
    /// Encoded Program Blob.
    pub program: Blob,
    /// Runtime configuration.
    pub options: Option<RuntimeOptions>,
    /// Ambient Runtime environment.
    pub environment: Option<Environment>,
    /// Active Program conditions.
    pub conditions: ConditionSet,
}

/// Request to list the Runtimes in one World.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ListRuntimesRequest {
    /// Hosted World.
    pub world_id: WorldId,
}

/// Request to remove one Runtime from a World.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct RemoveRuntimeRequest {
    /// World containing the Runtime.
    pub world_id: WorldId,
    /// Runtime to remove.
    pub runtime_id: RuntimeId,
}

/// Request to invoke one Runtime entrypoint.
#[derive(Debug, Serialize, Deserialize, Reflect)]
pub struct InvokeRequest {
    /// World containing the Runtime.
    pub world_id: WorldId,
    /// Runtime receiving the invocation.
    pub runtime_id: RuntimeId,
    /// Program entrypoint to invoke.
    pub entry: Entry,
    /// Entrypoint arguments.
    pub arguments: Vec<program::Value>,
}

/// Request to run one bounded World execution operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct RunRequest {
    /// World to run.
    pub world_id: WorldId,
    /// Bounded execution operation to perform.
    pub run: Run,
}

/// Request to resume one debugger-stopped Worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ResumeRequest {
    /// World containing the stopped Worker.
    pub world_id: WorldId,
    /// Runtime containing the stopped Worker.
    pub runtime_id: RuntimeId,
    /// Worker to resume.
    pub worker_id: WorkerId,
}

/// Request to advance one runtime-controlled World clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct AdvanceRequest {
    /// World whose clock advances.
    pub world_id: WorldId,
    /// Absolute World deadline to reach.
    pub deadline: Instant,
}

/// Request to read one Branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReadBranchRequest {
    /// World containing the Branch.
    pub world_id: WorldId,
    /// Branch to read.
    pub branch_id: BranchId,
}

/// Request to list the Branches in one World.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ListBranchesRequest {
    /// Hosted World.
    pub world_id: WorldId,
}

/// Request to rewind one World.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct RewindRequest {
    /// World to rewind.
    pub world_id: WorldId,
    /// Committed Moment to restore.
    pub moment: Moment,
}

/// Request to fork one hosted World.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ForkRequest {
    /// World containing the fork Moment.
    pub world_id: WorldId,
    /// Committed Moment to fork.
    pub moment: Moment,
    /// Child Branch name.
    pub name: String,
}

/// Request to read one Checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReadCheckpointRequest {
    /// World containing the Checkpoint.
    pub world_id: WorldId,
    /// Checkpoint to read.
    pub checkpoint_id: CheckpointId,
}

/// Request to list the Checkpoints in one World.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ListCheckpointsRequest {
    /// Hosted World.
    pub world_id: WorldId,
}

/// Request to create one Checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CreateCheckpointRequest {
    /// World receiving the Checkpoint.
    pub world_id: WorldId,
    /// Checkpoint name.
    pub name: String,
}

/// Request to store one Checkpoint snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SnapshotRequest {
    /// World containing the Checkpoint.
    pub world_id: WorldId,
    /// Checkpoint to snapshot.
    pub checkpoint_id: CheckpointId,
}

/// Request to restore one World snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct RestoreRequest {
    /// Snapshot Blob to restore.
    pub snapshot: Blob,
}

/// Stable identifier for one hosted World.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct WorldId(u64);

impl WorldId {
    /// Create one World identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw World identifier.
    pub const fn get(self) -> u64 {
        self.0
    }
}
