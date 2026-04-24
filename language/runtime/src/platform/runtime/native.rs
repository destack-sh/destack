use std::collections::BTreeMap;
use std::sync::Arc;

use super::{
    NativeRuntimeBinding, PinnedWorldView, RuntimeDescriptorCodec, RuntimeHandleCodec,
    RuntimeRequestCodec,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeStringRef;
use crate::platform::core::{ensure_out, io_not_found};
use crate::platform::runtime::{
    BranchDescriptor, BranchFilter, BranchId, CheckpointDescriptor, CheckpointFilter, CheckpointId,
    EngineDescriptor, EventLoopDescriptor, HeapDescriptor, ImageDescriptor, ImageFilter, ImageId,
    ObservationHandle, ObservationOptions, ObservationRecord, ResourceDescriptor, ResourceFilter,
    RevisionDescriptor, RevisionFilter, RevisionId, RuntimeCreateOptions, RuntimeDescriptor,
    RuntimeFilter, RuntimeHandle, RuntimeId, RuntimeLabel, RuntimeTickOutcome, SnapshotDescriptor,
    SnapshotFormat, SnapshotId, TopologyEdge as TopologyEdgeDescriptor, TopologyEdgeFilter,
    TopologyEdgeId, TopologyEntity as TopologyEntityDescriptor, TopologyEntityFilter,
    TopologyEntityId, TraceCursorHandle, TraceCursorOptions, TraceDescriptor, TraceRecord,
    TraceSequence, WorkerCreateOptions, WorkerDescriptor, WorkerFilter, WorkerHandle, WorkerId,
    WorldCreateOptions, WorldDescriptor, WorldHandle, WorldResourceId, WorldViewHandle,
    WorldViewOptions,
};
use crate::platform::{NativeArray, PlatformError};
use crate::runtime;
use crate::runtime::control::inspect::labels_match_selectors;
use crate::runtime::control::{control, empty_vm_engine};
use crate::runtime::{BindingCallContext, TickOutcome};

/// Close one worker.
pub(crate) unsafe fn destack_runtime_worker_close(
    binding: &BindingCallContext,
    argument_worker: WorkerHandle,
) -> RuntimeResult<()> {
    binding.clear_values();

    // remove the external handle first
    let control = control();
    let mut table = control.lock();
    let entry = table.close_worker(RuntimeHandleCodec::decode_worker_handle(argument_worker))?;
    let world = table.world_mut(entry.world_handle_id)?;

    world.remove_worker(entry.worker_id)
}

/// Spawn one worker in one runtime.
pub(crate) unsafe fn destack_runtime_worker_create(
    binding: &BindingCallContext,
    out: *mut WorkerHandle,
    runtime_handle: RuntimeHandle,
    options: Option<WorkerCreateOptions>,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;

    let runtime_binding = NativeRuntimeBinding::new(binding);

    // decode create options
    let options = options.unwrap_or(WorkerCreateOptions {
        name: None,
        labels: None,
    });
    let name = unsafe { runtime_binding.decode_optional(options.name) }?;
    let labels = unsafe { runtime_binding.decode_optional(options.labels) }?;
    let runtime_options = RuntimeRequestCodec::worker_create_options(name, labels);

    // clear call-local output storage
    binding.clear_values();

    // spawn one worker in the live runtime
    let control = control();
    let mut table = control.lock();
    let entry = table.runtime_entry(RuntimeHandleCodec::decode_runtime_handle(runtime_handle))?;
    let runtime_id = entry.runtime_id;
    let world = table.world_mut(entry.world_handle_id)?;
    let worker_id =
        world.spawn_worker_with_options(runtime_id, &runtime_options, empty_vm_engine()?)?;
    let handle = RuntimeHandleCodec::encode_worker_handle(table.register_worker(
        entry.world_handle_id,
        runtime_id,
        worker_id,
    ));

    unsafe { out.write(handle) };

    Ok(())
}

/// Describe one worker.
pub(crate) unsafe fn destack_runtime_worker_describe(
    binding: &BindingCallContext,
    out: *mut WorkerDescriptor,
    argument_worker: WorkerHandle,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve one live worker descriptor
    let control = control();
    let table = control.lock();
    let entry = table.worker_entry(RuntimeHandleCodec::decode_worker_handle(argument_worker))?;
    let runtime_id = entry.runtime_id;
    let worker_id = entry.worker_id;
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let world = table.world(entry.world_handle_id)?;
    let runtime = world.runtime(runtime_id)?;
    let worker = runtime.worker(worker_id).ok_or_else(|| {
        RuntimeError::WorkerNotFound {
            worker_id: worker_id.0,
        }
        .boxed()
    })?;
    let descriptor: WorkerDescriptor = runtime_binding.encode(
        RuntimeDescriptorCodec::worker_descriptor_for_live(world, worker)?,
    );

    unsafe { out.write(descriptor) };

    Ok(())
}

/// Close one runtime.
pub(crate) unsafe fn destack_runtime_runtime_close(
    binding: &BindingCallContext,
    argument_runtime: RuntimeHandle,
) -> RuntimeResult<()> {
    binding.clear_values();

    // remove the external handle first
    let control = control();
    let mut table = control.lock();
    let entry = table.close_runtime(RuntimeHandleCodec::decode_runtime_handle(argument_runtime))?;
    let world = table.world_mut(entry.world_handle_id)?;
    let runtime = world.remove_runtime(entry.runtime_id)?;
    let worker_ids = runtime.worker_ids();

    table.close_worker_handles(entry.world_handle_id, &worker_ids)?;

    Ok(())
}

/// Spawn one runtime in one world.
pub(crate) unsafe fn destack_runtime_runtime_create(
    binding: &BindingCallContext,
    out: *mut RuntimeHandle,
    argument_world: WorldHandle,
    options: Option<RuntimeCreateOptions>,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;

    let runtime_binding = NativeRuntimeBinding::new(binding);

    // decode create options
    let options = options.unwrap_or(RuntimeCreateOptions {
        name: None,
        labels: None,
    });
    let name = unsafe { runtime_binding.decode_optional(options.name) }?;
    let labels = unsafe { runtime_binding.decode_optional(options.labels) }?;
    let runtime_options = RuntimeRequestCodec::runtime_create_options(name, labels);

    // clear call-local output storage
    binding.clear_values();

    // spawn one runtime in the live world
    let control = control();
    let mut table = control.lock();
    let world = table.world_mut(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let runtime_id =
        world.spawn_runtime(Vec::<String>::new(), &runtime_options, empty_vm_engine()?)?;
    let handle = RuntimeHandleCodec::encode_runtime_handle(table.register_runtime(
        RuntimeHandleCodec::decode_world_handle(argument_world),
        runtime_id,
    ));

    unsafe { out.write(handle) };

    Ok(())
}

/// Describe one runtime.
pub(crate) unsafe fn destack_runtime_runtime_describe(
    binding: &BindingCallContext,
    out: *mut RuntimeDescriptor,
    argument_runtime: RuntimeHandle,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve one live runtime descriptor
    let control = control();
    let table = control.lock();
    let entry = table.runtime_entry(RuntimeHandleCodec::decode_runtime_handle(argument_runtime))?;
    let runtime_id = entry.runtime_id;
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let world = table.world(entry.world_handle_id)?;
    let runtime = world.runtime(runtime_id)?;
    let descriptor: RuntimeDescriptor = runtime_binding.encode(
        RuntimeDescriptorCodec::runtime_descriptor_for_live(world, &runtime)?,
    );

    unsafe { out.write(descriptor) };

    Ok(())
}

/// Close one world.
pub(crate) unsafe fn destack_runtime_world_close(
    binding: &BindingCallContext,
    argument_world: WorldHandle,
) -> RuntimeResult<()> {
    binding.clear_values();

    let control = control();
    let mut table = control.lock();
    table.close_world(RuntimeHandleCodec::decode_world_handle(argument_world))
}

/// Create one world.
pub(crate) unsafe fn destack_runtime_world_create(
    binding: &BindingCallContext,
    out: *mut WorldHandle,
    options: Option<WorldCreateOptions>,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;

    let runtime_binding = NativeRuntimeBinding::new(binding);

    // decode labels
    let options = options.unwrap_or(WorldCreateOptions {
        engine: None,
        execution: None,
        world: None,
        labels: None,
    });
    let labels = unsafe { runtime_binding.decode_optional(options.labels) }?;
    let labels = RuntimeRequestCodec::labels_from_value(labels);
    let runtime_options =
        RuntimeRequestCodec::runtime_options_from_create(options.execution, options.world);

    // clear call-local output storage
    binding.clear_values();

    let world = runtime::world::World::from_options(&runtime_options)?;
    let control = control();
    let mut table = control.lock();
    let handle = RuntimeHandleCodec::encode_world_handle(table.register_world(world, labels));

    unsafe {
        *out = handle;
    }

    Ok(())
}

/// Describe one world.
pub(crate) unsafe fn destack_runtime_world_describe(
    binding: &BindingCallContext,
    out: *mut WorldDescriptor,
    argument_world: WorldHandle,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve live world state
    let control = control();
    let table = control.lock();
    let labels = table.world_labels(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let descriptor: WorldDescriptor = runtime_binding.encode::<WorldDescriptor>(
        RuntimeDescriptorCodec::world_descriptor(argument_world, world, labels)?,
    );

    unsafe {
        *out = descriptor;
    }

    Ok(())
}

/// Advance one world by one scheduler step.
pub(crate) unsafe fn destack_runtime_world_tick(
    binding: &BindingCallContext,
    out: *mut RuntimeTickOutcome,
    argument_world: WorldHandle,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // tick one live world
    let control = control();
    let mut table = control.lock();
    let world = table.world_mut(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let outcome = match world.tick()? {
        TickOutcome::Idle => RuntimeTickOutcome::Idle,
        TickOutcome::Progressed => RuntimeTickOutcome::Progressed,
        TickOutcome::AdvancedTime => RuntimeTickOutcome::AdvancedTime,
        TickOutcome::Concurrent => RuntimeTickOutcome::Progressed,
    };

    unsafe {
        *out = outcome;
    }

    Ok(())
}

/// Close one pinned runtime view.
pub(crate) unsafe fn destack_runtime_world_view_close(
    binding: &BindingCallContext,
    view: WorldViewHandle,
) -> RuntimeResult<()> {
    binding.clear_values();

    // drop the external pinned view handle
    let control = control();
    let mut table = control.lock();
    table.close_world_view(RuntimeHandleCodec::decode_world_view_handle(view))?;

    Ok(())
}

/// Open one pinned world view.
pub(crate) unsafe fn destack_runtime_world_view_open(
    binding: &BindingCallContext,
    out: *mut WorldViewHandle,
    argument_world: WorldHandle,
    options: Option<WorldViewOptions>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    binding.clear_values();

    // pin one explicit or current revision
    let options = options.unwrap_or(WorldViewOptions { revision_id: None });
    let revision = {
        let control = control();
        let mut table = control.lock();
        let revision = match options.revision_id {
            Some(revision_id) => RuntimeHandleCodec::decode_revision_id(revision_id),
            None => table
                .world(RuntimeHandleCodec::decode_world_handle(argument_world))?
                .revision(),
        };
        let world = table.world_mut(RuntimeHandleCodec::decode_world_handle(argument_world))?;
        world.revision_state(revision).map(|_| ())?;

        revision
    };

    // register the pinned view
    let control = control();
    let mut table = control.lock();
    let handle = RuntimeHandleCodec::encode_world_view_handle(table.open_world_view(
        RuntimeHandleCodec::decode_world_handle(argument_world),
        revision,
    )?);
    unsafe { out.write(handle) };

    Ok(())
}

/// Describe the world visible through one pinned world view.
pub(crate) unsafe fn destack_runtime_world_view(
    binding: &BindingCallContext,
    out: *mut WorldDescriptor,
    view: WorldViewHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    binding.clear_values();

    // resolve the pinned revision backing for this view
    let control = control();
    let table = control.lock();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let descriptor = runtime_binding.encode(world_view.world_descriptor()?);

    unsafe { out.write(descriptor) };

    Ok(())
}

/// Describe the pinned revision for one world view.
pub(crate) unsafe fn destack_runtime_revision_view(
    binding: &BindingCallContext,
    out: *mut RevisionDescriptor,
    view: WorldViewHandle,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve one pinned revision
    let control = control();
    let table = control.lock();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let descriptor = runtime_binding.encode(world_view.revision_descriptor()?);

    unsafe { out.write(descriptor) };

    Ok(())
}

/// Describe the pinned image for one world view.
pub(crate) unsafe fn destack_runtime_image_view(
    binding: &BindingCallContext,
    out: *mut ImageDescriptor,
    view: WorldViewHandle,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve one pinned image
    let control = control();
    let table = control.lock();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let descriptor = runtime_binding.encode(world_view.image_descriptor()?);

    unsafe { out.write(descriptor) };

    Ok(())
}

/// Describe the pinned trace state for one world view.
pub(crate) unsafe fn destack_runtime_trace_view(
    binding: &BindingCallContext,
    out: *mut TraceDescriptor,
    view: WorldViewHandle,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve one pinned trace position
    let control = control();
    let table = control.lock();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let descriptor = world_view.trace_descriptor()?;

    unsafe { out.write(descriptor) };

    Ok(())
}

/// List runtimes visible through one pinned world view.
pub(crate) unsafe fn destack_runtime_runtime_list(
    binding: &BindingCallContext,
    out: *mut NativeArray<RuntimeDescriptor>,
    view: WorldViewHandle,
    filter: Option<RuntimeFilter>,
    after: Option<RuntimeId>,
    limit: Option<u32>,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;

    // enumerate pinned runtimes in stable order
    let control = control();
    let table = control.lock();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let filter = unsafe { runtime_binding.decode_optional(filter) }?;
    let filter = RuntimeRequestCodec::runtime_filter_from_value(filter);
    let after = after.map(RuntimeHandleCodec::decode_runtime_id);
    let limit = RuntimeRequestCodec::list_limit(limit);

    // clear call-local output storage
    binding.clear_values();

    let descriptors: NativeArray<RuntimeDescriptor> =
        runtime_binding.encode(world_view.runtime_descriptors(&filter, after, limit)?);

    unsafe { out.write(descriptors) };

    Ok(())
}

/// Describe one runtime visible through one pinned world view.
pub(crate) unsafe fn destack_runtime_runtime_view(
    binding: &BindingCallContext,
    out: *mut RuntimeDescriptor,
    view: WorldViewHandle,
    runtime_id: RuntimeId,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve one pinned runtime
    let control = control();
    let table = control.lock();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let descriptor: RuntimeDescriptor = runtime_binding
        .encode(world_view.runtime_descriptor(RuntimeHandleCodec::decode_runtime_id(runtime_id))?);

    unsafe { out.write(descriptor) };

    Ok(())
}

/// List workers visible through one pinned world view.
pub(crate) unsafe fn destack_runtime_worker_list(
    binding: &BindingCallContext,
    out: *mut NativeArray<WorkerDescriptor>,
    view: WorldViewHandle,
    filter: Option<WorkerFilter>,
    after: Option<WorkerId>,
    limit: Option<u32>,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;

    // enumerate pinned workers in stable order
    let control = control();
    let table = control.lock();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let filter = unsafe { runtime_binding.decode_optional(filter) }?;
    let filter = RuntimeRequestCodec::worker_filter_from_value(filter);
    let after = after.map(RuntimeHandleCodec::decode_worker_id);
    let limit = RuntimeRequestCodec::list_limit(limit);

    // clear call-local output storage
    binding.clear_values();

    let descriptors: NativeArray<WorkerDescriptor> =
        runtime_binding.encode(world_view.worker_descriptors(&filter, after, limit)?);

    unsafe { out.write(descriptors) };

    Ok(())
}

/// Describe one worker visible through one pinned world view.
pub(crate) unsafe fn destack_runtime_worker_view(
    binding: &BindingCallContext,
    out: *mut WorkerDescriptor,
    view: WorldViewHandle,
    worker_id: WorkerId,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve one pinned worker
    let control = control();
    let table = control.lock();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let descriptor: WorkerDescriptor = runtime_binding
        .encode(world_view.worker_descriptor(RuntimeHandleCodec::decode_worker_id(worker_id))?);

    unsafe { out.write(descriptor) };

    Ok(())
}

/// List logical world resources visible through one pinned world view.
pub(crate) unsafe fn destack_runtime_resource_list(
    binding: &BindingCallContext,
    out: *mut NativeArray<ResourceDescriptor>,
    view: WorldViewHandle,
    filter: Option<ResourceFilter>,
    after: Option<WorldResourceId>,
    limit: Option<u32>,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;

    // enumerate pinned logical resources in stable order
    let control = control();
    let table = control.lock();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let filter = unsafe { runtime_binding.decode_optional(filter) }?;
    let filter = RuntimeRequestCodec::resource_filter_from_value(filter);
    let after = after
        .map(RuntimeHandleCodec::decode_world_resource_id)
        .transpose()?;
    let limit = RuntimeRequestCodec::list_limit(limit);

    // clear call-local output storage
    binding.clear_values();

    let descriptors: NativeArray<ResourceDescriptor> =
        runtime_binding.encode(world_view.resource_descriptors(&filter, after, limit)?);

    unsafe { out.write(descriptors) };

    Ok(())
}

/// Describe one logical world resource visible through one pinned world view.
pub(crate) unsafe fn destack_runtime_resource_view(
    binding: &BindingCallContext,
    out: *mut ResourceDescriptor,
    view: WorldViewHandle,
    resource_id: WorldResourceId,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve one pinned logical resource
    let resource_id = RuntimeHandleCodec::decode_world_resource_id(resource_id)?;
    let control = control();
    let table = control.lock();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let descriptor: ResourceDescriptor =
        runtime_binding.encode(world_view.resource_descriptor(resource_id)?);

    unsafe { out.write(descriptor) };

    Ok(())
}

/// List topology entities visible through one pinned world view.
pub(crate) unsafe fn destack_runtime_entity_list(
    binding: &BindingCallContext,
    out: *mut NativeArray<TopologyEntityDescriptor>,
    view: WorldViewHandle,
    filter: Option<TopologyEntityFilter>,
    after: Option<TopologyEntityId>,
    limit: Option<u32>,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;

    // enumerate pinned topology entities in stable order
    let control = control();
    let table = control.lock();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let filter = unsafe { runtime_binding.decode_optional(filter) }?;
    let filter = RuntimeRequestCodec::entity_filter_from_value(filter);
    let after = unsafe { runtime_binding.decode_optional(after) }?;
    let after = RuntimeRequestCodec::topology_entity_id_from_value(after);
    let limit = RuntimeRequestCodec::list_limit(limit);

    // clear call-local output storage
    binding.clear_values();

    let descriptors: NativeArray<TopologyEntityDescriptor> =
        runtime_binding.encode(world_view.entity_descriptors(&filter, after.as_deref(), limit)?);

    unsafe { out.write(descriptors) };

    Ok(())
}

/// Describe one topology entity visible through one pinned world view.
pub(crate) unsafe fn destack_runtime_entity_view(
    binding: &BindingCallContext,
    out: *mut TopologyEntityDescriptor,
    view: WorldViewHandle,
    entity_id: TopologyEntityId,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;

    // resolve one pinned topology entity
    let entity_id = unsafe { entity_id.0.as_str()? };
    let control = control();
    let table = control.lock();
    let world_view = PinnedWorldView::from_handle(&table, view)?;

    // clear call-local output storage
    binding.clear_values();

    // encode one stable topology entity descriptor
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let descriptor: TopologyEntityDescriptor =
        runtime_binding.encode(world_view.entity_descriptor(entity_id)?);

    unsafe { out.write(descriptor) };

    Ok(())
}

/// List topology edges visible through one pinned world view.
pub(crate) unsafe fn destack_runtime_edge_list(
    binding: &BindingCallContext,
    out: *mut NativeArray<TopologyEdgeDescriptor>,
    view: WorldViewHandle,
    filter: Option<TopologyEdgeFilter>,
    after: Option<TopologyEdgeId>,
    limit: Option<u32>,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;

    // enumerate pinned topology edges in stable order
    let control = control();
    let table = control.lock();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let filter = unsafe { runtime_binding.decode_optional(filter) }?;
    let filter = RuntimeRequestCodec::edge_filter_from_value(filter);
    let after = unsafe { runtime_binding.decode_optional(after) }?;
    let after = RuntimeRequestCodec::topology_edge_id_from_value(after);
    let limit = RuntimeRequestCodec::list_limit(limit);

    // clear call-local output storage
    binding.clear_values();

    let descriptors: NativeArray<TopologyEdgeDescriptor> =
        runtime_binding.encode(world_view.edge_descriptors(&filter, after.as_deref(), limit)?);

    unsafe { out.write(descriptors) };

    Ok(())
}

/// Describe one topology edge visible through one pinned world view.
pub(crate) unsafe fn destack_runtime_edge_view(
    binding: &BindingCallContext,
    out: *mut TopologyEdgeDescriptor,
    view: WorldViewHandle,
    edge_id: TopologyEdgeId,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;

    // resolve one pinned topology edge
    let edge_id = unsafe { edge_id.0.as_str()? };
    let control = control();
    let table = control.lock();
    let world_view = PinnedWorldView::from_handle(&table, view)?;

    // clear call-local output storage
    binding.clear_values();

    // encode one stable topology edge descriptor
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let descriptor: TopologyEdgeDescriptor =
        runtime_binding.encode(world_view.edge_descriptor(edge_id)?);

    unsafe { out.write(descriptor) };

    Ok(())
}

/// Describe the event loop for one worker visible through one pinned world view.
pub(crate) unsafe fn destack_runtime_event_loop_view(
    binding: &BindingCallContext,
    out: *mut EventLoopDescriptor,
    view: WorldViewHandle,
    worker_id: WorkerId,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve one pinned worker event loop
    let control = control();
    let table = control.lock();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let descriptor = runtime_binding
        .encode(world_view.event_loop_descriptor(RuntimeHandleCodec::decode_worker_id(worker_id))?);

    unsafe { out.write(descriptor) };

    Ok(())
}

/// Describe the heap for one worker visible through one pinned world view.
pub(crate) unsafe fn destack_runtime_heap_view(
    binding: &BindingCallContext,
    out: *mut HeapDescriptor,
    view: WorldViewHandle,
    worker_id: WorkerId,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve one pinned worker heap
    let control = control();
    let table = control.lock();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let descriptor = runtime_binding
        .encode(world_view.heap_descriptor(RuntimeHandleCodec::decode_worker_id(worker_id))?);

    unsafe { out.write(descriptor) };

    Ok(())
}

/// Describe the execution engine for one worker visible through one pinned world view.
pub(crate) unsafe fn destack_runtime_engine_view(
    binding: &BindingCallContext,
    out: *mut EngineDescriptor,
    view: WorldViewHandle,
    worker_id: WorkerId,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve one pinned worker engine
    let control = control();
    let table = control.lock();
    let world_view = PinnedWorldView::from_handle(&table, view)?;
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let descriptor = runtime_binding
        .encode(world_view.engine_descriptor(RuntimeHandleCodec::decode_worker_id(worker_id))?);

    unsafe { out.write(descriptor) };

    Ok(())
}

/// Describe one branch.
pub(crate) unsafe fn destack_runtime_branch_describe(
    binding: &BindingCallContext,
    out: *mut BranchDescriptor,
    argument_world: WorldHandle,
    branchid: BranchId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    binding.clear_values();

    // load one live world and branch record
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let control = control();
    let table = control.lock();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let branch = world.branch_info(RuntimeHandleCodec::decode_branch_id(branchid))?;
    let descriptor: BranchDescriptor =
        runtime_binding.encode(RuntimeDescriptorCodec::branch_descriptor(branch)?);

    unsafe { out.write(descriptor) };

    Ok(())
}

/// List branches in one world lineage.
pub(crate) unsafe fn destack_runtime_branch_list(
    binding: &BindingCallContext,
    out: *mut NativeArray<BranchDescriptor>,
    argument_world: WorldHandle,
    filter: Option<BranchFilter>,
    after: Option<BranchId>,
    limit: Option<u32>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // enumerate branch heads in stable order
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let filter = unsafe { runtime_binding.decode_optional(filter) }?;
    let filter = RuntimeRequestCodec::branch_filter_from_value(filter);
    let limit = RuntimeRequestCodec::list_limit_or_max(limit);
    let after = after.map(|value| value.0).unwrap_or(0);

    // clear call-local output storage
    binding.clear_values();

    let control = control();
    let table = control.lock();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let descriptors = world
        .branch_ids()
        .into_iter()
        .filter(|branch_id: &runtime::world::BranchId| {
            after == 0 || branch_id.get() > u128::from(after)
        })
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
            let branch = branch?;
            let descriptor: BranchDescriptor =
                runtime_binding.encode(RuntimeDescriptorCodec::branch_descriptor(branch)?);

            Ok(descriptor)
        })
        .collect::<RuntimeResult<Vec<_>>>()?;

    unsafe { out.write(binding.store_array(descriptors)) };

    Ok(())
}

/// Create one checkpoint on the active branch.
pub(crate) unsafe fn destack_runtime_checkpoint_create(
    binding: &BindingCallContext,
    out: *mut CheckpointId,
    argument_world: WorldHandle,
    name: Option<NativeStringRef>,
    labels: Option<NativeArray<RuntimeLabel>>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode one checkpoint request
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let name = unsafe { runtime_binding.decode_optional(name) }?;
    let labels = unsafe { runtime_binding.decode_optional(labels) }?;
    let labels = RuntimeRequestCodec::labels_from_value(labels);
    let checkpoint_name = name.as_deref().unwrap_or("checkpoint");

    // clear call-local output storage
    binding.clear_values();

    // capture the checkpoint through world lineage
    let control = control();
    let mut table = control.lock();
    let world = table.world_mut(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let checkpoint_id = world.checkpoint(checkpoint_name)?;

    // carry through low-level labels until checkpoint metadata grows a direct API
    if !labels.is_empty() {
        world.set_checkpoint_labels(checkpoint_id, labels)?;
    }

    unsafe { out.write(RuntimeHandleCodec::encode_checkpoint_id(checkpoint_id)?) };

    Ok(())
}

/// Describe one checkpoint.
pub(crate) unsafe fn destack_runtime_checkpoint_describe(
    binding: &BindingCallContext,
    out: *mut CheckpointDescriptor,
    argument_world: WorldHandle,
    checkpointid: CheckpointId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    binding.clear_values();

    // load one live checkpoint record
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let control = control();
    let table = control.lock();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let checkpoint =
        world.checkpoint_info(RuntimeHandleCodec::decode_checkpoint_id(checkpointid))?;
    let descriptor: CheckpointDescriptor =
        runtime_binding.encode(RuntimeDescriptorCodec::checkpoint_descriptor(checkpoint)?);

    unsafe { out.write(descriptor) };

    Ok(())
}

/// List checkpoints in one world lineage.
pub(crate) unsafe fn destack_runtime_checkpoint_list(
    binding: &BindingCallContext,
    out: *mut NativeArray<CheckpointDescriptor>,
    argument_world: WorldHandle,
    filter: Option<CheckpointFilter>,
    after: Option<CheckpointId>,
    limit: Option<u32>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // enumerate checkpoints in stable order
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let filter = unsafe { runtime_binding.decode_optional(filter) }?;
    let filter = RuntimeRequestCodec::checkpoint_filter_from_value(filter);
    let limit = RuntimeRequestCodec::list_limit_or_max(limit);
    let after = after.map(|value| value.0).unwrap_or(0);

    // clear call-local output storage
    binding.clear_values();

    let control = control();
    let table = control.lock();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let descriptors = world
        .checkpoint_ids()
        .into_iter()
        .filter(|checkpoint_id: &runtime::world::CheckpointId| {
            after == 0 || checkpoint_id.get() > u128::from(after)
        })
        .filter(|checkpoint_id| {
            let Ok(checkpoint) = world.checkpoint_info(*checkpoint_id) else {
                return false;
            };

            if let Some(revision) = filter.revision
                && checkpoint.revision != revision
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
            let checkpoint = checkpoint?;
            let descriptor: CheckpointDescriptor =
                runtime_binding.encode(RuntimeDescriptorCodec::checkpoint_descriptor(checkpoint)?);

            Ok(descriptor)
        })
        .collect::<RuntimeResult<Vec<_>>>()?;

    unsafe { out.write(binding.store_array(descriptors)) };

    Ok(())
}

/// Capture one image at the active revision.
pub(crate) unsafe fn destack_runtime_image_capture(
    binding: &BindingCallContext,
    out: *mut ImageId,
    argument_world: WorldHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    binding.clear_values();

    // materialize one live image through suspend capture
    let control = control();
    let mut table = control.lock();
    let world = table.world_mut(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let revision = world.suspend()?;
    let revision = world.revision_state(revision)?;

    unsafe { out.write(RuntimeHandleCodec::encode_image_id(revision.image_id)?) };

    Ok(())
}

/// Describe one image.
pub(crate) unsafe fn destack_runtime_image_describe(
    binding: &BindingCallContext,
    out: *mut ImageDescriptor,
    argument_world: WorldHandle,
    imageid: ImageId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    binding.clear_values();

    // load one stored image descriptor
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let control = control();
    let table = control.lock();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let image_id = RuntimeHandleCodec::decode_image_id(imageid);
    let image = world.image_info(image_id)?;
    let revision = world.revision_for_image(image_id)?;
    let descriptor: ImageDescriptor = runtime_binding.encode(
        RuntimeDescriptorCodec::image_descriptor(revision, image_id, &image)?,
    );

    unsafe { out.write(descriptor) };

    Ok(())
}

/// List images in one world lineage.
pub(crate) unsafe fn destack_runtime_image_list(
    binding: &BindingCallContext,
    out: *mut NativeArray<ImageDescriptor>,
    argument_world: WorldHandle,
    filter: Option<ImageFilter>,
    after: Option<ImageId>,
    limit: Option<u32>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    binding.clear_values();

    // enumerate materialized images in stable order
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let filter = unsafe { runtime_binding.decode_optional(filter) }?;
    let filter = RuntimeRequestCodec::image_filter_from_value(filter);
    let limit = RuntimeRequestCodec::list_limit_or_max(limit);
    let after = after.map(|value| value.0).unwrap_or(0);
    let control = control();
    let table = control.lock();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let descriptors = world
        .image_ids()
        .into_iter()
        .filter(|image_id: &runtime::world::ImageId| {
            after == 0 || image_id.get() > u128::from(after)
        })
        .filter(|image_id| {
            let Some(revision) = filter.revision else {
                return true;
            };

            world.revision_for_image(*image_id).ok() == Some(revision)
        })
        .take(limit)
        .map(|image_id| {
            let image = world.image_info(image_id)?;
            let revision = world.revision_for_image(image_id)?;
            let descriptor: ImageDescriptor = runtime_binding.encode(
                RuntimeDescriptorCodec::image_descriptor(revision, image_id, &image)?,
            );

            Ok(descriptor)
        })
        .collect::<RuntimeResult<Vec<_>>>()?;

    unsafe { out.write(binding.store_array(descriptors)) };

    Ok(())
}

/// Describe one revision.
pub(crate) unsafe fn destack_runtime_revision_describe(
    binding: &BindingCallContext,
    out: *mut RevisionDescriptor,
    argument_world: WorldHandle,
    revisionid: RevisionId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    binding.clear_values();

    // load one stored revision descriptor
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let control = control();
    let table = control.lock();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let revision = world.revision_state(RuntimeHandleCodec::decode_revision_id(revisionid))?;
    let image = world.image_info(revision.image_id)?;
    let descriptor: RevisionDescriptor =
        runtime_binding.encode(RuntimeDescriptorCodec::revision_descriptor(
            RuntimeHandleCodec::decode_revision_id(revisionid),
            revision,
            &image,
        )?);

    unsafe { out.write(descriptor) };

    Ok(())
}

/// List revisions in one world lineage.
pub(crate) unsafe fn destack_runtime_revision_list(
    binding: &BindingCallContext,
    out: *mut NativeArray<RevisionDescriptor>,
    argument_world: WorldHandle,
    filter: Option<RevisionFilter>,
    after: Option<RevisionId>,
    limit: Option<u32>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    binding.clear_values();

    // enumerate revisions in stable order
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let filter = unsafe { runtime_binding.decode_optional(filter) }?;
    let filter = RuntimeRequestCodec::revision_filter_from_value(filter);
    let limit = RuntimeRequestCodec::list_limit_or_max(limit);
    let after = after.map(|value| value.0).unwrap_or(0);
    let control = control();
    let table = control.lock();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let descriptors = world
        .revisions()
        .into_iter()
        .filter(|revision: &runtime::world::Revision| {
            after == 0 || revision.get() > u128::from(after)
        })
        .filter(|revision| {
            let Some(branch_id) = filter.branch_id else {
                return true;
            };

            world
                .revision_state(*revision)
                .map(|revision| revision.branch_id == branch_id)
                .unwrap_or(false)
        })
        .take(limit)
        .map(|revision_id| {
            let revision = world.revision_state(revision_id)?;
            let image = world.image_info(revision.image_id)?;
            let descriptor: RevisionDescriptor = runtime_binding.encode(
                RuntimeDescriptorCodec::revision_descriptor(revision_id, revision, &image)?,
            );

            Ok(descriptor)
        })
        .collect::<RuntimeResult<Vec<_>>>()?;

    unsafe { out.write(binding.store_array(descriptors)) };

    Ok(())
}

/// Return the active branch for one world.
pub(crate) unsafe fn destack_runtime_world_branch(
    binding: &BindingCallContext,
    out: *mut BranchId,
    argument_world: WorldHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    binding.clear_values();

    // read the active branch directly from the live world
    let control = control();
    let table = control.lock();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let branch_id = RuntimeHandleCodec::encode_branch_id(world.branch_id())?;

    unsafe { out.write(branch_id) };

    Ok(())
}

/// Fork one child world from one revision.
pub(crate) unsafe fn destack_runtime_world_fork(
    binding: &BindingCallContext,
    out: *mut WorldHandle,
    argument_world: WorldHandle,
    revisionid: RevisionId,
    name: Option<NativeStringRef>,
    labels: Option<NativeArray<RuntimeLabel>>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode one fork request
    let revision = RuntimeHandleCodec::decode_revision_id(revisionid);
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let name = unsafe { runtime_binding.decode_optional(name) }?;
    let labels = unsafe { runtime_binding.decode_optional(labels) }?;
    let labels = RuntimeRequestCodec::labels_from_value(labels);
    let branch_name = name.as_deref().unwrap_or("fork");

    // clear call-local output storage
    binding.clear_values();

    // fork one child world from one explicit revision
    let child = {
        let control = control();
        let mut table = control.lock();
        let world = table.world_mut(RuntimeHandleCodec::decode_world_handle(argument_world))?;
        let child = world.fork_revision(revision, branch_name)?;

        // carry branch labels through the child lineage until branch create grows them directly
        if !labels.is_empty() {
            child.set_branch_labels(child.branch_id(), labels)?;
        }

        child
    };

    // register the child world
    let control = control();
    let mut table = control.lock();
    let child_handle =
        RuntimeHandleCodec::encode_world_handle(table.register_world(child, BTreeMap::new()));
    unsafe { out.write(child_handle) };

    Ok(())
}

/// Return the active revision for one world.
pub(crate) unsafe fn destack_runtime_world_revision(
    binding: &BindingCallContext,
    out: *mut RevisionId,
    argument_world: WorldHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    binding.clear_values();

    // read the active revision directly from the live world
    let control = control();
    let table = control.lock();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let revision_id = RuntimeHandleCodec::encode_revision_id(world.revision())?;

    unsafe { out.write(revision_id) };

    Ok(())
}

/// Rewind one world to one checkpoint.
pub(crate) unsafe fn destack_runtime_world_rewind_checkpoint(
    binding: &BindingCallContext,
    argument_world: WorldHandle,
    checkpointid: CheckpointId,
) -> RuntimeResult<()> {
    binding.clear_values();

    // restore one live world to one checkpointed revision
    let control = control();
    let mut table = control.lock();
    let world = table.world_mut(RuntimeHandleCodec::decode_world_handle(argument_world))?;

    world.rewind(RuntimeHandleCodec::decode_checkpoint_id(checkpointid))
}

/// Rewind one world to one revision.
pub(crate) unsafe fn destack_runtime_world_rewind_revision(
    binding: &BindingCallContext,
    argument_world: WorldHandle,
    revisionid: RevisionId,
) -> RuntimeResult<()> {
    binding.clear_values();

    // restore one live world to one explicit revision
    let control = control();
    let mut table = control.lock();
    let world = table.world_mut(RuntimeHandleCodec::decode_world_handle(argument_world))?;

    world.rewind_revision(RuntimeHandleCodec::decode_revision_id(revisionid))
}

/// Close one runtime observation subscription.
pub(crate) unsafe fn destack_runtime_observation_close(
    binding: &BindingCallContext,
    handle: ObservationHandle,
) -> RuntimeResult<()> {
    binding.clear_values();

    // drop the external observe handle first
    let control = control();
    let mut table = control.lock();
    let entry = table.close_observation(RuntimeHandleCodec::decode_observation_handle(handle))?;
    let world = table.world_mut(entry.world_handle_id)?;

    world.observations().close(entry.subscription_id)
}

/// Read the next batch of observation records.
pub(crate) unsafe fn destack_runtime_observation_next(
    binding: &BindingCallContext,
    out: *mut NativeArray<ObservationRecord>,
    handle: ObservationHandle,
    limit: Option<u32>,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // read the next batch from the live subscription
    let control = control();
    let table = control.lock();
    let entry = table.observation_entry(RuntimeHandleCodec::decode_observation_handle(handle))?;
    let limit = RuntimeRequestCodec::list_limit_or_max(limit);
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let world = table.world(entry.world_handle_id)?;
    let records = world.observations().next(entry.subscription_id, limit)?;
    let records: NativeArray<ObservationRecord> = runtime_binding
        .encode::<NativeArray<ObservationRecord>>(RuntimeDescriptorCodec::observation_records(
            records,
        )?);

    unsafe {
        *out = records;
    }

    Ok(())
}

/// Open one runtime observation subscription.
pub(crate) unsafe fn destack_runtime_observation_open(
    binding: &BindingCallContext,
    out: *mut ObservationHandle,
    argument_world: WorldHandle,
    options: Option<ObservationOptions>,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // open one live observation subscription
    let options = options.unwrap_or(ObservationOptions {
        trace: None,
        topology: None,
        resources: None,
        scheduler: None,
        diagnostics: None,
        profiles: None,
    });
    let control = control();
    let mut table = control.lock();
    let world = table.world_mut(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let subscription_id =
        world
            .observations()
            .open(RuntimeRequestCodec::observation_options_from_flags(
                options.trace,
                options.topology,
                options.resources,
                options.scheduler,
                options.diagnostics,
                options.profiles,
            ));

    // register the observation handle
    let handle = RuntimeHandleCodec::encode_observation_handle(table.open_observation_handle(
        RuntimeHandleCodec::decode_world_handle(argument_world),
        subscription_id,
    )?);

    unsafe {
        *out = handle;
    }

    Ok(())
}

/// Export one snapshot from one image.
pub(crate) unsafe fn destack_runtime_snapshot_create(
    binding: &BindingCallContext,
    out: *mut SnapshotId,
    argument_world: WorldHandle,
    imageid: ImageId,
    format: SnapshotFormat,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve the live world and export one snapshot
    let (snapshot, bytes) = {
        let control = control();
        let table = control.lock();
        let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
        let snapshot = world.snapshot(RuntimeHandleCodec::decode_image_id(imageid))?;
        let bytes = Arc::<[u8]>::from(snapshot.encode()?);

        (snapshot, bytes)
    };

    // store the exported snapshot
    let control = control();
    let mut table = control.lock();
    let snapshot_id = RuntimeHandleCodec::encode_snapshot_id(table.store_snapshot_handle(
        RuntimeHandleCodec::decode_world_handle(argument_world),
        RuntimeHandleCodec::decode_snapshot_format(format),
        snapshot,
        bytes,
    )?);

    unsafe { out.write(snapshot_id) };

    Ok(())
}

/// Describe one snapshot.
pub(crate) unsafe fn destack_runtime_snapshot_describe(
    binding: &BindingCallContext,
    out: *mut SnapshotDescriptor,
    argument_world: WorldHandle,
    snapshotid: SnapshotId,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve the stored snapshot for this world
    let control = control();
    let table = control.lock();
    let entry = RuntimeDescriptorCodec::snapshot_entry(
        table.snapshot(RuntimeHandleCodec::decode_snapshot_id(snapshotid))?,
    );
    if entry.world != argument_world {
        return Err(io_not_found(
            "destack.runtime.snapshot.describe",
            format!("unknown snapshot {}", snapshotid.0),
        ));
    }
    let runtime_binding = NativeRuntimeBinding::new(binding);
    let descriptor: SnapshotDescriptor = runtime_binding.encode::<SnapshotDescriptor>(
        RuntimeDescriptorCodec::snapshot_descriptor(snapshotid, &entry)?,
    );

    unsafe { out.write(descriptor) };

    Ok(())
}

/// Import one serialized snapshot payload.
pub(crate) unsafe fn destack_runtime_snapshot_import(
    binding: &BindingCallContext,
    out: *mut SnapshotId,
    argument_world: WorldHandle,
    argument_payload: NativeArray<u8>,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;

    // resolve the live world before storing the imported snapshot
    let control = control();
    let table = control.lock();
    let _world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;

    // decode one stored snapshot payload
    let payload = unsafe { argument_payload.as_slice()? }.to_vec();
    let snapshot = runtime::world::WorldSnapshot::decode(&payload)?;
    let bytes = Arc::<[u8]>::from(payload);

    // clear call-local output storage
    binding.clear_values();
    drop(table);

    // store the imported snapshot
    let snapshot_id = RuntimeHandleCodec::encode_snapshot_id({
        let mut table = control.lock();
        table.store_snapshot_handle(
            RuntimeHandleCodec::decode_world_handle(argument_world),
            RuntimeHandleCodec::decode_snapshot_format(SnapshotFormat::Portable),
            snapshot,
            bytes,
        )?
    });

    unsafe { out.write(snapshot_id) };

    Ok(())
}

/// List snapshots in one world lineage.
pub(crate) unsafe fn destack_runtime_snapshot_list(
    binding: &BindingCallContext,
    out: *mut NativeArray<SnapshotDescriptor>,
    argument_world: WorldHandle,
    after: Option<SnapshotId>,
    limit: Option<u32>,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // enumerate stored snapshots for this world
    let limit = RuntimeRequestCodec::list_limit(limit);
    let control = control();
    let table = control.lock();
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
            let runtime_binding = NativeRuntimeBinding::new(binding);
            let descriptor: SnapshotDescriptor = runtime_binding.encode(
                RuntimeDescriptorCodec::snapshot_descriptor(snapshot_id, &entry)?,
            );

            Ok(descriptor)
        })
        .collect::<RuntimeResult<Vec<_>>>()?;

    unsafe { out.write(binding.store_array(descriptors)) };

    Ok(())
}

/// Read one serialized snapshot payload.
pub(crate) unsafe fn destack_runtime_snapshot_read(
    binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    argument_world: WorldHandle,
    snapshotid: SnapshotId,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve the stored snapshot payload for this world
    let control = control();
    let table = control.lock();
    let entry = RuntimeDescriptorCodec::snapshot_entry(
        table.snapshot(RuntimeHandleCodec::decode_snapshot_id(snapshotid))?,
    );
    if entry.world != argument_world {
        return Err(io_not_found(
            "destack.runtime.snapshot.read",
            format!("unknown snapshot {}", snapshotid.0),
        ));
    }

    unsafe { out.write(binding.store_array_copy(entry.bytes.as_ref())) };

    Ok(())
}

/// Restore one world from one image.
pub(crate) unsafe fn destack_runtime_restore_image(
    binding: &BindingCallContext,
    argument_world: WorldHandle,
    imageid: ImageId,
) -> RuntimeResult<()> {
    binding.clear_values();

    // restore the requested image into the live world
    let owner = control();
    let mut table = owner.lock();
    let world = table.world_mut(RuntimeHandleCodec::decode_world_handle(argument_world))?;

    world.restore_image_id(RuntimeHandleCodec::decode_image_id(imageid), None)
}

/// Restore one world from one snapshot.
pub(crate) unsafe fn destack_runtime_restore_snapshot(
    binding: &BindingCallContext,
    argument_world: WorldHandle,
    snapshotid: SnapshotId,
) -> RuntimeResult<()> {
    binding.clear_values();

    // resolve the snapshot and restore it into the live world
    let control = control();
    let table = control.lock();
    let entry = RuntimeDescriptorCodec::snapshot_entry(
        table.snapshot(RuntimeHandleCodec::decode_snapshot_id(snapshotid))?,
    );
    if entry.world != argument_world {
        return Err(io_not_found(
            "destack.runtime.snapshot.restoreSnapshot",
            format!("unknown snapshot {}", snapshotid.0),
        ));
    }

    drop(table);

    let mut table = control.lock();
    let world = table.world_mut(RuntimeHandleCodec::decode_world_handle(argument_world))?;

    world.restore_snapshot(&entry.snapshot, None)
}

/// Close one causal trace cursor.
pub(crate) unsafe fn destack_runtime_trace_close(
    binding: &BindingCallContext,
    cursor: TraceCursorHandle,
) -> RuntimeResult<()> {
    binding.clear_values();

    // drop the external cursor handle
    let control = control();
    let mut table = control.lock();
    table.close_trace_cursor(RuntimeHandleCodec::decode_trace_cursor_handle(cursor))?;

    Ok(())
}

/// Describe one world's causal trace.
pub(crate) unsafe fn destack_runtime_trace_describe(
    binding: &BindingCallContext,
    out: *mut TraceDescriptor,
    argument_world: WorldHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    binding.clear_values();

    // summarize the active world trace
    let control = control();
    let table = control.lock();
    let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let descriptor = TraceDescriptor {
        branch_id: RuntimeHandleCodec::encode_branch_id(world.branch_id())?,
        sequence: TraceSequence(world.trace().log().next_sequence().get()),
    };

    unsafe { out.write(descriptor) };

    Ok(())
}

/// Append one explicit trace marker.
pub(crate) unsafe fn destack_runtime_trace_mark(
    binding: &BindingCallContext,
    out: *mut TraceSequence,
    argument_world: WorldHandle,
    label: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the marker label before clearing call-local values
    let label = unsafe { label.as_str()? }.to_string();

    // clear call-local output storage
    binding.clear_values();

    // resolve the live world and record the marker
    let control = control();
    let mut table = control.lock();
    let world = table.world_mut(RuntimeHandleCodec::decode_world_handle(argument_world))?;
    let sequence = world.label(label)?;

    unsafe { out.write(TraceSequence(sequence.get())) };

    Ok(())
}

/// Read the next batch of causal trace records.
pub(crate) unsafe fn destack_runtime_trace_next(
    binding: &BindingCallContext,
    out: *mut NativeArray<TraceRecord>,
    cursor: TraceCursorHandle,
    limit: Option<u32>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    binding.clear_values();

    // read the next batch from one live cursor
    let control = control();
    let table = control.lock();
    let entry = table.trace_cursor_entry(RuntimeHandleCodec::decode_trace_cursor_handle(cursor))?;
    let cursor = entry.cursor;
    let limit = RuntimeRequestCodec::list_limit_or_max(limit);
    let mut records = Vec::new();

    for _ in 0..limit {
        let sequence = cursor.sequence();
        let Some(event) = cursor.next_event()? else {
            break;
        };

        records.push((sequence, event));
    }

    let runtime_binding = NativeRuntimeBinding::new(binding);

    let records: NativeArray<TraceRecord> = runtime_binding
        .encode::<NativeArray<TraceRecord>>(RuntimeDescriptorCodec::trace_records(records)?);

    unsafe { out.write(records) };

    Ok(())
}

/// Open one causal trace cursor.
pub(crate) unsafe fn destack_runtime_trace_open(
    binding: &BindingCallContext,
    out: *mut TraceCursorHandle,
    argument_world: WorldHandle,
    options: Option<TraceCursorOptions>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    binding.clear_values();

    // open one live cursor at the requested sequence
    let cursor = {
        let control = control();
        let table = control.lock();
        let world = table.world(RuntimeHandleCodec::decode_world_handle(argument_world))?;
        Arc::new(world.trace().log().reader())
    };
    let options = options.unwrap_or(TraceCursorOptions {
        start_sequence: None,
    });
    if let Some(start_sequence) = options.start_sequence {
        cursor.seek_sequence(runtime::trace::TraceSequence::new(start_sequence.0))?;
    }

    // register the trace cursor
    let control = control();
    let mut table = control.lock();
    let handle = RuntimeHandleCodec::encode_trace_cursor_handle(table.open_trace_cursor_handle(
        RuntimeHandleCodec::decode_world_handle(argument_world),
        cursor,
    )?);
    unsafe { out.write(handle) };

    Ok(())
}

/// Seek one causal trace cursor to one checkpoint boundary.
pub(crate) unsafe fn destack_runtime_trace_seek_checkpoint(
    binding: &BindingCallContext,
    cursor: TraceCursorHandle,
    checkpointid: CheckpointId,
) -> RuntimeResult<()> {
    binding.clear_values();

    // seek to the revision sequence anchored by one checkpoint
    let control = control();
    let table = control.lock();
    let entry = table.trace_cursor_entry(RuntimeHandleCodec::decode_trace_cursor_handle(cursor))?;
    let world = table.world(entry.world_handle_id)?;
    let checkpoint =
        world.checkpoint_info(RuntimeHandleCodec::decode_checkpoint_id(checkpointid))?;
    let revision = world.revision_state(checkpoint.revision)?;

    entry.cursor.seek_sequence(revision.sequence)
}

/// Seek one causal trace cursor to one revision boundary.
pub(crate) unsafe fn destack_runtime_trace_seek_revision(
    binding: &BindingCallContext,
    cursor: TraceCursorHandle,
    revisionid: RevisionId,
) -> RuntimeResult<()> {
    binding.clear_values();

    // seek to the sequence captured by one revision
    let control = control();
    let table = control.lock();
    let entry = table.trace_cursor_entry(RuntimeHandleCodec::decode_trace_cursor_handle(cursor))?;
    let world = table.world(entry.world_handle_id)?;
    let revision = world.revision_state(RuntimeHandleCodec::decode_revision_id(revisionid))?;

    entry.cursor.seek_sequence(revision.sequence)
}

/// Seek one causal trace cursor to one sequence.
pub(crate) unsafe fn destack_runtime_trace_seek_sequence(
    binding: &BindingCallContext,
    cursor: TraceCursorHandle,
    sequence: TraceSequence,
) -> RuntimeResult<()> {
    binding.clear_values();

    // seek one live cursor directly to one sequence boundary
    let control = control();
    let table = control.lock();
    let entry = table.trace_cursor_entry(RuntimeHandleCodec::decode_trace_cursor_handle(cursor))?;

    entry
        .cursor
        .seek_sequence(runtime::trace::TraceSequence::new(sequence.0))
}

/// Return the current sequence position of one causal trace cursor.
pub(crate) unsafe fn destack_runtime_trace_tell(
    binding: &BindingCallContext,
    out: *mut TraceSequence,
    cursor: TraceCursorHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    binding.clear_values();

    // expose the next visible sequence for one live cursor
    let control = control();
    let table = control.lock();
    let entry = table.trace_cursor_entry(RuntimeHandleCodec::decode_trace_cursor_handle(cursor))?;
    let sequence = entry.cursor.sequence();
    let sequence = TraceSequence(sequence.get());

    unsafe { out.write(sequence) };

    Ok(())
}
