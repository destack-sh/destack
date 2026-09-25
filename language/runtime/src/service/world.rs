use serde::{Deserialize, Serialize};
use tspp_artifact::ConditionSet;
use tspp_core::Blob;
use tspp_program as program;
use tspp_repository::{Environment, RuntimeOptions};
use tspp_rpc::service;
use tspp_serde::Reflect;

use crate::machine::Entry;
use crate::runtime::{self, RuntimeId};
use crate::world::observation::{ObservationEntry, ObservationQuery, ObservationSequence};
use crate::world::{
    Branch, BranchId, Edge, EdgeDefinition, Entity, EntityDefinition, Image, ImageId, Moment,
    Policy, Rule, RuleId, Run, RunOutcome, Snapshot,
};

/// RPC operations over hosted Worlds.
#[service(name = "tspp.world.World")]
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

    /// Read one Runtime in a World.
    #[rpc(name = "ReadRuntime", idempotency = "no_side_effects")]
    fn read_runtime(request: ReadRuntimeRequest) -> Runtime;

    /// List the Runtimes in one World.
    #[rpc(name = "ListRuntimes", idempotency = "no_side_effects")]
    fn list_runtimes(request: ListRuntimesRequest) -> Vec<Runtime>;

    /// Spawn one Runtime from an encoded Program Blob.
    #[rpc(name = "SpawnRuntime")]
    fn spawn_runtime(request: SpawnRuntimeRequest) -> Runtime;

    /// Replace one Runtime's Program at a committed safepoint.
    #[rpc(name = "ReloadRuntime")]
    fn reload_runtime(request: ReloadRuntimeRequest) -> Runtime;

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

    // =============================================================================
    // Observation
    // =============================================================================

    /// List one page of World Observations.
    #[rpc(name = "ListObservations", idempotency = "no_side_effects")]
    fn list_observations(request: ListObservationsRequest) -> ObservationPage;

    /// Watch World Observations after one sequence.
    #[rpc(name = "WatchObservations", response_stream(ObservationEntry))]
    fn watch_observations(request: WatchObservationsRequest) -> ();

    // =============================================================================
    // Topology
    // =============================================================================

    /// Read one World's Topology.
    #[rpc(name = "ReadTopology", idempotency = "no_side_effects")]
    fn read_topology(request: ReadTopologyRequest) -> Topology;

    // =============================================================================
    // Policy
    // =============================================================================

    /// Read one World's Policy.
    #[rpc(name = "ReadPolicy", idempotency = "no_side_effects")]
    fn read_policy(request: ReadPolicyRequest) -> Policy;

    /// Replace one World's active Policy.
    #[rpc(name = "ReplacePolicy")]
    fn replace_policy(request: ReplacePolicyRequest) -> ();

    /// Add one Rule to a World's active Policy.
    #[rpc(name = "AddRule")]
    fn add_rule(request: AddRuleRequest) -> ();

    /// Replace one Rule in a World's active Policy.
    #[rpc(name = "ReplaceRule")]
    fn replace_rule(request: ReplaceRuleRequest) -> ();

    /// Remove one Rule from a World's active Policy.
    #[rpc(name = "RemoveRule", idempotency = "idempotent")]
    fn remove_rule(request: RemoveRuleRequest) -> ();

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
    // Image
    // =============================================================================

    /// Read one retained Image in a World's lineage.
    #[rpc(name = "ReadImage", idempotency = "no_side_effects")]
    fn read_image(request: ReadImageRequest) -> Image;

    /// List the retained Images in a World's lineage.
    #[rpc(name = "ListImages", idempotency = "no_side_effects")]
    fn list_images(request: ListImagesRequest) -> Vec<Image>;

    /// Capture one Image for a World.
    #[rpc(name = "Capture")]
    fn capture(request: CaptureRequest) -> Image;

    // =============================================================================
    // Snapshot
    // =============================================================================

    /// Store one retained Image as a Blob-backed Snapshot.
    #[rpc(name = "Snapshot")]
    fn snapshot(request: SnapshotRequest) -> Snapshot;
}

/// Stable identifier for one hosted World.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct WorldId(u64);

/// One Runtime hosted inside a World.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Runtime {
    /// World-local Runtime identity.
    pub id: RuntimeId,
    /// Exact Program Blob instantiated by the Runtime.
    pub program: Blob,
    /// Runtime configuration.
    pub options: RuntimeOptions,
    /// Ambient Runtime environment.
    pub environment: Environment,
    /// Active Program conditions.
    pub conditions: ConditionSet,
}

/// One materialized World topology.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Topology {
    /// Registered entity definitions.
    pub entity_definitions: Vec<EntityDefinition>,
    /// Registered edge definitions.
    pub edge_definitions: Vec<EdgeDefinition>,
    /// Live topology entities.
    pub entities: Vec<Entity>,
    /// Live topology edges.
    pub edges: Vec<Edge>,
}

/// One page of World Observations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ObservationPage {
    /// Matching Observations in sequence order.
    pub observations: Vec<ObservationEntry>,
    /// Sequence preceding the next page when more Observations remain.
    pub next: Option<ObservationSequence>,
}

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

impl From<&runtime::Runtime> for Runtime {
    /// Capture one Runtime's client-visible configuration.
    fn from(runtime: &runtime::Runtime) -> Self {
        Self {
            id: runtime.runtime_id(),
            program: runtime.program().blob(),
            options: runtime.options().clone(),
            environment: runtime.environment().clone(),
            conditions: runtime.conditions().clone(),
        }
    }
}

/// Request to read one World's current Moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReadMomentRequest {
    /// Hosted World.
    pub world_id: WorldId,
}

/// Request to read one Runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReadRuntimeRequest {
    /// World containing the Runtime.
    pub world_id: WorldId,
    /// Runtime to read.
    pub runtime_id: RuntimeId,
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

/// Request to replace one Runtime's Program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReloadRuntimeRequest {
    /// World containing the Runtime.
    pub world_id: WorldId,
    /// Runtime receiving the Program.
    pub runtime_id: RuntimeId,
    /// Replacement Program Blob.
    pub program: Blob,
    /// Active conditions for the replacement Program.
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

/// Request to list one page of World Observations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ListObservationsRequest {
    /// World to inspect.
    pub world_id: WorldId,
    /// Observation selection.
    pub query: ObservationQuery,
    /// Maximum number of Observations to return.
    pub limit: u32,
}

/// Request to watch World Observations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct WatchObservationsRequest {
    /// World to watch.
    pub world_id: WorldId,
    /// Observation selection.
    pub query: ObservationQuery,
}

/// Request to read one World's Topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReadTopologyRequest {
    /// World to inspect.
    pub world_id: WorldId,
    /// Committed Moment to inspect, or the current World when absent.
    pub moment: Option<Moment>,
}

/// Request to read one World's Policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReadPolicyRequest {
    /// World to inspect.
    pub world_id: WorldId,
    /// Committed Moment to inspect, or the current World when absent.
    pub moment: Option<Moment>,
}

/// Request to replace one World's active Policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReplacePolicyRequest {
    /// World receiving the Policy.
    pub world_id: WorldId,
    /// Complete replacement Policy.
    pub policy: Policy,
}

/// Request to add one Rule to a World's active Policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct AddRuleRequest {
    /// World receiving the Rule.
    pub world_id: WorldId,
    /// Rule to add.
    pub rule: Rule,
}

/// Request to replace one Rule in a World's active Policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReplaceRuleRequest {
    /// World containing the Rule.
    pub world_id: WorldId,
    /// Complete replacement Rule.
    pub rule: Rule,
}

/// Request to remove one Rule from a World's active Policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct RemoveRuleRequest {
    /// World containing the Rule.
    pub world_id: WorldId,
    /// Rule to remove.
    pub rule_id: RuleId,
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

/// Request to read one retained Image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReadImageRequest {
    /// World containing the Image.
    pub world_id: WorldId,
    /// Image to read.
    pub image_id: ImageId,
}

/// Request to list the retained Images in one World.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ListImagesRequest {
    /// Hosted World.
    pub world_id: WorldId,
}

/// Request to capture one Image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CaptureRequest {
    /// World receiving the Image.
    pub world_id: WorldId,
    /// Image name.
    pub name: String,
}

/// Request to store one retained Image as a Blob-backed Snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SnapshotRequest {
    /// World containing the Image.
    pub world_id: WorldId,
    /// Image to snapshot.
    pub image_id: ImageId,
}
