#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::runtime::{
    AgentCreateOptionsVm, AgentDescriptorVm, AgentFilterVm, AgentHandle, AgentId,
    BranchDescriptorVm, BranchFilterVm, BranchId, CheckpointDescriptorVm, CheckpointFilterVm,
    CheckpointId, EngineDescriptorKind, EngineDescriptorVm, EventLoopDescriptorVm,
    HeapDescriptorVm, ImageDescriptorVm, ImageFilterVm, ImageId, ObservationEventKind,
    ObservationHandle, ObservationOptionsVm, ObservationRecordVm, ResourceDescriptorVm,
    ResourceFilterVm, RevisionDescriptorVm, RevisionFilterVm, RevisionId, RuntimeCreateOptionsVm,
    RuntimeDescriptorVm, RuntimeEngineKind, RuntimeExecutionMode, RuntimeFilterVm, RuntimeHandle,
    RuntimeId, RuntimeLabelSelectorVm, RuntimeLabelVm, RuntimeTickOutcome, RuntimeWorldKind,
    SnapshotDescriptorVm, SnapshotFormat, SnapshotId, TopologyEdgeFilterVm, TopologyEdgeIdVm,
    TopologyEdgeKindVm, TopologyEdgeVm, TopologyEntityFilterVm, TopologyEntityIdVm,
    TopologyEntityKindVm, TopologyEntityVm, TraceCursorHandle, TraceCursorOptionsVm,
    TraceDescriptorVm, TraceEventKind, TraceRecordVm, TraceSequence, WorldCreateOptionsVm,
    WorldDescriptorVm, WorldHandle, WorldResourceIdVm, WorldViewHandle, WorldViewOptionsVm,
};
use crate::platform::{PlatformError, VmArray, resource};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

/// Close one agent.
/// Remove one execution lane from its owning runtime and release its lane-local state.
/// Resource teardown follows runtime and resource policy.
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime agent teardown logic.
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
/// # Security
/// Requires `runtime.agent.control`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_agent_close(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    agent: AgentHandle,
) -> RuntimeResult<()> {
    let _ = agent;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.core.agentClose is not available in the VM yet",
    ))
    .boxed())
}

/// Spawn one agent in one runtime.
/// Create one execution lane inside one runtime with explicit metadata options.
/// The agent is attached to the runtime's current world and trace context.
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime agent construction logic.
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
/// # Security
/// Requires `runtime.agent.create`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_agent_create(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    runtimehandle: RuntimeHandle,
    options: Option<AgentCreateOptionsVm>,
) -> RuntimeResult<AgentHandle> {
    let _ = (runtimehandle, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.core.agentCreate is not available in the VM yet",
    ))
    .boxed())
}

/// Describe one agent.
/// Return one structured agent descriptor for one live agent handle.
/// Descriptor fields summarize ownership, labels, pending-work state, and resource counts.
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime agent state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.agent.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_agent_describe(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    agent: AgentHandle,
) -> RuntimeResult<AgentDescriptorVm> {
    let _ = agent;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.core.agentDescribe is not available in the VM yet",
    ))
    .boxed())
}

/// Close one runtime.
/// Remove one runtime container from its owning world and release its live execution state.
/// Agent teardown follows runtime shutdown policy.
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime teardown logic.
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
/// # Security
/// Requires `runtime.runtime.control`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_runtime_close(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    runtimehandle: RuntimeHandle,
) -> RuntimeResult<()> {
    let _ = runtimehandle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.core.runtimeClose is not available in the VM yet",
    ))
    .boxed())
}

/// Spawn one runtime in one world.
/// Create one process-like runtime container inside one world with explicit metadata options.
/// The new runtime is attached to the world's active branch and trace state.
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime world ownership and runtime construction logic.
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
/// # Security
/// Requires `runtime.runtime.create`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_runtime_create(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    options: Option<RuntimeCreateOptionsVm>,
) -> RuntimeResult<RuntimeHandle> {
    let _ = (argument_world, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.core.runtimeCreate is not available in the VM yet",
    ))
    .boxed())
}

/// Describe one runtime.
/// Return one structured runtime descriptor for one live runtime handle.
/// Descriptor fields summarize runtime ownership, primary agent, labels, and agent counts.
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.runtime.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_runtime_describe(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    runtimehandle: RuntimeHandle,
) -> RuntimeResult<RuntimeDescriptorVm> {
    let _ = runtimehandle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.core.runtimeDescribe is not available in the VM yet",
    ))
    .boxed())
}

/// Close one world.
/// Tear down one low-level world control object and release its owned runtime state.
/// Active runtimes, agents, and attached runtime resources are closed according to runtime shutdown policy.
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime world teardown logic.
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
/// # Security
/// Requires `runtime.world.control`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_world_close(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
) -> RuntimeResult<()> {
    let _ = argument_world;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.core.worldClose is not available in the VM yet",
    ))
    .boxed())
}

/// Create one world.
/// Allocate one new low-level world control object with explicit runtime options.
/// World creation initializes clocks, lineage, topology, simulation, trace, and other world-owned state.
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime world construction rather than host syscalls.
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
/// # Security
/// Requires `runtime.world.create`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_world_create(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    options: Option<WorldCreateOptionsVm>,
) -> RuntimeResult<WorldHandle> {
    let _ = options;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.core.worldCreate is not available in the VM yet",
    ))
    .boxed())
}

/// Describe one world.
/// Return one structured world descriptor for one live world handle.
/// Descriptor fields reflect the current active branch, revision, time state, and runtime counts.
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime world state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.world.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_world_describe(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
) -> RuntimeResult<WorldDescriptorVm> {
    let _ = argument_world;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.core.worldDescribe is not available in the VM yet",
    ))
    .boxed())
}

/// Advance one world by one scheduler step.
/// Execute one world-level deterministic advance step across runtimes, agents, clocks, and simulation.
/// The returned outcome distinguishes idle, progressed, and time-advance results.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world scheduler and simulation coordination.
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
/// # Security
/// Requires `runtime.world.control`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_world_tick(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
) -> RuntimeResult<RuntimeTickOutcome> {
    let _ = argument_world;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.core.worldTick is not available in the VM yet",
    ))
    .boxed())
}

/// List agents visible through one pinned world view.
/// Enumerate agent descriptors from the image backing one pinned world view.
/// When one runtime identifier is provided, only agents owned by that runtime are returned.
/// # Platform
/// Runtime-managed on all targets.
/// Uses pinned image and topology state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.inspect.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_agent_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    filter: Option<AgentFilterVm>,
    after: Option<AgentId>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<AgentDescriptorVm>> {
    let _ = (view, filter, after, limit);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.inspect.agentList is not available in the VM yet",
    ))
    .boxed())
}

/// Describe one agent visible through one pinned world view.
/// Return one agent descriptor from the image backing one pinned world view.
/// Descriptor values are stable for the lifetime of the view.
/// # Platform
/// Runtime-managed on all targets.
/// Uses pinned image and topology state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.inspect.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_agent_view(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    agentid: AgentId,
) -> RuntimeResult<AgentDescriptorVm> {
    let _ = (view, agentid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.inspect.agentView is not available in the VM yet",
    ))
    .boxed())
}

/// List topology edges visible through one pinned world view.
/// Enumerate topology edges from the image backing one pinned world view.
/// Descriptor values are stable for the lifetime of the view.
/// # Platform
/// Runtime-managed on all targets.
/// Uses pinned topology state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.inspect.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_edge_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    filter: Option<TopologyEdgeFilterVm>,
    after: Option<TopologyEdgeIdVm>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<TopologyEdgeVm>> {
    let _ = (view, filter, after, limit);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.inspect.edgeList is not available in the VM yet",
    ))
    .boxed())
}

/// Describe one topology edge visible through one pinned world view.
/// Return one topology edge from the image backing one pinned world view.
/// Descriptor values are stable for the lifetime of the view.
/// # Platform
/// Runtime-managed on all targets.
/// Uses pinned topology state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.inspect.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_edge_view(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    edgeid: TopologyEdgeIdVm,
) -> RuntimeResult<TopologyEdgeVm> {
    let _ = (view, edgeid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.inspect.edgeView is not available in the VM yet",
    ))
    .boxed())
}

/// Describe the execution engine for one agent visible through one pinned world view.
/// Return one engine summary from the image backing one pinned world view.
/// Descriptor values are stable for the lifetime of the view.
/// # Platform
/// Runtime-managed on all targets.
/// Uses pinned image state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.inspect.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_engine_view(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    agentid: AgentId,
) -> RuntimeResult<EngineDescriptorVm> {
    let _ = (view, agentid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.inspect.engineView is not available in the VM yet",
    ))
    .boxed())
}

/// List topology entities visible through one pinned world view.
/// Enumerate topology entities from the image backing one pinned world view.
/// Descriptor values are stable for the lifetime of the view.
/// # Platform
/// Runtime-managed on all targets.
/// Uses pinned topology state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.inspect.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_entity_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    filter: Option<TopologyEntityFilterVm>,
    after: Option<TopologyEntityIdVm>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<TopologyEntityVm>> {
    let _ = (view, filter, after, limit);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.inspect.entityList is not available in the VM yet",
    ))
    .boxed())
}

/// Describe one topology entity visible through one pinned world view.
/// Return one topology entity from the image backing one pinned world view.
/// Descriptor values are stable for the lifetime of the view.
/// # Platform
/// Runtime-managed on all targets.
/// Uses pinned topology state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.inspect.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_entity_view(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    entityid: TopologyEntityIdVm,
) -> RuntimeResult<TopologyEntityVm> {
    let _ = (view, entityid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.inspect.entityView is not available in the VM yet",
    ))
    .boxed())
}

/// Describe the event loop for one agent visible through one pinned world view.
/// Return one event-loop summary from the image backing one pinned world view.
/// Descriptor values are stable for the lifetime of the view.
/// # Platform
/// Runtime-managed on all targets.
/// Uses pinned image state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.inspect.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_event_loop_view(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    agentid: AgentId,
) -> RuntimeResult<EventLoopDescriptorVm> {
    let _ = (view, agentid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.inspect.eventLoopView is not available in the VM yet",
    ))
    .boxed())
}

/// Describe the heap for one agent visible through one pinned world view.
/// Return one heap summary from the image backing one pinned world view.
/// Descriptor values are stable for the lifetime of the view.
/// # Platform
/// Runtime-managed on all targets.
/// Uses pinned image state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.inspect.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_heap_view(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    agentid: AgentId,
) -> RuntimeResult<HeapDescriptorVm> {
    let _ = (view, agentid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.inspect.heapView is not available in the VM yet",
    ))
    .boxed())
}

/// Describe the pinned image for one world view.
/// Return one image descriptor for the image backing the pinned revision in one world view.
/// Descriptor values are stable for the lifetime of the view.
/// # Platform
/// Runtime-managed on all targets.
/// Uses pinned lineage and image state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.inspect.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_image_view(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
) -> RuntimeResult<ImageDescriptorVm> {
    let _ = view;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.inspect.imageView is not available in the VM yet",
    ))
    .boxed())
}

/// List logical world resources visible through one pinned world view.
/// Enumerate logical resource descriptors from the image and topology backing one pinned world view.
/// Descriptor values are stable for the lifetime of the view.
/// # Platform
/// Runtime-managed on all targets.
/// Uses pinned image and topology state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.inspect.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_resource_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    filter: Option<ResourceFilterVm>,
    after: Option<WorldResourceIdVm>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<ResourceDescriptorVm>> {
    let _ = (view, filter, after, limit);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.inspect.resourceList is not available in the VM yet",
    ))
    .boxed())
}

/// Describe one logical world resource visible through one pinned world view.
/// Return one resource descriptor from the image and topology backing one pinned world view.
/// Descriptor values are stable for the lifetime of the view.
/// # Platform
/// Runtime-managed on all targets.
/// Uses pinned image and topology state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.inspect.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_resource_view(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    resourceid: WorldResourceIdVm,
) -> RuntimeResult<ResourceDescriptorVm> {
    let _ = (view, resourceid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.inspect.resourceView is not available in the VM yet",
    ))
    .boxed())
}

/// Describe the pinned revision for one world view.
/// Return one revision descriptor for the exact revision pinned by one world view.
/// Descriptor values are stable for the lifetime of the view.
/// # Platform
/// Runtime-managed on all targets.
/// Uses pinned lineage and image state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.inspect.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_revision_view(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
) -> RuntimeResult<RevisionDescriptorVm> {
    let _ = view;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.inspect.revisionView is not available in the VM yet",
    ))
    .boxed())
}

/// List runtimes visible through one pinned world view.
/// Enumerate runtime descriptors from the image backing one pinned world view.
/// Descriptor values are stable for the lifetime of the view.
/// # Platform
/// Runtime-managed on all targets.
/// Uses pinned image and topology state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.inspect.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_runtime_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    filter: Option<RuntimeFilterVm>,
    after: Option<RuntimeId>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<RuntimeDescriptorVm>> {
    let _ = (view, filter, after, limit);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.inspect.runtimeList is not available in the VM yet",
    ))
    .boxed())
}

/// Describe one runtime visible through one pinned world view.
/// Return one runtime descriptor from the image backing one pinned world view.
/// Descriptor values are stable for the lifetime of the view.
/// # Platform
/// Runtime-managed on all targets.
/// Uses pinned image and topology state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.inspect.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_runtime_view(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    runtimeid: RuntimeId,
) -> RuntimeResult<RuntimeDescriptorVm> {
    let _ = (view, runtimeid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.inspect.runtimeView is not available in the VM yet",
    ))
    .boxed())
}

/// Describe the pinned trace state for one world view.
/// Return one trace descriptor for the exact trace position captured by one world view.
/// Descriptor values are stable for the lifetime of the view.
/// # Platform
/// Runtime-managed on all targets.
/// Uses pinned lineage and trace state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.inspect.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_trace_view(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
) -> RuntimeResult<TraceDescriptorVm> {
    let _ = view;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.inspect.traceView is not available in the VM yet",
    ))
    .boxed())
}

/// Describe the world visible through one pinned world view.
/// Return one world descriptor as observed through one pinned world view.
/// Descriptor values are stable for the lifetime of the view.
/// # Platform
/// Runtime-managed on all targets.
/// Uses pinned inspection state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.inspect.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_world_view(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
) -> RuntimeResult<WorldDescriptorVm> {
    let _ = view;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.inspect.worldView is not available in the VM yet",
    ))
    .boxed())
}

/// Close one pinned world view.
/// Release one previously opened world view and its pinned state.
/// Closing one view does not affect the underlying world, trace, or lineage objects.
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime inspection teardown logic.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.inspect.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_world_view_close(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
) -> RuntimeResult<()> {
    let _ = view;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.inspect.worldViewClose is not available in the VM yet",
    ))
    .boxed())
}

/// Open one pinned world view.
/// Create one read-consistent world view over the current world head or one explicit pinned revision.
/// Views provide stable reads across lineage, state, and trace metadata without racing live mutation.
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime inspection and revision pinning logic.
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
/// # Security
/// Requires `runtime.inspect.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_world_view_open(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    options: Option<WorldViewOptionsVm>,
) -> RuntimeResult<WorldViewHandle> {
    let _ = (argument_world, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.inspect.worldViewOpen is not available in the VM yet",
    ))
    .boxed())
}

/// Describe one branch.
/// Return one branch descriptor for one branch identifier in one world lineage.
/// Descriptors include the current head revision and branch labels.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world lineage state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.lineage.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_branch_describe(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    branchid: BranchId,
) -> RuntimeResult<BranchDescriptorVm> {
    let _ = (argument_world, branchid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.lineage.branchDescribe is not available in the VM yet",
    ))
    .boxed())
}

/// List branches in one world lineage.
/// Enumerate branch descriptors in stable branch-id order starting after the optional cursor.
/// This is the low-level bulk enumeration surface for world branch heads.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world lineage state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.lineage.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_branch_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    filter: Option<BranchFilterVm>,
    after: Option<BranchId>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<BranchDescriptorVm>> {
    let _ = (argument_world, filter, after, limit);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.lineage.branchList is not available in the VM yet",
    ))
    .boxed())
}

/// Create one checkpoint on the active branch.
/// Materialize one durable checkpoint at the world's current active revision.
/// The checkpoint anchors one revision and may force image capture according to runtime policy.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world lineage and image capture logic.
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
/// # Security
/// Requires `runtime.lineage.control`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_checkpoint_create(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    name: Option<vm::StringHandle>,
    labels: Option<VmArray<RuntimeLabelVm>>,
) -> RuntimeResult<CheckpointId> {
    let _ = (argument_world, name, labels);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.lineage.checkpointCreate is not available in the VM yet",
    ))
    .boxed())
}

/// Describe one checkpoint.
/// Return one checkpoint descriptor for one checkpoint identifier in one world lineage.
/// Descriptors include the anchored revision and checkpoint labels.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world lineage state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.lineage.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_checkpoint_describe(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    checkpointid: CheckpointId,
) -> RuntimeResult<CheckpointDescriptorVm> {
    let _ = (argument_world, checkpointid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.lineage.checkpointDescribe is not available in the VM yet",
    ))
    .boxed())
}

/// List checkpoints in one world lineage.
/// Enumerate checkpoint descriptors in stable checkpoint-id order starting after the optional cursor.
/// This is the low-level bulk enumeration surface for checkpoint tooling and inspection.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world lineage state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.lineage.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_checkpoint_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    filter: Option<CheckpointFilterVm>,
    after: Option<CheckpointId>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<CheckpointDescriptorVm>> {
    let _ = (argument_world, filter, after, limit);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.lineage.checkpointList is not available in the VM yet",
    ))
    .boxed())
}

/// Capture one image at the active revision.
/// Materialize one immutable image for the world's current active revision.
/// Image capture may share lower-level heap and VM image backing with prior revisions.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world image capture logic.
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
/// # Security
/// Requires `runtime.snapshot.create`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_image_capture(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
) -> RuntimeResult<ImageId> {
    let _ = argument_world;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.lineage.imageCapture is not available in the VM yet",
    ))
    .boxed())
}

/// Describe one image.
/// Return one image descriptor for one materialized image in one world lineage.
/// Descriptors summarize the owning revision and optional shared backing size.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world lineage and image metadata only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.snapshot.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_image_describe(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    imageid: ImageId,
) -> RuntimeResult<ImageDescriptorVm> {
    let _ = (argument_world, imageid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.lineage.imageDescribe is not available in the VM yet",
    ))
    .boxed())
}

/// List images in one world lineage.
/// Enumerate image descriptors in stable image-id order starting after the optional cursor.
/// This is the low-level bulk enumeration surface for materialized world images.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world image metadata only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.snapshot.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_image_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    filter: Option<ImageFilterVm>,
    after: Option<ImageId>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<ImageDescriptorVm>> {
    let _ = (argument_world, filter, after, limit);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.lineage.imageList is not available in the VM yet",
    ))
    .boxed())
}

/// Describe one revision.
/// Return one revision descriptor for one revision identifier in one world lineage.
/// Descriptors include the captured trace sequence, clock instants, and optional image reference.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world lineage state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.lineage.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_revision_describe(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    revisionid: RevisionId,
) -> RuntimeResult<RevisionDescriptorVm> {
    let _ = (argument_world, revisionid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.lineage.revisionDescribe is not available in the VM yet",
    ))
    .boxed())
}

/// List revisions in one world lineage.
/// Enumerate revision descriptors in stable revision-id order starting after the optional cursor.
/// When one branch identifier is provided, only revisions from that branch are returned.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world lineage state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.lineage.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_revision_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    filter: Option<RevisionFilterVm>,
    after: Option<RevisionId>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<RevisionDescriptorVm>> {
    let _ = (argument_world, filter, after, limit);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.lineage.revisionList is not available in the VM yet",
    ))
    .boxed())
}

/// Return the active branch for one world.
/// Read the current active branch identifier for one live world.
/// Worlds always execute on exactly one active branch at a time.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world lineage state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.lineage.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_world_branch(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
) -> RuntimeResult<BranchId> {
    let _ = argument_world;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.lineage.worldBranch is not available in the VM yet",
    ))
    .boxed())
}

/// Fork one child world from one revision.
/// Create one new world handle from one selected revision in the source world lineage.
/// The forked world shares immutable lineage and image backing where possible and diverges only on later mutation.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world fork and lineage sharing logic.
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
/// # Security
/// Requires `runtime.world.create`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_world_fork(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    revisionid: RevisionId,
    name: Option<vm::StringHandle>,
    labels: Option<VmArray<RuntimeLabelVm>>,
) -> RuntimeResult<WorldHandle> {
    let _ = (argument_world, revisionid, name, labels);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.lineage.worldFork is not available in the VM yet",
    ))
    .boxed())
}

/// Return the active revision for one world.
/// Read the current active revision identifier for one live world.
/// The active revision reflects the current trace position and optional image materialization.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world lineage state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.lineage.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_world_revision(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
) -> RuntimeResult<RevisionId> {
    let _ = argument_world;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.lineage.worldRevision is not available in the VM yet",
    ))
    .boxed())
}

/// Rewind one world to one checkpoint.
/// Restore one live world to the revision anchored by the requested checkpoint.
/// Rewind keeps the same live world handle while replacing its active revision and image state.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world restore and lineage control logic.
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
/// # Security
/// Requires `runtime.lineage.control`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_world_rewind_checkpoint(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    checkpointid: CheckpointId,
) -> RuntimeResult<()> {
    let _ = (argument_world, checkpointid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.lineage.worldRewindCheckpoint is not available in the VM yet",
    ))
    .boxed())
}

/// Rewind one world to one revision.
/// Restore one live world to the requested revision identifier.
/// Rewind keeps the same live world handle while replacing its active revision and image state.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world restore and lineage control logic.
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
/// # Security
/// Requires `runtime.lineage.control`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_world_rewind_revision(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    revisionid: RevisionId,
) -> RuntimeResult<()> {
    let _ = (argument_world, revisionid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.lineage.worldRewindRevision is not available in the VM yet",
    ))
    .boxed())
}

/// Close one runtime observation subscription.
/// Release one previously opened observation subscription and its buffered event state.
/// Closing one observation feed does not affect the underlying world.
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime observation teardown logic.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.observation.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_observation_close(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: ObservationHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.observation.close is not available in the VM yet",
    ))
    .boxed())
}

/// Read the next batch of observation records.
/// Decode up to the requested limit of live observation records from one open subscription.
/// Observation records may include topology, resource, scheduler, diagnostic, and profile events.
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime observation feed state.
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
/// # Security
/// Requires `runtime.observation.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_observation_next(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: ObservationHandle,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<ObservationRecordVm>> {
    let _ = (handle, limit);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.observation.next is not available in the VM yet",
    ))
    .boxed())
}

/// Open one runtime observation subscription.
/// Create one low-level subscription for live runtime observation events from one world.
/// Observation feeds are distinct from the structured causal trace and may include higher-volume diagnostic streams.
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime observation and subscription state.
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
/// # Security
/// Requires `runtime.observation.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_observation_open(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    options: Option<ObservationOptionsVm>,
) -> RuntimeResult<ObservationHandle> {
    let _ = (argument_world, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.observation.open is not available in the VM yet",
    ))
    .boxed())
}

/// Export one snapshot from one image.
/// Serialize one materialized image into the requested snapshot format and return its snapshot identifier.
/// Fast snapshots prefer runtime-specific backing, while portable snapshots favor compatibility metadata and transportability.
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime snapshot export logic.
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
/// # Security
/// Requires `runtime.snapshot.create`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_snapshot_create(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    imageid: ImageId,
    format: SnapshotFormat,
) -> RuntimeResult<SnapshotId> {
    let _ = (argument_world, imageid, format);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.snapshot.create is not available in the VM yet",
    ))
    .boxed())
}

/// Describe one snapshot.
/// Return one snapshot descriptor for one stored snapshot identifier in one world lineage.
/// Descriptors summarize the backing image, format, and optional serialized size.
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime snapshot metadata only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.snapshot.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_snapshot_describe(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    snapshotid: SnapshotId,
) -> RuntimeResult<SnapshotDescriptorVm> {
    let _ = (argument_world, snapshotid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.snapshot.describe is not available in the VM yet",
    ))
    .boxed())
}

/// Import one serialized snapshot payload.
/// Decode one serialized snapshot payload and register it in the current world lineage.
/// Imported snapshots may later be described, read again, or restored.
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime snapshot import logic.
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
/// # Security
/// Requires `runtime.snapshot.create`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_snapshot_import(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    argument_payload: VmArray<u8>,
) -> RuntimeResult<SnapshotId> {
    let _ = (argument_world, argument_payload);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.snapshot.import is not available in the VM yet",
    ))
    .boxed())
}

/// List snapshots in one world lineage.
/// Enumerate snapshot descriptors in stable snapshot-id order starting after the optional cursor.
/// This is the low-level bulk enumeration surface for stored snapshot artifacts.
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime snapshot metadata only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.snapshot.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_snapshot_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    after: Option<SnapshotId>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<SnapshotDescriptorVm>> {
    let _ = (argument_world, after, limit);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.snapshot.list is not available in the VM yet",
    ))
    .boxed())
}

/// Read one serialized snapshot payload.
/// Return the encoded bytes for one stored snapshot identifier.
/// Encoded bytes may be used for transport, persistence, or offline analysis.
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime snapshot storage only.
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
/// # Security
/// Requires `runtime.snapshot.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_snapshot_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    snapshotid: SnapshotId,
) -> RuntimeResult<VmArray<u8>> {
    let _ = (argument_world, snapshotid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.snapshot.read is not available in the VM yet",
    ))
    .boxed())
}

/// Restore one world from one image.
/// Replace one live world's active state with the requested materialized image.
/// Image restore keeps the same world handle while switching its active revision and state payload.
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime image restore logic.
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
/// # Security
/// Requires `runtime.snapshot.restore`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_restore_image(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    imageid: ImageId,
) -> RuntimeResult<()> {
    let _ = (argument_world, imageid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.snapshot.restoreImage is not available in the VM yet",
    ))
    .boxed())
}

/// Restore one world from one snapshot.
/// Replace one live world's active state with the requested stored snapshot.
/// Snapshot restore may reconstruct one image before activating the restored revision.
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime snapshot restore logic.
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
/// # Security
/// Requires `runtime.snapshot.restore`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_restore_snapshot(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    snapshotid: SnapshotId,
) -> RuntimeResult<()> {
    let _ = (argument_world, snapshotid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.snapshot.restoreSnapshot is not available in the VM yet",
    ))
    .boxed())
}

/// Close one causal trace cursor.
/// Release one previously opened trace cursor and its pinned reader state.
/// Closing one cursor does not affect the underlying world trace.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world trace reader teardown logic.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.trace.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_trace_close(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    cursor: TraceCursorHandle,
) -> RuntimeResult<()> {
    let _ = cursor;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.trace.close is not available in the VM yet",
    ))
    .boxed())
}

/// Describe one world's causal trace.
/// Return one summary of the active branch and current sequence boundary for one world trace.
/// This is the low-level causal trace metadata surface rather than one debug sink interface.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world trace state only.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.trace.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_trace_describe(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
) -> RuntimeResult<TraceDescriptorVm> {
    let _ = argument_world;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.trace.describe is not available in the VM yet",
    ))
    .boxed())
}

/// Append one explicit trace marker.
/// Record one explicit causal marker in the active world trace and return its sequence number.
/// Markers are part of the structured causal trace rather than one debug sink side channel.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world trace append logic.
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
/// # Security
/// Requires `runtime.trace.control`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_trace_mark(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    label: vm::StringHandle,
) -> RuntimeResult<TraceSequence> {
    let _ = (argument_world, label);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.trace.mark is not available in the VM yet",
    ))
    .boxed())
}

/// Read the next batch of causal trace records.
/// Decode up to the requested limit of causal trace records from one open cursor.
/// Record payloads are returned as structured metadata plus optional encoded payload bytes.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world trace reader state.
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
/// # Security
/// Requires `runtime.trace.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_trace_next(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    cursor: TraceCursorHandle,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<TraceRecordVm>> {
    let _ = (cursor, limit);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.trace.next is not available in the VM yet",
    ))
    .boxed())
}

/// Open one causal trace cursor.
/// Create one cursor for reading structured causal trace records from one world.
/// Cursors start at one optional sequence boundary and advance independently from the live trace head.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world trace reader state.
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
/// # Security
/// Requires `runtime.trace.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_trace_open(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    options: Option<TraceCursorOptionsVm>,
) -> RuntimeResult<TraceCursorHandle> {
    let _ = (argument_world, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.trace.open is not available in the VM yet",
    ))
    .boxed())
}

/// Seek one causal trace cursor to one checkpoint boundary.
/// Reposition one open trace cursor so the next read starts at the trace sequence anchored by the requested checkpoint.
/// This is the low-level checkpoint-to-trace bridge for debugging and replay tooling.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world trace and lineage metadata.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.trace.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_trace_seek_checkpoint(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    cursor: TraceCursorHandle,
    checkpointid: CheckpointId,
) -> RuntimeResult<()> {
    let _ = (cursor, checkpointid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.trace.seekCheckpoint is not available in the VM yet",
    ))
    .boxed())
}

/// Seek one causal trace cursor to one revision boundary.
/// Reposition one open trace cursor so the next read starts at the trace sequence captured by the requested revision.
/// This is the low-level revision-to-trace bridge for debugging and replay tooling.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world trace and lineage metadata.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.trace.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_trace_seek_revision(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    cursor: TraceCursorHandle,
    revisionid: RevisionId,
) -> RuntimeResult<()> {
    let _ = (cursor, revisionid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.trace.seekRevision is not available in the VM yet",
    ))
    .boxed())
}

/// Seek one causal trace cursor to one sequence.
/// Reposition one open trace cursor so the next read starts at the requested sequence number.
/// Seeking does not mutate the underlying world trace.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world trace reader state.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.trace.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_trace_seek_sequence(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    cursor: TraceCursorHandle,
    sequence: TraceSequence,
) -> RuntimeResult<()> {
    let _ = (cursor, sequence);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.trace.seekSequence is not available in the VM yet",
    ))
    .boxed())
}

/// Return the current sequence position of one causal trace cursor.
/// Read the next sequence number that would be returned by one open trace cursor.
/// This is the low-level cursor-position primitive for trace tooling and resumption.
/// # Platform
/// Runtime-managed on all targets.
/// Uses world trace reader state.
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
/// # Security
/// Requires `runtime.trace.read`.
/// # Replay
/// Deterministic.
pub(crate) fn destack_runtime_trace_tell(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    cursor: TraceCursorHandle,
) -> RuntimeResult<TraceSequence> {
    let _ = cursor;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.runtime.trace.tell is not available in the VM yet",
    ))
    .boxed())
}
