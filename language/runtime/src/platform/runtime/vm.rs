use std::collections::BTreeMap;
use std::sync::Arc;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::{VmDecodeCodec, io_not_found};
use crate::platform::runtime::{
    AgentCreateOptionsVm, AgentDescriptorVm, AgentFilterVm, AgentHandle, AgentId,
    BranchDescriptorVm, BranchFilterVm, BranchId, CheckpointDescriptorVm, CheckpointFilterVm,
    CheckpointId, EngineDescriptorVm, EventLoopDescriptorVm, HeapDescriptorVm, ImageDescriptorVm,
    ImageFilterVm, ImageId, ObservationHandle, ObservationOptionsVm, ObservationRecordVm,
    ResourceDescriptorVm, ResourceFilterVm, RevisionDescriptorVm, RevisionFilterVm, RevisionId,
    RuntimeCreateOptionsVm, RuntimeDescriptorVm, RuntimeFilterVm, RuntimeHandle, RuntimeId,
    RuntimeLabelVm, RuntimeTickOutcome, SnapshotDescriptorVm, SnapshotFormat, SnapshotId,
    TopologyEdgeFilterVm, TopologyEdgeIdVm, TopologyEdgeVm, TopologyEntityFilterVm,
    TopologyEntityIdVm, TopologyEntityVm, TraceCursorHandle, TraceCursorOptionsVm,
    TraceDescriptorVm, TraceRecordVm, TraceSequence, WorldCreateOptionsVm, WorldDescriptorVm,
    WorldHandle, WorldResourceIdVm, WorldViewHandle, WorldViewOptionsVm,
};
use crate::platform::{PlatformError, VmArray};
use crate::runtime;
use crate::runtime::control::inspect::labels_match_selectors;
use crate::runtime::control::{control_table, empty_vm_engine};
use crate::runtime::{BindingCallContext, TickOutcome};
use destack_vm;

use super::{
    PinnedWorldView, RuntimeDescriptorCodec, RuntimeHandleCodec, RuntimeRequestCodec,
    VmRuntimeBinding, VmRuntimeDecode,
};

/// Close one agent.
pub(crate) fn destack_runtime_agent_close(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    argument_agent: AgentHandle,
) -> RuntimeResult<()> {
    // remove the external handle first
    let mut table = control_table().write();
    let entry = table.close_agent(RuntimeHandleCodec::decode_agent_handle(argument_agent))?;
    let world = table.world(entry.world_handle_id)?;

    world.remove_agent(entry.agent_id)
}

/// Spawn one agent in one runtime.
pub(crate) fn destack_runtime_agent_create(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    runtimehandle: RuntimeHandle,
    options: Option<AgentCreateOptionsVm>,
) -> RuntimeResult<AgentHandle> {
    // request decode
    let runtime_decode = VmRuntimeDecode::new(context);

    // decode create options
    let options = options.unwrap_or(AgentCreateOptionsVm {
        name: None,
        labels: None,
    });
    let name = runtime_decode.decode_optional(options.name)?;
    let labels = runtime_decode.decode_optional(options.labels)?;
    let runtime_options = RuntimeRequestCodec::agent_create_options(name, labels);

    // spawn one agent in the live runtime
    let mut table = control_table().write();
    let entry = table.runtime_entry(RuntimeHandleCodec::decode_runtime_handle(runtimehandle))?;
    let world = table.world(entry.world_handle_id)?;
    let agent_id =
        world.spawn_agent_with_options(entry.runtime_id, &runtime_options, empty_vm_engine()?)?;

    Ok(RuntimeHandleCodec::encode_agent_handle(
        table.register_agent(entry.world_handle_id, world, entry.runtime_id, agent_id),
    ))
}

/// Describe one agent.
pub(crate) fn destack_runtime_agent_describe(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    argument_agent: AgentHandle,
) -> RuntimeResult<AgentDescriptorVm> {
    // resolve one live agent descriptor
    let table = control_table().read();
    let (world, runtime_id, agent_id) =
        table.agent(RuntimeHandleCodec::decode_agent_handle(argument_agent))?;
    let mut runtime_binding = VmRuntimeBinding::new(context);

    world.with_runtime(runtime_id, |runtime| {
        let agent = runtime.agent(agent_id).ok_or_else(|| {
            RuntimeError::AgentNotFound {
                agent_id: agent_id.0,
            }
            .boxed()
        })?;

        let descriptor = RuntimeDescriptorCodec::agent_descriptor_for_live(&world, agent)?;

        runtime_binding.encode(descriptor)
    })
}

/// Close one runtime.
pub(crate) fn destack_runtime_runtime_close(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    argument_runtime: RuntimeHandle,
) -> RuntimeResult<()> {
    // remove the external handle first
    let mut table = control_table().write();
    let entry = table.close_runtime(RuntimeHandleCodec::decode_runtime_handle(argument_runtime))?;
    let world = table.world(entry.world_handle_id)?;
    let runtime = world.remove_runtime(entry.runtime_id)?;
    let agent_ids = runtime.agent_ids();

    table.close_agent_handles(entry.world_handle_id, &agent_ids)?;

    Ok(())
}

/// Spawn one runtime in one world.
pub(crate) fn destack_runtime_runtime_create(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    options: Option<RuntimeCreateOptionsVm>,
) -> RuntimeResult<RuntimeHandle> {
    // request decode
    let runtime_decode = VmRuntimeDecode::new(context);

    // decode create options
    let options = options.unwrap_or(RuntimeCreateOptionsVm {
        name: None,
        labels: None,
    });
    let name = runtime_decode.decode_optional(options.name)?;
    let labels = runtime_decode.decode_optional(options.labels)?;
    let runtime_options = RuntimeRequestCodec::runtime_create_options(name, labels);

    // spawn one runtime in the live world
    let mut table = control_table().write();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let runtime_id =
        world.spawn_runtime(Vec::<String>::new(), &runtime_options, empty_vm_engine()?)?;

    Ok(RuntimeHandleCodec::encode_runtime_handle(
        table.register_runtime(
            RuntimeHandleCodec::decode_world_handle(argument_world),
            world,
            runtime_id,
        ),
    ))
}

/// Describe one runtime.
pub(crate) fn destack_runtime_runtime_describe(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    argument_runtime: RuntimeHandle,
) -> RuntimeResult<RuntimeDescriptorVm> {
    // resolve one live runtime descriptor
    let table = control_table().read();
    let (world, runtime_id) =
        table.runtime(RuntimeHandleCodec::decode_runtime_handle(argument_runtime))?;
    let mut runtime_binding = VmRuntimeBinding::new(context);

    world.with_runtime(runtime_id, |runtime| {
        let descriptor = RuntimeDescriptorCodec::runtime_descriptor_for_live(&world, runtime)?;

        runtime_binding.encode(descriptor)
    })
}

/// Close one world.
pub(crate) fn destack_runtime_world_close(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
) -> RuntimeResult<()> {
    let mut table = control_table().write();
    table.close_world(RuntimeHandleCodec::decode_world_handle(argument_world))
}

/// Create one world.
pub(crate) fn destack_runtime_world_create(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    options: Option<WorldCreateOptionsVm>,
) -> RuntimeResult<WorldHandle> {
    // request decode
    let runtime_decode = VmRuntimeDecode::new(context);

    let options = options.unwrap_or(WorldCreateOptionsVm {
        engine: None,
        execution: None,
        world: None,
        labels: None,
    });
    let labels = runtime_decode.decode_optional(options.labels)?;
    let labels = RuntimeRequestCodec::labels_from_value(labels);
    let runtime_options =
        RuntimeRequestCodec::runtime_options_from_create(options.execution, options.world);
    let world = runtime::world::World::from_options(&runtime_options)?;

    let mut table = control_table().write();
    Ok(RuntimeHandleCodec::encode_world_handle(
        table.register_world(world, labels),
    ))
}

/// Describe one world.
pub(crate) fn destack_runtime_world_describe(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
) -> RuntimeResult<WorldDescriptorVm> {
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let labels = table.world_labels(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let mut runtime_binding = VmRuntimeBinding::new(context);

    runtime_binding.encode(RuntimeDescriptorCodec::world_descriptor(
        argument_world,
        &world,
        labels,
    )?)
}

/// Advance one world by one scheduler step.
pub(crate) fn destack_runtime_world_tick(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
) -> RuntimeResult<RuntimeTickOutcome> {
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let outcome = world.tick()?;

    Ok(match outcome {
        TickOutcome::Idle => RuntimeTickOutcome::Idle,
        TickOutcome::Progressed => RuntimeTickOutcome::Progressed,
        TickOutcome::AdvancedTime => RuntimeTickOutcome::AdvancedTime,
    })
}

/// Close one pinned runtime view.
pub(crate) fn destack_runtime_world_view_close(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
) -> RuntimeResult<()> {
    // drop the external pinned view handle
    let mut table = control_table().write();
    table.close_world_view(RuntimeHandleCodec::decode_world_view_handle(view))?;

    Ok(())
}

/// Open one pinned runtime view.
pub(crate) fn destack_runtime_world_view_open(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    options: Option<WorldViewOptionsVm>,
) -> RuntimeResult<WorldViewHandle> {
    // pin one explicit or current revision
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let options = options.unwrap_or(WorldViewOptionsVm { revision_id: None });
    let revision_id = options
        .revision_id
        .map(RuntimeHandleCodec::decode_revision_id)
        .unwrap_or_else(|| world.revision_id());
    world.revision_info(revision_id)?;

    drop(table);

    // register the pinned view
    let mut table = control_table().write();
    Ok(RuntimeHandleCodec::encode_world_view_handle(
        table.open_world_view(
            RuntimeHandleCodec::decode_world_handle(argument_world),
            revision_id,
        )?,
    ))
}

/// Describe the world visible through one pinned runtime view.
pub(crate) fn destack_runtime_world_view(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
) -> RuntimeResult<WorldDescriptorVm> {
    // resolve the pinned revision backing for this view
    let table = control_table().read();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let mut runtime_binding = VmRuntimeBinding::new(context);

    runtime_binding.encode(world_view.world_descriptor()?)
}

/// Describe the pinned revision for one world view.
pub(crate) fn destack_runtime_revision_view(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
) -> RuntimeResult<RevisionDescriptorVm> {
    // resolve one pinned revision
    let table = control_table().read();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let mut runtime_binding = VmRuntimeBinding::new(context);

    runtime_binding.encode(world_view.revision_descriptor()?)
}

/// Describe the pinned image for one world view.
pub(crate) fn destack_runtime_image_view(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
) -> RuntimeResult<ImageDescriptorVm> {
    // resolve one pinned image
    let table = control_table().read();
    let world_view = PinnedWorldView::from_handle(&table, view)?;

    let mut runtime_binding = VmRuntimeBinding::new(context);

    runtime_binding.encode(world_view.image_descriptor()?)
}

/// Describe the pinned trace state for one world view.
pub(crate) fn destack_runtime_trace_view(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
) -> RuntimeResult<TraceDescriptorVm> {
    // resolve one pinned trace position
    let table = control_table().read();
    let world_view = PinnedWorldView::from_handle(&table, view)?;

    world_view.trace_descriptor()
}

/// List runtimes visible through one pinned world view.
pub(crate) fn destack_runtime_runtime_list(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    filter: Option<RuntimeFilterVm>,
    after: Option<RuntimeId>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<RuntimeDescriptorVm>> {
    // request decode
    let runtime_decode = VmRuntimeDecode::new(context);

    // enumerate pinned runtimes in stable order
    let table = control_table().read();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let filter = runtime_decode.decode_optional(filter)?;
    let filter = RuntimeRequestCodec::runtime_filter_from_value(filter);
    let after = after.map(RuntimeHandleCodec::decode_runtime_id);
    let limit = RuntimeRequestCodec::list_limit(limit);
    let descriptors = world_view.runtime_descriptors(&filter, after, limit)?;
    let mut runtime_binding = VmRuntimeBinding::new(context);

    runtime_binding.encode(descriptors)
}

/// Describe one runtime visible through one pinned world view.
pub(crate) fn destack_runtime_runtime_view(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    runtime_id: RuntimeId,
) -> RuntimeResult<RuntimeDescriptorVm> {
    // resolve one pinned runtime
    let table = control_table().read();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let mut runtime_binding = VmRuntimeBinding::new(context);
    let descriptor =
        world_view.runtime_descriptor(RuntimeHandleCodec::decode_runtime_id(runtime_id))?;

    runtime_binding.encode(descriptor)
}

/// List agents visible through one pinned world view.
pub(crate) fn destack_runtime_agent_list(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    filter: Option<AgentFilterVm>,
    after: Option<AgentId>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<AgentDescriptorVm>> {
    // request decode
    let runtime_decode = VmRuntimeDecode::new(context);

    // enumerate pinned agents in stable order
    let table = control_table().read();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let filter = runtime_decode.decode_optional(filter)?;
    let filter = RuntimeRequestCodec::agent_filter_from_value(filter);
    let after = after.map(RuntimeHandleCodec::decode_agent_id);
    let limit = RuntimeRequestCodec::list_limit(limit);
    let descriptors = world_view.agent_descriptors(&filter, after, limit)?;
    let mut runtime_binding = VmRuntimeBinding::new(context);

    runtime_binding.encode(descriptors)
}

/// Describe one agent visible through one pinned world view.
pub(crate) fn destack_runtime_agent_view(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    agent_id: AgentId,
) -> RuntimeResult<AgentDescriptorVm> {
    // resolve one pinned agent
    let table = control_table().read();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let mut runtime_binding = VmRuntimeBinding::new(context);
    let descriptor = world_view.agent_descriptor(RuntimeHandleCodec::decode_agent_id(agent_id))?;

    runtime_binding.encode(descriptor)
}

/// List logical world resources visible through one pinned world view.
pub(crate) fn destack_runtime_resource_list(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    filter: Option<ResourceFilterVm>,
    after: Option<WorldResourceIdVm>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<ResourceDescriptorVm>> {
    // request decode
    let runtime_decode = VmRuntimeDecode::new(context);

    // enumerate pinned logical resources in stable order
    let table = control_table().read();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let filter = runtime_decode.decode_optional(filter)?;
    let filter = RuntimeRequestCodec::resource_filter_from_value(filter);
    let after = after
        .map(RuntimeHandleCodec::decode_world_resource_id)
        .transpose()?;
    let limit = RuntimeRequestCodec::list_limit(limit);
    let descriptors = world_view.resource_descriptors(&filter, after, limit)?;
    let mut runtime_binding = VmRuntimeBinding::new(context);

    runtime_binding.encode(descriptors)
}

/// Describe one logical world resource visible through one pinned world view.
pub(crate) fn destack_runtime_resource_view(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    resource_id: WorldResourceIdVm,
) -> RuntimeResult<ResourceDescriptorVm> {
    // resolve one pinned logical resource
    let resource_id = RuntimeHandleCodec::decode_world_resource_id_vm(resource_id)?;
    let table = control_table().read();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let mut runtime_binding = VmRuntimeBinding::new(context);
    let descriptor = world_view.resource_descriptor(resource_id)?;

    runtime_binding.encode(descriptor)
}

/// List topology entities visible through one pinned world view.
pub(crate) fn destack_runtime_entity_list(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    filter: Option<TopologyEntityFilterVm>,
    after: Option<TopologyEntityIdVm>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<TopologyEntityVm>> {
    // request decode
    let runtime_decode = VmRuntimeDecode::new(context);

    // enumerate pinned topology entities in stable order
    let table = control_table().read();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let filter = runtime_decode.decode_optional(filter)?;
    let filter = RuntimeRequestCodec::entity_filter_from_value(filter);
    let after = runtime_decode.decode_optional(after)?;
    let after = RuntimeRequestCodec::topology_entity_id_from_value(after);
    let limit = RuntimeRequestCodec::list_limit(limit);
    let descriptors = world_view.entity_descriptors(&filter, after.as_deref(), limit)?;
    let mut runtime_binding = VmRuntimeBinding::new(context);

    runtime_binding.encode(descriptors)
}

/// Describe one topology entity visible through one pinned world view.
pub(crate) fn destack_runtime_entity_view(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    entity_id: TopologyEntityIdVm,
) -> RuntimeResult<TopologyEntityVm> {
    // request decode
    let runtime_decode = VmRuntimeDecode::new(context);

    // resolve one pinned topology entity
    let entity_id = runtime_decode.decode_optional(Some(entity_id))?;
    let entity_id =
        RuntimeRequestCodec::topology_entity_id_from_value(entity_id).ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "entityId",
                "topology entity id is required",
            ))
            .boxed()
        })?;
    let table = control_table().read();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let mut runtime_binding = VmRuntimeBinding::new(context);
    let descriptor = world_view.entity_descriptor(entity_id.as_str())?;

    runtime_binding.encode(descriptor)
}

/// List topology edges visible through one pinned world view.
pub(crate) fn destack_runtime_edge_list(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    filter: Option<TopologyEdgeFilterVm>,
    after: Option<TopologyEdgeIdVm>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<TopologyEdgeVm>> {
    // request decode
    let runtime_decode = VmRuntimeDecode::new(context);

    // enumerate pinned topology edges in stable order
    let table = control_table().read();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let filter = runtime_decode.decode_optional(filter)?;
    let filter = RuntimeRequestCodec::edge_filter_from_value(filter);
    let after = runtime_decode.decode_optional(after)?;
    let after = RuntimeRequestCodec::topology_edge_id_from_value(after);
    let limit = RuntimeRequestCodec::list_limit(limit);
    let descriptors = world_view.edge_descriptors(&filter, after.as_deref(), limit)?;
    let mut runtime_binding = VmRuntimeBinding::new(context);

    runtime_binding.encode(descriptors)
}

/// Describe one topology edge visible through one pinned world view.
pub(crate) fn destack_runtime_edge_view(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    edge_id: TopologyEdgeIdVm,
) -> RuntimeResult<TopologyEdgeVm> {
    // request decode
    let runtime_decode = VmRuntimeDecode::new(context);

    // resolve one pinned topology edge
    let edge_id = runtime_decode.decode_optional(Some(edge_id))?;
    let edge_id = RuntimeRequestCodec::topology_edge_id_from_value(edge_id).ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "edgeId",
            "topology edge id is required",
        ))
        .boxed()
    })?;
    let table = control_table().read();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let mut runtime_binding = VmRuntimeBinding::new(context);
    let descriptor = world_view.edge_descriptor(edge_id.as_str())?;

    runtime_binding.encode(descriptor)
}

/// Describe the event loop for one agent visible through one pinned world view.
pub(crate) fn destack_runtime_event_loop_view(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    agent_id: AgentId,
) -> RuntimeResult<EventLoopDescriptorVm> {
    // resolve one pinned agent event loop
    let table = control_table().read();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let mut runtime_binding = VmRuntimeBinding::new(context);

    runtime_binding
        .encode(world_view.event_loop_descriptor(RuntimeHandleCodec::decode_agent_id(agent_id))?)
}

/// Describe the heap for one agent visible through one pinned world view.
pub(crate) fn destack_runtime_heap_view(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    agent_id: AgentId,
) -> RuntimeResult<HeapDescriptorVm> {
    // resolve one pinned agent heap
    let table = control_table().read();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let mut runtime_binding = VmRuntimeBinding::new(context);

    runtime_binding
        .encode(world_view.heap_descriptor(RuntimeHandleCodec::decode_agent_id(agent_id))?)
}

/// Describe the execution engine for one agent visible through one pinned world view.
pub(crate) fn destack_runtime_engine_view(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    agent_id: AgentId,
) -> RuntimeResult<EngineDescriptorVm> {
    // resolve one pinned agent engine
    let table = control_table().read();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let mut runtime_binding = VmRuntimeBinding::new(context);

    runtime_binding
        .encode(world_view.engine_descriptor(RuntimeHandleCodec::decode_agent_id(agent_id))?)
}

/// Describe one branch.
pub(crate) fn destack_runtime_branch_describe(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    branchid: BranchId,
) -> RuntimeResult<BranchDescriptorVm> {
    // load one live world and branch record
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let branch = world.branch_info(RuntimeHandleCodec::decode_branch_id(branchid))?;
    let mut runtime_binding = VmRuntimeBinding::new(context);

    runtime_binding.encode(RuntimeDescriptorCodec::branch_descriptor(branch)?)
}

/// List branches in one world lineage.
pub(crate) fn destack_runtime_branch_list(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    filter: Option<BranchFilterVm>,
    after: Option<BranchId>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<BranchDescriptorVm>> {
    // request decode
    let runtime_decode = VmRuntimeDecode::new(context);

    // enumerate branch heads in stable order
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let filter = runtime_decode.decode_optional(filter)?;
    let filter = RuntimeRequestCodec::branch_filter_from_value(filter);
    let limit = RuntimeRequestCodec::list_limit_or_max(limit);
    let after = after.map(|value| value.0).unwrap_or(0);
    let mut runtime_binding = VmRuntimeBinding::new(context);
    let descriptors = world
        .branch_ids()
        .into_iter()
        .filter(|branch_id| after == 0 || branch_id.get() > u128::from(after))
        .filter(|branch_id| {
            let Ok(branch) = world.branch_info(*branch_id) else {
                return false;
            };

            if let Some(name) = filter.name.as_deref()
                && branch.name != name
            {
                return false;
            }

            labels_match_selectors(&branch.labels, &filter.labels)
        })
        .take(limit)
        .map(|branch_id| world.branch_info(branch_id))
        .map(|branch| {
            branch.and_then(|branch| {
                runtime_binding.encode(RuntimeDescriptorCodec::branch_descriptor(branch)?)
            })
        })
        .collect::<RuntimeResult<Vec<_>>>()?;

    VmArray::from_values(context, &descriptors)
}

/// Create one checkpoint on the active branch.
pub(crate) fn destack_runtime_checkpoint_create(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    name: Option<destack_vm::StringHandle>,
    labels: Option<VmArray<RuntimeLabelVm>>,
) -> RuntimeResult<CheckpointId> {
    // request decode
    let runtime_decode = VmRuntimeDecode::new(context);

    // decode one checkpoint request
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let name = runtime_decode.decode_optional(name)?;
    let labels = runtime_decode.decode_optional(labels)?;
    let labels = RuntimeRequestCodec::labels_from_value(labels);
    let checkpoint_name = name.as_deref().unwrap_or("checkpoint");

    // capture the checkpoint through world lineage
    let checkpoint_id = world.checkpoint(checkpoint_name)?;

    // carry through low-level labels until checkpoint metadata grows a direct API
    if !labels.is_empty() {
        world.set_checkpoint_labels(checkpoint_id, labels)?;
    }

    RuntimeHandleCodec::encode_checkpoint_id(checkpoint_id)
}

/// Describe one checkpoint.
pub(crate) fn destack_runtime_checkpoint_describe(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    checkpointid: CheckpointId,
) -> RuntimeResult<CheckpointDescriptorVm> {
    // load one live checkpoint record
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let checkpoint =
        world.checkpoint_info(RuntimeHandleCodec::decode_checkpoint_id(checkpointid))?;
    let mut runtime_binding = VmRuntimeBinding::new(context);

    runtime_binding.encode(RuntimeDescriptorCodec::checkpoint_descriptor(checkpoint)?)
}

/// List checkpoints in one world lineage.
pub(crate) fn destack_runtime_checkpoint_list(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    filter: Option<CheckpointFilterVm>,
    after: Option<CheckpointId>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<CheckpointDescriptorVm>> {
    // request decode
    let runtime_decode = VmRuntimeDecode::new(context);

    // enumerate checkpoints in stable order
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let filter = runtime_decode.decode_optional(filter)?;
    let filter = RuntimeRequestCodec::checkpoint_filter_from_value(filter);
    let limit = RuntimeRequestCodec::list_limit_or_max(limit);
    let after = after.map(|value| value.0).unwrap_or(0);
    let mut runtime_binding = VmRuntimeBinding::new(context);
    let descriptors = world
        .checkpoint_ids()
        .into_iter()
        .filter(|checkpoint_id| after == 0 || checkpoint_id.get() > u128::from(after))
        .filter(|checkpoint_id| {
            let Ok(checkpoint) = world.checkpoint_info(*checkpoint_id) else {
                return false;
            };

            if let Some(revision_id) = filter.revision_id
                && checkpoint.revision_id != revision_id
            {
                return false;
            }

            if let Some(name) = filter.name.as_deref()
                && checkpoint.name != name
            {
                return false;
            }

            labels_match_selectors(&checkpoint.labels, &filter.labels)
        })
        .take(limit)
        .map(|checkpoint_id| world.checkpoint_info(checkpoint_id))
        .map(|checkpoint| {
            checkpoint.and_then(|checkpoint| {
                runtime_binding.encode(RuntimeDescriptorCodec::checkpoint_descriptor(checkpoint)?)
            })
        })
        .collect::<RuntimeResult<Vec<_>>>()?;

    VmArray::from_values(context, &descriptors)
}

/// Capture one image at the active revision.
pub(crate) fn destack_runtime_image_capture(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
) -> RuntimeResult<ImageId> {
    // materialize one live image through suspend capture
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let revision_id = world.suspend()?;
    let revision = world.revision_info(revision_id)?;

    RuntimeHandleCodec::encode_image_id(revision.image_id)
}

/// Describe one image.
pub(crate) fn destack_runtime_image_describe(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    imageid: ImageId,
) -> RuntimeResult<ImageDescriptorVm> {
    // load one stored image descriptor
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let image = world.image_info(RuntimeHandleCodec::decode_image_id(imageid))?;

    let mut runtime_binding = VmRuntimeBinding::new(context);

    runtime_binding.encode(RuntimeDescriptorCodec::image_descriptor(&world, &image)?)
}

/// List images in one world lineage.
pub(crate) fn destack_runtime_image_list(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    filter: Option<ImageFilterVm>,
    after: Option<ImageId>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<ImageDescriptorVm>> {
    // enumerate materialized images in stable order
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let filter = RuntimeRequestCodec::image_filter_from_value(filter);
    let limit = RuntimeRequestCodec::list_limit_or_max(limit);
    let after = after.map(|value| value.0).unwrap_or(0);
    let descriptors = world
        .image_ids()
        .into_iter()
        .filter(|image_id| after == 0 || image_id.get() > u128::from(after))
        .filter(|image_id| {
            let Some(revision_id) = filter.revision_id else {
                return true;
            };

            world.revision_ids().into_iter().any(|candidate| {
                world
                    .revision_info(candidate)
                    .map(|revision| revision.id == revision_id && revision.image_id == *image_id)
                    .unwrap_or(false)
            })
        })
        .take(limit)
        .map(|image_id| world.image_info(image_id))
        .map(|image| {
            image.and_then(|image| {
                let mut runtime_binding = VmRuntimeBinding::new(context);

                runtime_binding.encode(RuntimeDescriptorCodec::image_descriptor(&world, &image)?)
            })
        })
        .collect::<RuntimeResult<Vec<_>>>()?;

    VmArray::from_values(context, &descriptors)
}

/// Describe one revision.
pub(crate) fn destack_runtime_revision_describe(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    revisionid: RevisionId,
) -> RuntimeResult<RevisionDescriptorVm> {
    // load one stored revision descriptor
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let revision = world.revision_info(RuntimeHandleCodec::decode_revision_id(revisionid))?;
    let image = world.image_info(revision.image_id)?;
    let mut runtime_binding = VmRuntimeBinding::new(context);

    runtime_binding.encode(RuntimeDescriptorCodec::revision_descriptor(
        revision, &image,
    )?)
}

/// List revisions in one world lineage.
pub(crate) fn destack_runtime_revision_list(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    filter: Option<RevisionFilterVm>,
    after: Option<RevisionId>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<RevisionDescriptorVm>> {
    // enumerate revisions in stable order
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let filter = RuntimeRequestCodec::revision_filter_from_value(filter);
    let limit = RuntimeRequestCodec::list_limit_or_max(limit);
    let after = after.map(|value| value.0).unwrap_or(0);
    let mut runtime_binding = VmRuntimeBinding::new(context);
    let descriptors = world
        .revision_ids()
        .into_iter()
        .filter(|revision_id| after == 0 || revision_id.get() > u128::from(after))
        .filter(|revision_id| {
            let Some(branch_id) = filter.branch_id else {
                return true;
            };

            world
                .revision_info(*revision_id)
                .map(|revision| revision.branch_id == branch_id)
                .unwrap_or(false)
        })
        .take(limit)
        .map(|revision_id| {
            let revision = world.revision_info(revision_id)?;
            let image = world.image_info(revision.image_id)?;

            runtime_binding.encode(RuntimeDescriptorCodec::revision_descriptor(
                revision, &image,
            )?)
        })
        .collect::<RuntimeResult<Vec<_>>>()?;

    VmArray::from_values(context, &descriptors)
}

/// Return the active branch for one world.
pub(crate) fn destack_runtime_world_branch(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
) -> RuntimeResult<BranchId> {
    // read the active branch directly from the live world
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;

    RuntimeHandleCodec::encode_branch_id(world.branch_id())
}

/// Fork one child world from one revision.
pub(crate) fn destack_runtime_world_fork(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    revisionid: RevisionId,
    name: Option<destack_vm::StringHandle>,
    labels: Option<VmArray<RuntimeLabelVm>>,
) -> RuntimeResult<WorldHandle> {
    // request decode
    let runtime_decode = VmRuntimeDecode::new(context);

    // decode one fork request
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let revision_id = RuntimeHandleCodec::decode_revision_id(revisionid);
    let name = runtime_decode.decode_optional(name)?;
    let labels = runtime_decode.decode_optional(labels)?;
    let labels = RuntimeRequestCodec::labels_from_value(labels);
    let branch_name = name.as_deref().unwrap_or("fork");

    // fork one child world from one explicit revision
    let child = world.fork_revision(revision_id, branch_name)?;

    // carry branch labels through the child lineage until branch create grows them directly
    if !labels.is_empty() {
        child.set_branch_labels(child.branch_id(), labels)?;
    }

    drop(table);

    // register the child world
    let mut table = control_table().write();
    Ok(RuntimeHandleCodec::encode_world_handle(
        table.register_world(child, BTreeMap::new()),
    ))
}

/// Return the active revision for one world.
pub(crate) fn destack_runtime_world_revision(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
) -> RuntimeResult<RevisionId> {
    // read the active revision directly from the live world
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;

    RuntimeHandleCodec::encode_revision_id(world.revision_id())
}

/// Rewind one world to one checkpoint.
pub(crate) fn destack_runtime_world_rewind_checkpoint(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    checkpointid: CheckpointId,
) -> RuntimeResult<()> {
    // restore one live world to one checkpointed revision
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    world.rewind(RuntimeHandleCodec::decode_checkpoint_id(checkpointid))
}

/// Rewind one world to one revision.
pub(crate) fn destack_runtime_world_rewind_revision(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    revisionid: RevisionId,
) -> RuntimeResult<()> {
    // restore one live world to one explicit revision
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    world.rewind_revision(RuntimeHandleCodec::decode_revision_id(revisionid))
}

/// Close one runtime observation subscription.
pub(crate) fn destack_runtime_observation_close(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    handle: ObservationHandle,
) -> RuntimeResult<()> {
    let mut table = control_table().write();
    let entry = RuntimeDescriptorCodec::observation_entry(
        table.close_observation(RuntimeHandleCodec::decode_observation_handle(handle))?,
    );
    let world = table.world(RuntimeHandleCodec::decode_world_handle(entry.world))?;

    world.observations().close(entry.subscription_id)
}

/// Read the next batch of observation records.
pub(crate) fn destack_runtime_observation_next(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    handle: ObservationHandle,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<ObservationRecordVm>> {
    let table = control_table().read();
    let (world, subscription_id) =
        table.observation(RuntimeHandleCodec::decode_observation_handle(handle))?;
    let limit = RuntimeRequestCodec::list_limit_or_max(limit);
    let records = world.observations().next(subscription_id, limit)?;
    let mut runtime_binding = VmRuntimeBinding::new(context);

    runtime_binding.encode(RuntimeDescriptorCodec::observation_records(records)?)
}

/// Open one runtime observation subscription.
pub(crate) fn destack_runtime_observation_open(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    options: Option<ObservationOptionsVm>,
) -> RuntimeResult<ObservationHandle> {
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let options = options.unwrap_or(ObservationOptionsVm {
        runtime: None,
        topology: None,
        resource: None,
        scheduler: None,
        diagnostic: None,
        profile: None,
        domain: None,
    });
    let subscription_id =
        world
            .observations()
            .open(RuntimeRequestCodec::observation_options_from_flags(
                options.runtime,
                options.topology,
                options.resource,
                options.scheduler,
                options.diagnostic,
                options.profile,
                options.domain,
            ));

    drop(table);

    // register the observation handle
    let mut table = control_table().write();
    Ok(RuntimeHandleCodec::encode_observation_handle(
        table.open_observation_handle(
            RuntimeHandleCodec::decode_world_handle(argument_world),
            subscription_id,
        )?,
    ))
}

/// Export one snapshot from one image.
pub(crate) fn destack_runtime_snapshot_create(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    imageid: ImageId,
    format: SnapshotFormat,
) -> RuntimeResult<SnapshotId> {
    // resolve the live world and export one snapshot
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let snapshot = world.snapshot(RuntimeHandleCodec::decode_image_id(imageid))?;
    let bytes = Arc::<[u8]>::from(snapshot.encode()?);

    drop(table);

    // store the exported snapshot
    let mut table = control_table().write();

    Ok(RuntimeHandleCodec::encode_snapshot_id(
        table.store_snapshot_handle(
            RuntimeHandleCodec::decode_world_handle(argument_world),
            RuntimeHandleCodec::decode_snapshot_format(format),
            snapshot,
            bytes,
        )?,
    ))
}

/// Describe one snapshot.
pub(crate) fn destack_runtime_snapshot_describe(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    snapshotid: SnapshotId,
) -> RuntimeResult<SnapshotDescriptorVm> {
    // resolve the stored snapshot for this world
    let table = control_table().read();
    let entry = RuntimeDescriptorCodec::snapshot_entry(
        table.snapshot(RuntimeHandleCodec::decode_snapshot_id(snapshotid))?,
    );
    if entry.world != argument_world {
        return Err(io_not_found(
            "destack.runtime.snapshot.describe",
            format!("unknown snapshot {}", snapshotid.0),
        ));
    }

    let mut runtime_binding = VmRuntimeBinding::new(context);

    runtime_binding.encode(RuntimeDescriptorCodec::snapshot_descriptor(
        snapshotid, &entry,
    )?)
}

/// Import one serialized snapshot payload.
pub(crate) fn destack_runtime_snapshot_import(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    argument_payload: VmArray<u8>,
) -> RuntimeResult<SnapshotId> {
    // resolve the live world before storing the imported snapshot
    let table = control_table().read();
    let _world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;

    // decode one stored snapshot payload
    let payload = argument_payload.read_bytes(context)?;
    let snapshot = runtime::world::Snapshot::decode(&payload)?;
    let bytes = Arc::<[u8]>::from(payload);

    // release the read lock before storing the imported snapshot
    drop(table);

    Ok(RuntimeHandleCodec::encode_snapshot_id(
        control_table().write().store_snapshot_handle(
            RuntimeHandleCodec::decode_world_handle(argument_world),
            RuntimeHandleCodec::decode_snapshot_format(SnapshotFormat::Portable),
            snapshot,
            bytes,
        )?,
    ))
}

/// List snapshots in one world lineage.
pub(crate) fn destack_runtime_snapshot_list(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    after: Option<SnapshotId>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<SnapshotDescriptorVm>> {
    // enumerate stored snapshots for this world
    let limit = RuntimeRequestCodec::list_limit(limit);
    let table = control_table().read();
    let descriptors = table
        .snapshots_for_world(
            RuntimeHandleCodec::decode_world_handle(argument_world),
            after.map(RuntimeHandleCodec::decode_snapshot_id),
            limit,
        )
        .into_iter()
        .map(|(snapshot_id, entry)| {
            (
                RuntimeHandleCodec::encode_snapshot_id(snapshot_id),
                RuntimeDescriptorCodec::snapshot_entry(entry),
            )
        })
        .map(|(snapshot_id, entry)| {
            let mut runtime_binding = VmRuntimeBinding::new(context);

            runtime_binding.encode(RuntimeDescriptorCodec::snapshot_descriptor(
                snapshot_id,
                &entry,
            )?)
        })
        .collect::<RuntimeResult<Vec<_>>>()?;

    VmArray::from_values(context, &descriptors)
}

/// Read one serialized snapshot payload.
pub(crate) fn destack_runtime_snapshot_read(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    snapshotid: SnapshotId,
) -> RuntimeResult<VmArray<u8>> {
    // resolve the stored snapshot payload for this world
    let table = control_table().read();
    let entry = RuntimeDescriptorCodec::snapshot_entry(
        table.snapshot(RuntimeHandleCodec::decode_snapshot_id(snapshotid))?,
    );
    if entry.world != argument_world {
        return Err(io_not_found(
            "destack.runtime.snapshot.read",
            format!("unknown snapshot {}", snapshotid.0),
        ));
    }

    VmArray::from_bytes(context, entry.bytes.as_ref())
}

/// Restore one world from one image.
pub(crate) fn destack_runtime_restore_image(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    imageid: ImageId,
) -> RuntimeResult<()> {
    // restore the requested image into the live world
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;

    // release the read lock before mutating the world
    drop(table);

    world.restore_image_id(RuntimeHandleCodec::decode_image_id(imageid), None)
}

/// Restore one world from one snapshot.
pub(crate) fn destack_runtime_restore_snapshot(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    snapshotid: SnapshotId,
) -> RuntimeResult<()> {
    // resolve the snapshot and restore it into the live world
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let entry = RuntimeDescriptorCodec::snapshot_entry(
        table.snapshot(RuntimeHandleCodec::decode_snapshot_id(snapshotid))?,
    );
    if entry.world != argument_world {
        return Err(io_not_found(
            "destack.runtime.snapshot.restoreSnapshot",
            format!("unknown snapshot {}", snapshotid.0),
        ));
    }

    // release the read lock before mutating the world
    drop(table);

    world.restore_snapshot(&entry.snapshot, None)
}

/// Close one causal trace cursor.
pub(crate) fn destack_runtime_trace_close(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    cursor: TraceCursorHandle,
) -> RuntimeResult<()> {
    // drop the external cursor handle
    let mut table = control_table().write();
    table.close_trace_cursor(RuntimeHandleCodec::decode_trace_cursor_handle(cursor))?;

    Ok(())
}

/// Describe one world's causal trace.
pub(crate) fn destack_runtime_trace_describe(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
) -> RuntimeResult<TraceDescriptorVm> {
    // summarize the active world trace
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;

    Ok(TraceDescriptorVm {
        branch_id: RuntimeHandleCodec::encode_branch_id(world.branch_id())?,
        sequence: TraceSequence(world.trace().log().next_sequence().get()),
    })
}

/// Append one explicit trace marker.
pub(crate) fn destack_runtime_trace_mark(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    label: destack_vm::StringHandle,
) -> RuntimeResult<TraceSequence> {
    // decode the marker label from the vm call context
    let codec = VmDecodeCodec::new(context);
    let label = codec.decode(label)?;

    // resolve the live world and record the marker
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    drop(table);
    let sequence = world.label(label)?;

    Ok(TraceSequence(sequence.get()))
}

/// Read the next batch of causal trace records.
pub(crate) fn destack_runtime_trace_next(
    _binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
    cursor: TraceCursorHandle,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<TraceRecordVm>> {
    // read the next batch from one live cursor
    let table = control_table().read();
    let (_world, cursor) =
        table.trace_cursor(RuntimeHandleCodec::decode_trace_cursor_handle(cursor))?;
    let limit = RuntimeRequestCodec::list_limit_or_max(limit);
    let mut records = Vec::new();

    for _ in 0..limit {
        let sequence = cursor.sequence();
        let Some(event) = cursor.next_event()? else {
            break;
        };

        records.push((sequence, event));
    }
    let mut runtime_binding = VmRuntimeBinding::new(context);

    runtime_binding.encode(RuntimeDescriptorCodec::trace_records(records)?)
}

/// Open one causal trace cursor.
pub(crate) fn destack_runtime_trace_open(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    options: Option<TraceCursorOptionsVm>,
) -> RuntimeResult<TraceCursorHandle> {
    // open one live cursor at the requested sequence
    let table = control_table().read();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let cursor = Arc::new(world.trace().log().reader());
    let options = options.unwrap_or(TraceCursorOptionsVm {
        start_sequence: None,
    });
    if let Some(start_sequence) = options.start_sequence {
        cursor.seek_sequence(runtime::trace::TraceSequence::new(start_sequence.0))?;
    }

    drop(table);

    // register the trace cursor
    let mut table = control_table().write();
    Ok(RuntimeHandleCodec::encode_trace_cursor_handle(
        table.open_trace_cursor_handle(
            RuntimeHandleCodec::decode_world_handle(argument_world),
            cursor,
        )?,
    ))
}

/// Seek one causal trace cursor to one checkpoint boundary.
pub(crate) fn destack_runtime_trace_seek_checkpoint(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    cursor: TraceCursorHandle,
    checkpointid: CheckpointId,
) -> RuntimeResult<()> {
    // seek to the revision sequence anchored by one checkpoint
    let table = control_table().read();
    let (world, cursor) =
        table.trace_cursor(RuntimeHandleCodec::decode_trace_cursor_handle(cursor))?;
    let checkpoint =
        world.checkpoint_info(RuntimeHandleCodec::decode_checkpoint_id(checkpointid))?;
    let revision = world.revision_info(checkpoint.revision_id)?;

    cursor.seek_sequence(revision.sequence)
}

/// Seek one causal trace cursor to one revision boundary.
pub(crate) fn destack_runtime_trace_seek_revision(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    cursor: TraceCursorHandle,
    revisionid: RevisionId,
) -> RuntimeResult<()> {
    // seek to the sequence captured by one revision
    let table = control_table().read();
    let (world, cursor) =
        table.trace_cursor(RuntimeHandleCodec::decode_trace_cursor_handle(cursor))?;
    let revision = world.revision_info(RuntimeHandleCodec::decode_revision_id(revisionid))?;

    cursor.seek_sequence(revision.sequence)
}

/// Seek one causal trace cursor to one sequence.
pub(crate) fn destack_runtime_trace_seek_sequence(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    cursor: TraceCursorHandle,
    sequence: TraceSequence,
) -> RuntimeResult<()> {
    // seek one live cursor directly to one sequence boundary
    let table = control_table().read();
    let (_world, cursor) =
        table.trace_cursor(RuntimeHandleCodec::decode_trace_cursor_handle(cursor))?;
    cursor.seek_sequence(runtime::trace::TraceSequence::new(sequence.0))
}

/// Return the current sequence position of one causal trace cursor.
pub(crate) fn destack_runtime_trace_tell(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::ExternalCallContext<'_>,
    cursor: TraceCursorHandle,
) -> RuntimeResult<TraceSequence> {
    // expose the next visible sequence for one live cursor
    let table = control_table().read();
    let (_world, cursor) =
        table.trace_cursor(RuntimeHandleCodec::decode_trace_cursor_handle(cursor))?;
    let sequence = cursor.sequence();

    Ok(TraceSequence(sequence.get()))
}
