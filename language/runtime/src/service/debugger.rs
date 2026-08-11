use destack_memory::MemoryRange;
use destack_program as program;
use destack_rpc::service;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::worker::WorkerId;
use crate::world::{
    Breakpoint, Frame, MemoryFilter, Moment, PointFilter, Probe, ProbeAction, ProbeFilter, ProbeId,
    Watchpoint,
};

use super::WorldId;

/// RPC operations over hosted World debuggers.
#[service(name = "destack.world.Debugger")]
pub trait DebuggerService {
    // =============================================================================
    // Frame
    // =============================================================================

    /// Read captured Frames from one World.
    #[rpc(name = "ReadFrames", idempotency = "no_side_effects")]
    fn read_frames(request: ReadFramesRequest) -> Vec<Frame>;

    // =============================================================================
    // Memory
    // =============================================================================

    /// Read one exact World memory range.
    #[rpc(name = "ReadMemory", idempotency = "no_side_effects")]
    fn read_memory(request: ReadMemoryRequest) -> MemoryChunk;

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

/// Request to read one exact World memory range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReadMemoryRequest {
    /// World to inspect.
    pub world_id: WorldId,
    /// Committed Moment to inspect, or the current World when absent.
    pub moment: Option<Moment>,
    /// Exact logical memory range to read.
    pub range: MemoryRange,
}

/// One exact World memory range and its bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct MemoryChunk {
    /// Exact logical memory range.
    pub range: MemoryRange,
    /// Bytes read from the range.
    pub bytes: Vec<u8>,
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
