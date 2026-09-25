use serde::{Deserialize, Serialize};
use tspp_memory::MemoryRange;
use tspp_program as program;
use tspp_rpc::service;
use tspp_serde::Reflect;

use crate::debugger::{
    Allocation, AllocationId, Breakpoint, EvaluationLimits, EvaluationMode, EvaluationOutcome,
    EvaluationTarget, Execution, Frame, FrameId, MemoryFilter, MemoryMap, PointFilter, Probe,
    ProbeAction, ProbeFilter, ProbeId, Reference, Root, RootId, Step, Watchpoint,
};
use crate::runtime::RuntimeId;
use crate::worker::WorkerId;
use crate::world::{Moment, RunOutcome};

use super::WorldId;

/// RPC operations over hosted World debuggers.
#[service(name = "tspp.world.Debugger")]
pub trait DebuggerService {
    // =============================================================================
    // Execution
    // =============================================================================

    /// Pause selected World execution.
    #[rpc(name = "Pause")]
    fn pause(request: PauseRequest) -> ();

    /// Resume one debugger-stopped Worker.
    #[rpc(name = "Resume")]
    fn resume(request: ResumeRequest) -> RunOutcome;

    /// Step one debugger-stopped Frame.
    #[rpc(name = "Step")]
    fn step(request: StepRequest) -> RunOutcome;

    // =============================================================================
    // Evaluation
    // =============================================================================

    /// Evaluate one TS++ expression in a selected execution context.
    #[rpc(name = "Evaluate")]
    fn evaluate(request: EvaluateRequest) -> Evaluation;

    // =============================================================================
    // Frame
    // =============================================================================

    /// Read captured Frames from one World.
    #[rpc(name = "ReadFrames", idempotency = "no_side_effects")]
    fn read_frames(request: ReadFramesRequest) -> Vec<Frame>;

    // =============================================================================
    // Memory
    // =============================================================================

    /// Read one World's mapped memory Regions.
    #[rpc(name = "ReadMemoryMap", idempotency = "no_side_effects")]
    fn read_memory_map(request: ReadMemoryMapRequest) -> MemoryMap;

    /// Stream one exact World memory range.
    #[rpc(name = "ReadMemory", response_stream(Vec<u8>), idempotency = "no_side_effects")]
    fn read_memory(request: ReadMemoryRequest) -> ();

    // =============================================================================
    // Heap
    // =============================================================================

    /// Stream managed heap Allocations in one World.
    #[rpc(
        name = "ListAllocations",
        response_stream(Vec<Allocation>),
        idempotency = "no_side_effects"
    )]
    fn list_allocations(request: ListAllocationsRequest) -> ();

    /// Stream heap Roots in one World.
    #[rpc(
        name = "ListRoots",
        response_stream(Vec<Root>),
        idempotency = "no_side_effects"
    )]
    fn list_roots(request: ListRootsRequest) -> ();

    /// Stream outgoing References from one heap Allocation.
    #[rpc(
        name = "ListReferences",
        response_stream(Vec<Reference>),
        idempotency = "no_side_effects"
    )]
    fn list_references(request: ListReferencesRequest) -> ();

    // =============================================================================
    // Breakpoint
    // =============================================================================

    /// List the Breakpoints in one World.
    #[rpc(name = "ListBreakpoints", idempotency = "no_side_effects")]
    fn list_breakpoints(request: ListBreakpointsRequest) -> Vec<Breakpoint>;

    /// Add one Breakpoint to a World.
    #[rpc(name = "AddBreakpoint")]
    fn add_breakpoint(request: AddBreakpointRequest) -> program::BreakpointId;

    /// Update one Breakpoint in a World.
    #[rpc(name = "UpdateBreakpoint")]
    fn update_breakpoint(request: UpdateBreakpointRequest) -> ();

    /// Remove one Breakpoint from a World.
    #[rpc(name = "RemoveBreakpoint", idempotency = "idempotent")]
    fn remove_breakpoint(request: RemoveBreakpointRequest) -> ();

    // =============================================================================
    // Watchpoint
    // =============================================================================

    /// List the Watchpoints in one World.
    #[rpc(name = "ListWatchpoints", idempotency = "no_side_effects")]
    fn list_watchpoints(request: ListWatchpointsRequest) -> Vec<Watchpoint>;

    /// Add one Watchpoint to a World.
    #[rpc(name = "AddWatchpoint")]
    fn add_watchpoint(request: AddWatchpointRequest) -> program::WatchpointId;

    /// Update one Watchpoint in a World.
    #[rpc(name = "UpdateWatchpoint")]
    fn update_watchpoint(request: UpdateWatchpointRequest) -> ();

    /// Remove one Watchpoint from a World.
    #[rpc(name = "RemoveWatchpoint", idempotency = "idempotent")]
    fn remove_watchpoint(request: RemoveWatchpointRequest) -> ();

    // =============================================================================
    // Probe
    // =============================================================================

    /// List the Probes in one World.
    #[rpc(name = "ListProbes", idempotency = "no_side_effects")]
    fn list_probes(request: ListProbesRequest) -> Vec<Probe>;

    /// Add one Probe to a World.
    #[rpc(name = "AddProbe")]
    fn add_probe(request: AddProbeRequest) -> ProbeId;

    /// Update one Probe in a World.
    #[rpc(name = "UpdateProbe")]
    fn update_probe(request: UpdateProbeRequest) -> ();

    /// Remove one Probe from a World.
    #[rpc(name = "RemoveProbe", idempotency = "idempotent")]
    fn remove_probe(request: RemoveProbeRequest) -> ();
}

/// Result of one hosted expression evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Evaluation {
    /// World that performed the evaluation.
    pub world_id: WorldId,
    /// Moment containing the evaluation outcome.
    pub moment: Moment,
    /// Evaluation outcome.
    pub outcome: EvaluationOutcome,
}

/// Request to pause selected World execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct PauseRequest {
    /// World containing the selected execution.
    pub world_id: WorldId,
    /// Execution to pause.
    pub execution: Execution,
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

/// Request to step one debugger-stopped Frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct StepRequest {
    /// World containing the stopped Frame.
    pub world_id: WorldId,
    /// Frame to step.
    pub frame_id: FrameId,
    /// Step granularity.
    pub step: Step,
}

/// Request to evaluate one TS++ expression.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct EvaluateRequest {
    /// World containing the selected context.
    pub world_id: WorldId,
    /// Committed Moment to inspect, or the current World when absent.
    pub moment: Option<Moment>,
    /// Evaluation context.
    pub target: EvaluationTarget,
    /// TS++ source expression.
    pub expression: String,
    /// Mutation behavior.
    pub mode: EvaluationMode,
    /// Evaluation resource limits.
    pub limits: EvaluationLimits,
}

/// Request to read captured Frames from one World.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReadFramesRequest {
    /// World to inspect.
    pub world_id: WorldId,
    /// Committed Moment to inspect, or the current World when absent.
    pub moment: Option<Moment>,
    /// Worker to inspect, or every Worker when absent.
    pub worker_id: Option<WorkerId>,
}

/// Request to read one World's mapped memory Regions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReadMemoryMapRequest {
    /// World to inspect.
    pub world_id: WorldId,
    /// Committed Moment to inspect, or the current World when absent.
    pub moment: Option<Moment>,
}

/// Request to read one exact World memory range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReadMemoryRequest {
    /// World to inspect.
    pub world_id: WorldId,
    /// Exact committed Moment to inspect.
    pub moment: Moment,
    /// Exact logical memory range to read.
    pub range: MemoryRange,
}

/// Request to list managed heap Allocations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ListAllocationsRequest {
    /// World to inspect.
    pub world_id: WorldId,
    /// Exact committed Moment to inspect.
    pub moment: Moment,
    /// Allocation preceding the requested stream.
    pub after: Option<AllocationId>,
}

/// Request to list heap Roots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ListRootsRequest {
    /// World to inspect.
    pub world_id: WorldId,
    /// Exact committed Moment to inspect.
    pub moment: Moment,
    /// Root preceding the requested stream.
    pub after: Option<RootId>,
}

/// Request to list outgoing References from one Allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ListReferencesRequest {
    /// World to inspect.
    pub world_id: WorldId,
    /// Exact committed Moment to inspect.
    pub moment: Moment,
    /// Source Allocation.
    pub allocation_id: AllocationId,
    /// Reference byte offset preceding the requested stream.
    pub after: Option<u64>,
}

/// Request to list the Breakpoints in one World.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ListBreakpointsRequest {
    /// World to inspect.
    pub world_id: WorldId,
    /// Committed Moment to inspect, or the current World when absent.
    pub moment: Option<Moment>,
}

/// Request to add one Breakpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct AddBreakpointRequest {
    /// World receiving the Breakpoint.
    pub world_id: WorldId,
    /// Executable Program point selection.
    pub filter: PointFilter,
}

/// Request to update one Breakpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct UpdateBreakpointRequest {
    /// World containing the Breakpoint.
    pub world_id: WorldId,
    /// Complete replacement Breakpoint.
    pub breakpoint: Breakpoint,
}

/// Request to remove one Breakpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct RemoveBreakpointRequest {
    /// World containing the Breakpoint.
    pub world_id: WorldId,
    /// Breakpoint to remove.
    pub breakpoint_id: program::BreakpointId,
}

/// Request to list the Watchpoints in one World.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ListWatchpointsRequest {
    /// World to inspect.
    pub world_id: WorldId,
    /// Committed Moment to inspect, or the current World when absent.
    pub moment: Option<Moment>,
}

/// Request to add one Watchpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct AddWatchpointRequest {
    /// World receiving the Watchpoint.
    pub world_id: WorldId,
    /// Memory access selection.
    pub filter: MemoryFilter,
}

/// Request to update one Watchpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct UpdateWatchpointRequest {
    /// World containing the Watchpoint.
    pub world_id: WorldId,
    /// Complete replacement Watchpoint.
    pub watchpoint: Watchpoint,
}

/// Request to remove one Watchpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct RemoveWatchpointRequest {
    /// World containing the Watchpoint.
    pub world_id: WorldId,
    /// Watchpoint to remove.
    pub watchpoint_id: program::WatchpointId,
}

/// Request to list the Probes in one World.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ListProbesRequest {
    /// World to inspect.
    pub world_id: WorldId,
    /// Committed Moment to inspect, or the current World when absent.
    pub moment: Option<Moment>,
}

/// Request to add one Probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct AddProbeRequest {
    /// World receiving the Probe.
    pub world_id: WorldId,
    /// Runtime activity selection.
    pub filter: ProbeFilter,
    /// Action performed for each match.
    pub action: ProbeAction,
}

/// Request to update one Probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct UpdateProbeRequest {
    /// World containing the Probe.
    pub world_id: WorldId,
    /// Complete replacement Probe.
    pub probe: Probe,
}

/// Request to remove one Probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct RemoveProbeRequest {
    /// World containing the Probe.
    pub world_id: WorldId,
    /// Probe to remove.
    pub probe_id: ProbeId,
}
