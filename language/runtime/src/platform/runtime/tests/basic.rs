use std::sync::Arc;

use destack_core::LocalStringPool;
use destack_mir::NodeTree;
use destack_vm as vm;
use destack_workspace::RuntimeOptions;

use crate::diagnostic::RuntimeResult;
use crate::platform::runtime::{
    AgentDescriptor, AgentDescriptorVm, AgentFilter, AgentFilterVm, AgentId, BranchDescriptor,
    BranchDescriptorVm, BranchFilter, BranchFilterVm, CheckpointDescriptor, CheckpointDescriptorVm,
    CheckpointFilter, CheckpointFilterVm, ImageDescriptor, ImageDescriptorVm, ImageFilter,
    ImageFilterVm, ObservationEventKind, ObservationOptions, ObservationOptionsVm,
    ObservationRecord, ObservationRecordVm, ResourceFilter, ResourceFilterVm, RevisionDescriptor,
    RevisionDescriptorVm, RevisionFilter, RevisionFilterVm, RuntimeDescriptor, RuntimeDescriptorVm,
    RuntimeEngineKind, RuntimeExecutionMode, RuntimeFilter, RuntimeFilterVm, RuntimeId,
    RuntimeLabel, RuntimeLabelVm, RuntimeTickOutcome, RuntimeWorldKind, SnapshotDescriptor,
    SnapshotDescriptorVm, SnapshotFormat, TopologyEdgeFilter, TopologyEdgeFilterVm, TopologyEdgeId,
    TopologyEdgeIdVm, TopologyEdgeKind, TopologyEdgeKindVm, TopologyEntityFilter,
    TopologyEntityFilterVm, TopologyEntityId, TopologyEntityIdVm, TopologyEntityKind,
    TopologyEntityKindVm, TraceCursorOptions, TraceCursorOptionsVm, TraceDescriptor,
    TraceDescriptorVm, TraceEventKind, TraceRecord, TraceRecordVm, WorldCreateOptions,
    WorldCreateOptionsVm, WorldResourceId as RuntimeWorldResourceId,
    WorldResourceIdVm as RuntimeWorldResourceIdVm, WorldViewOptions, WorldViewOptionsVm,
};
use crate::platform::{
    NativeArray, NativeStringRef, ResourceBacking, ResourceCapture, ResourceId,
    ResourcePortability, VmArray,
};
use crate::runtime::control::{ControlHandleId, control_table};
use crate::runtime::time::WorldInstant;
use crate::runtime::world::{Snapshot as WorldSnapshot, World};
use crate::runtime::{WorldEntityKind, WorldResource, WorldResourceId as LogicalWorldResourceId};

use super::tests::{HarnessValue, RuntimeHarnessContext, with_harness_context};

fn world_create_options_with_execution(
    context: &mut RuntimeHarnessContext<'_>,
    execution: RuntimeExecutionMode,
) -> RuntimeResult<HarnessValue<Option<WorldCreateOptions>, Option<WorldCreateOptionsVm>>> {
    // native and vm world options
    if let Some(vm_context) = context.vm_context {
        let _vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };

        return Ok(context.harness_value_vm(Some(WorldCreateOptionsVm {
            engine: Some(RuntimeEngineKind::Vm),
            execution: Some(execution),
            world: Some(RuntimeWorldKind::Simulation),
            labels: None,
        })));
    }

    Ok(context.harness_value(Some(WorldCreateOptions {
        engine: Some(RuntimeEngineKind::Vm),
        execution: Some(execution),
        world: Some(RuntimeWorldKind::Simulation),
        labels: None,
    })))
}

fn world_create_options(
    context: &mut RuntimeHarnessContext<'_>,
) -> RuntimeResult<HarnessValue<Option<WorldCreateOptions>, Option<WorldCreateOptionsVm>>> {
    world_create_options_with_execution(context, RuntimeExecutionMode::Deterministic)
}

fn scheduler_observe_options(
    context: &mut RuntimeHarnessContext<'_>,
) -> HarnessValue<Option<ObservationOptions>, Option<ObservationOptionsVm>> {
    // scheduler-only feed
    if context.vm_context.is_some() {
        return context.harness_value_vm(Some(ObservationOptionsVm {
            trace: Some(false),
            topology: Some(false),
            resources: Some(false),
            scheduler: Some(true),
            diagnostics: Some(false),
            profiles: Some(false),
        }));
    }

    context.harness_value(Some(ObservationOptions {
        trace: Some(false),
        topology: Some(false),
        resources: Some(false),
        scheduler: Some(true),
        diagnostics: Some(false),
        profiles: Some(false),
    }))
}

fn live_world(handle: crate::platform::runtime::WorldHandle) -> RuntimeResult<Arc<World>> {
    let control_table = control_table().read();

    control_table.world(ControlHandleId::new(handle.0.0))
}

fn empty_runtime_labels(
    context: &mut RuntimeHarnessContext<'_>,
) -> RuntimeResult<HarnessValue<Option<NativeArray<RuntimeLabel>>, Option<VmArray<RuntimeLabelVm>>>>
{
    // empty label array
    if context.vm_context.is_some() {
        return Ok(context.harness_value_vm(None));
    }

    Ok(context.harness_value(None))
}

fn runtime_name(
    context: &mut RuntimeHarnessContext<'_>,
    value: &str,
) -> HarnessValue<Option<NativeStringRef>, Option<vm::StringHandle>> {
    // native and vm string value
    if let Some(vm_context) = context.vm_context {
        let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
        return context
            .harness_value_vm(Some(vm::StringHandle::new(vm_context.intern_string(value))));
    }

    context.harness_value(Some(NativeStringRef::from(value)))
}

fn runtime_string(
    context: &mut RuntimeHarnessContext<'_>,
    value: &str,
) -> HarnessValue<NativeStringRef, vm::StringHandle> {
    // native and vm string value
    if let Some(vm_context) = context.vm_context {
        let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
        return context.harness_value_vm(vm::StringHandle::new(vm_context.intern_string(value)));
    }

    context.harness_value(NativeStringRef::from(value))
}

fn decode_observe_records(
    context: &RuntimeHarnessContext<'_>,
    records: HarnessValue<NativeArray<ObservationRecord>, VmArray<ObservationRecordVm>>,
) -> RuntimeResult<Vec<(ObservationEventKind, u64)>> {
    // decode native or vm records
    match records {
        HarnessValue::Native(records) => {
            let records = unsafe { records.as_slice()? };

            Ok(records
                .iter()
                .map(|record| {
                    (
                        record.kind,
                        record.sequence.map(|sequence| sequence.0).unwrap_or(0),
                    )
                })
                .collect())
        }
        HarnessValue::Vm(records) => {
            let vm_context = unsafe {
                &*(context.vm_context.expect("vm context") as *mut vm::ExternalCallContext<'_>)
            };
            let records = records.read_values(vm_context)?;

            Ok(records
                .iter()
                .map(|record| {
                    (
                        record.kind,
                        record.sequence.map(|sequence| sequence.0).unwrap_or(0),
                    )
                })
                .collect())
        }
    }
}

fn decode_branch_descriptor(
    context: &RuntimeHarnessContext<'_>,
    descriptor: HarnessValue<BranchDescriptor, BranchDescriptorVm>,
) -> RuntimeResult<(u64, u64, String)> {
    // decode one branch descriptor
    match descriptor {
        HarnessValue::Native(descriptor) => Ok((
            descriptor.id.0,
            descriptor.head_revision.0,
            match descriptor.name {
                Some(name) => unsafe { name.as_str()? }.to_string(),
                None => String::new(),
            },
        )),
        HarnessValue::Vm(descriptor) => {
            let vm_context = unsafe {
                &*(context.vm_context.expect("vm context") as *mut vm::ExternalCallContext<'_>)
            };
            let name = vm_context
                .string_ref(descriptor.name.unwrap())
                .map_err(Box::<crate::diagnostic::RuntimeError>::from)?
                .as_str()
                .to_string();

            Ok((descriptor.id.0, descriptor.head_revision.0, name))
        }
    }
}

fn decode_revision_descriptor(
    descriptor: HarnessValue<RevisionDescriptor, RevisionDescriptorVm>,
) -> (u64, u64, Option<u64>, u64, u64) {
    // decode one revision descriptor
    match descriptor {
        HarnessValue::Native(descriptor) => (
            descriptor.id.0,
            descriptor.branch_id.0,
            descriptor.parent_revision.map(|revision| revision.0),
            descriptor.image_id.0,
            descriptor.sequence.0,
        ),
        HarnessValue::Vm(descriptor) => (
            descriptor.id.0,
            descriptor.branch_id.0,
            descriptor.parent_revision.map(|revision| revision.0),
            descriptor.image_id.0,
            descriptor.sequence.0,
        ),
    }
}

fn decode_checkpoint_descriptor(
    context: &RuntimeHarnessContext<'_>,
    descriptor: HarnessValue<CheckpointDescriptor, CheckpointDescriptorVm>,
) -> RuntimeResult<(u64, u64, String)> {
    // decode one checkpoint descriptor
    match descriptor {
        HarnessValue::Native(descriptor) => Ok((
            descriptor.id.0,
            descriptor.revision_id.0,
            match descriptor.name {
                Some(name) => unsafe { name.as_str()? }.to_string(),
                None => String::new(),
            },
        )),
        HarnessValue::Vm(descriptor) => {
            let vm_context = unsafe {
                &*(context.vm_context.expect("vm context") as *mut vm::ExternalCallContext<'_>)
            };
            let name = vm_context
                .string_ref(descriptor.name.unwrap())
                .map_err(Box::<crate::diagnostic::RuntimeError>::from)?
                .as_str()
                .to_string();

            Ok((descriptor.id.0, descriptor.revision_id.0, name))
        }
    }
}

fn decode_image_descriptor(
    descriptor: HarnessValue<ImageDescriptor, ImageDescriptorVm>,
) -> (u64, u64, Option<u64>) {
    // decode one image descriptor
    match descriptor {
        HarnessValue::Native(descriptor) => (
            descriptor.id.0,
            descriptor.revision_id.0,
            descriptor.shared_bytes,
        ),
        HarnessValue::Vm(descriptor) => (
            descriptor.id.0,
            descriptor.revision_id.0,
            descriptor.shared_bytes,
        ),
    }
}

fn decode_snapshot_descriptor(
    descriptor: HarnessValue<SnapshotDescriptor, SnapshotDescriptorVm>,
) -> (u64, u64, SnapshotFormat, Option<u64>) {
    // decode one snapshot descriptor
    match descriptor {
        HarnessValue::Native(descriptor) => (
            descriptor.id.0,
            descriptor.image_id.0,
            descriptor.format,
            descriptor.size_bytes,
        ),
        HarnessValue::Vm(descriptor) => (
            descriptor.id.0,
            descriptor.image_id.0,
            descriptor.format,
            descriptor.size_bytes,
        ),
    }
}

fn trace_cursor_options(
    context: &mut RuntimeHarnessContext<'_>,
) -> HarnessValue<Option<TraceCursorOptions>, Option<TraceCursorOptionsVm>> {
    // default trace cursor options
    if context.vm_context.is_some() {
        return context.harness_value_vm(Some(TraceCursorOptionsVm {
            start_sequence: None,
        }));
    }

    context.harness_value(Some(TraceCursorOptions {
        start_sequence: None,
    }))
}

fn runtime_view_options(
    context: &mut RuntimeHarnessContext<'_>,
    revision_id: Option<u64>,
) -> HarnessValue<Option<WorldViewOptions>, Option<WorldViewOptionsVm>> {
    // pin one explicit revision or the current head
    if context.vm_context.is_some() {
        return context.harness_value_vm(Some(WorldViewOptionsVm {
            revision_id: revision_id.map(crate::platform::runtime::RevisionId),
        }));
    }

    context.harness_value(Some(WorldViewOptions {
        revision_id: revision_id.map(crate::platform::runtime::RevisionId),
    }))
}

fn branch_list_filter(
    context: &mut RuntimeHarnessContext<'_>,
    name: Option<&str>,
) -> HarnessValue<Option<BranchFilter>, Option<BranchFilterVm>> {
    // optional exact branch-name filter
    if context.vm_context.is_some() {
        return context.harness_value_vm(Some(BranchFilterVm {
            name: name.map(|name| {
                let vm_context = unsafe {
                    &mut *(context.vm_context.expect("vm context")
                        as *mut vm::ExternalCallContext<'_>)
                };
                vm::StringHandle::new(vm_context.intern_string(name))
            }),
            labels: None,
        }));
    }

    context.harness_value(Some(BranchFilter {
        name: name.map(NativeStringRef::from),
        labels: None,
    }))
}

fn revision_list_filter(
    context: &mut RuntimeHarnessContext<'_>,
    branch_id: Option<u64>,
) -> HarnessValue<Option<RevisionFilter>, Option<RevisionFilterVm>> {
    // optional branch-scoped revision filter
    if context.vm_context.is_some() {
        return context.harness_value_vm(Some(RevisionFilterVm {
            branch_id: branch_id.map(crate::platform::runtime::BranchId),
        }));
    }

    context.harness_value(Some(RevisionFilter {
        branch_id: branch_id.map(crate::platform::runtime::BranchId),
    }))
}

fn checkpoint_list_filter(
    context: &mut RuntimeHarnessContext<'_>,
    revision_id: Option<u64>,
    name: Option<&str>,
) -> HarnessValue<Option<CheckpointFilter>, Option<CheckpointFilterVm>> {
    // optional revision and name filter
    if context.vm_context.is_some() {
        return context.harness_value_vm(Some(CheckpointFilterVm {
            revision_id: revision_id.map(crate::platform::runtime::RevisionId),
            name: name.map(|name| {
                let vm_context = unsafe {
                    &mut *(context.vm_context.expect("vm context")
                        as *mut vm::ExternalCallContext<'_>)
                };
                vm::StringHandle::new(vm_context.intern_string(name))
            }),
            labels: None,
        }));
    }

    context.harness_value(Some(CheckpointFilter {
        revision_id: revision_id.map(crate::platform::runtime::RevisionId),
        name: name.map(NativeStringRef::from),
        labels: None,
    }))
}

fn image_list_filter(
    context: &mut RuntimeHarnessContext<'_>,
    revision_id: Option<u64>,
) -> HarnessValue<Option<ImageFilter>, Option<ImageFilterVm>> {
    // optional revision-scoped image filter
    if context.vm_context.is_some() {
        return context.harness_value_vm(Some(ImageFilterVm {
            revision_id: revision_id.map(crate::platform::runtime::RevisionId),
        }));
    }

    context.harness_value(Some(ImageFilter {
        revision_id: revision_id.map(crate::platform::runtime::RevisionId),
    }))
}

fn runtime_list_filter(
    context: &mut RuntimeHarnessContext<'_>,
) -> HarnessValue<Option<RuntimeFilter>, Option<RuntimeFilterVm>> {
    // no runtime filter
    if context.vm_context.is_some() {
        return context.harness_value_vm(None);
    }

    context.harness_value(None)
}

fn agent_list_filter(
    context: &mut RuntimeHarnessContext<'_>,
    runtime_id: Option<u64>,
) -> HarnessValue<Option<AgentFilter>, Option<AgentFilterVm>> {
    // runtime-scoped agent filter
    if context.vm_context.is_some() {
        return context.harness_value_vm(Some(AgentFilterVm {
            runtime_id: runtime_id.map(RuntimeId),
            name: None,
            has_pending_work: None,
            labels: None,
        }));
    }

    context.harness_value(Some(AgentFilter {
        runtime_id: runtime_id.map(RuntimeId),
        name: None,
        has_pending_work: None,
        labels: None,
    }))
}

fn resource_list_filter(
    context: &mut RuntimeHarnessContext<'_>,
    agent_id: Option<u64>,
) -> HarnessValue<Option<ResourceFilter>, Option<ResourceFilterVm>> {
    // agent-scoped resource filter
    if context.vm_context.is_some() {
        return context.harness_value_vm(Some(ResourceFilterVm {
            runtime_id: None,
            agent_id: agent_id.map(AgentId),
            kind: None,
            label: None,
        }));
    }

    context.harness_value(Some(ResourceFilter {
        runtime_id: None,
        agent_id: agent_id.map(AgentId),
        kind: None,
        label: None,
    }))
}

fn topology_entity_id(
    context: &mut RuntimeHarnessContext<'_>,
    value: &str,
) -> HarnessValue<TopologyEntityId, TopologyEntityIdVm> {
    // native and vm topology entity id
    if let Some(vm_context) = context.vm_context {
        let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };

        return context.harness_value_vm(crate::platform::runtime::TopologyEntityIdAbi(
            vm::StringHandle::new(vm_context.intern_string(value)),
        ));
    }

    context.harness_value(crate::platform::runtime::TopologyEntityIdAbi(
        NativeStringRef::from(value),
    ))
}

fn topology_entity_kind(
    context: &mut RuntimeHarnessContext<'_>,
    value: &str,
) -> HarnessValue<TopologyEntityKind, TopologyEntityKindVm> {
    // native and vm topology entity kind
    if let Some(vm_context) = context.vm_context {
        let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };

        return context.harness_value_vm(crate::platform::runtime::TopologyEntityKindAbi(
            vm::StringHandle::new(vm_context.intern_string(value)),
        ));
    }

    context.harness_value(crate::platform::runtime::TopologyEntityKindAbi(
        NativeStringRef::from(value),
    ))
}

fn topology_edge_id(
    context: &mut RuntimeHarnessContext<'_>,
    value: &str,
) -> HarnessValue<TopologyEdgeId, TopologyEdgeIdVm> {
    // native and vm topology edge id
    if let Some(vm_context) = context.vm_context {
        let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };

        return context.harness_value_vm(crate::platform::runtime::TopologyEdgeIdAbi(
            vm::StringHandle::new(vm_context.intern_string(value)),
        ));
    }

    context.harness_value(crate::platform::runtime::TopologyEdgeIdAbi(
        NativeStringRef::from(value),
    ))
}

fn topology_edge_kind(
    context: &mut RuntimeHarnessContext<'_>,
    value: &str,
) -> HarnessValue<TopologyEdgeKind, TopologyEdgeKindVm> {
    // native and vm topology edge kind
    if let Some(vm_context) = context.vm_context {
        let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };

        return context.harness_value_vm(crate::platform::runtime::TopologyEdgeKindAbi(
            vm::StringHandle::new(vm_context.intern_string(value)),
        ));
    }

    context.harness_value(crate::platform::runtime::TopologyEdgeKindAbi(
        NativeStringRef::from(value),
    ))
}

fn no_resource_after(
    context: &mut RuntimeHarnessContext<'_>,
) -> HarnessValue<Option<RuntimeWorldResourceId>, Option<RuntimeWorldResourceIdVm>> {
    // no resource cursor
    if context.vm_context.is_some() {
        return context.harness_value_vm(None);
    }

    context.harness_value(None)
}

fn no_entity_after(
    context: &mut RuntimeHarnessContext<'_>,
) -> HarnessValue<Option<TopologyEntityId>, Option<TopologyEntityIdVm>> {
    // no entity cursor
    if context.vm_context.is_some() {
        return context.harness_value_vm(None);
    }

    context.harness_value(None)
}

fn no_edge_after(
    context: &mut RuntimeHarnessContext<'_>,
) -> HarnessValue<Option<TopologyEdgeId>, Option<TopologyEdgeIdVm>> {
    // no edge cursor
    if context.vm_context.is_some() {
        return context.harness_value_vm(None);
    }

    context.harness_value(None)
}

fn world_resource_id(
    context: &mut RuntimeHarnessContext<'_>,
    agent_id: u64,
    resource_id: u64,
) -> HarnessValue<RuntimeWorldResourceId, RuntimeWorldResourceIdVm> {
    // native and vm logical resource id
    if context.vm_context.is_some() {
        return context.harness_value_vm(RuntimeWorldResourceIdVm {
            agent_id: AgentId(agent_id),
            resource_id: ResourceId(resource_id),
        });
    }

    context.harness_value(RuntimeWorldResourceId {
        agent_id: AgentId(agent_id),
        resource_id: ResourceId(resource_id),
    })
}

fn inspect_vm_engine() -> vm::Isolate {
    // empty vm isolate for runtime inspect tests
    let tree = NodeTree::new();
    let strings = LocalStringPool::new().into_immutable();

    vm::Isolate::build(tree, strings).expect("runtime inspect vm isolate should build")
}

fn decode_trace_descriptor(
    descriptor: HarnessValue<TraceDescriptor, TraceDescriptorVm>,
) -> (u64, u64) {
    // decode one trace descriptor
    match descriptor {
        HarnessValue::Native(descriptor) => (descriptor.branch_id.0, descriptor.sequence.0),
        HarnessValue::Vm(descriptor) => (descriptor.branch_id.0, descriptor.sequence.0),
    }
}

fn decode_runtime_descriptor(
    context: &RuntimeHarnessContext<'_>,
    descriptor: HarnessValue<RuntimeDescriptor, RuntimeDescriptorVm>,
) -> RuntimeResult<(u64, u64, String, u32)> {
    // decode one runtime descriptor
    match descriptor {
        HarnessValue::Native(descriptor) => Ok((
            descriptor.id.0,
            descriptor.primary_agent_id.0,
            match descriptor.name {
                Some(name) => unsafe { name.as_str()? }.to_string(),
                None => String::new(),
            },
            descriptor.agent_count,
        )),
        HarnessValue::Vm(descriptor) => {
            let vm_context = unsafe {
                &*(context.vm_context.expect("vm context") as *mut vm::ExternalCallContext<'_>)
            };
            let name = descriptor
                .name
                .map(|name| {
                    vm_context
                        .string_ref(name)
                        .map(|name| name.as_str().to_string())
                })
                .transpose()
                .map_err(Box::<crate::diagnostic::RuntimeError>::from)?
                .unwrap_or_default();

            Ok((
                descriptor.id.0,
                descriptor.primary_agent_id.0,
                name,
                descriptor.agent_count,
            ))
        }
    }
}

fn decode_agent_descriptor(
    context: &RuntimeHarnessContext<'_>,
    descriptor: HarnessValue<AgentDescriptor, AgentDescriptorVm>,
) -> RuntimeResult<(u64, u64, String, bool, u32)> {
    // decode one agent descriptor
    match descriptor {
        HarnessValue::Native(descriptor) => Ok((
            descriptor.id.0,
            descriptor.runtime_id.0,
            match descriptor.name {
                Some(name) => unsafe { name.as_str()? }.to_string(),
                None => String::new(),
            },
            descriptor.has_pending_work,
            descriptor.resource_count,
        )),
        HarnessValue::Vm(descriptor) => {
            let vm_context = unsafe {
                &*(context.vm_context.expect("vm context") as *mut vm::ExternalCallContext<'_>)
            };
            let name = descriptor
                .name
                .map(|name| {
                    vm_context
                        .string_ref(name)
                        .map(|name| name.as_str().to_string())
                })
                .transpose()
                .map_err(Box::<crate::diagnostic::RuntimeError>::from)?
                .unwrap_or_default();

            Ok((
                descriptor.id.0,
                descriptor.runtime_id.0,
                name,
                descriptor.has_pending_work,
                descriptor.resource_count,
            ))
        }
    }
}

fn decode_trace_records(
    context: &RuntimeHarnessContext<'_>,
    records: HarnessValue<NativeArray<TraceRecord>, VmArray<TraceRecordVm>>,
) -> RuntimeResult<Vec<(u64, TraceEventKind)>> {
    // decode native or vm trace records
    match records {
        HarnessValue::Native(records) => {
            let records = unsafe { records.as_slice()? };

            Ok(records
                .iter()
                .map(|record| (record.sequence.0, record.kind))
                .collect())
        }
        HarnessValue::Vm(records) => {
            let vm_context = unsafe {
                &*(context.vm_context.expect("vm context") as *mut vm::ExternalCallContext<'_>)
            };
            let records = records.read_values(vm_context)?;

            Ok(records
                .iter()
                .map(|record| (record.sequence.0, record.kind))
                .collect())
        }
    }
}

fn decode_snapshot_bytes(
    context: &RuntimeHarnessContext<'_>,
    bytes: HarnessValue<NativeArray<u8>, VmArray<u8>>,
) -> RuntimeResult<Vec<u8>> {
    // decode one serialized snapshot payload
    match bytes {
        HarnessValue::Native(bytes) => Ok(unsafe { bytes.as_slice()? }.to_vec()),
        HarnessValue::Vm(bytes) => {
            let vm_context = unsafe {
                &*(context.vm_context.expect("vm context") as *mut vm::ExternalCallContext<'_>)
            };

            Ok(bytes.read_bytes(vm_context)?)
        }
    }
}

fn snapshot_payload(
    context: &mut RuntimeHarnessContext<'_>,
    bytes: &[u8],
) -> HarnessValue<NativeArray<u8>, VmArray<u8>> {
    // native and vm byte payload
    if let Some(vm_context) = context.vm_context {
        let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
        return context.harness_value_vm(VmArray::from_bytes(vm_context, bytes));
    }

    context.harness_value(context.call_context.store_array(bytes.to_vec()))
}

#[cfg(any(unix, windows))]
#[test]
fn test_runtime_world_create_describe_and_close() {
    with_harness_context(|mut context| {
        // create and describe one low-level world
        let options = world_create_options(&mut context)?;
        let world = context.destack_runtime_world_create(options)?;
        let descriptor = context.destack_runtime_world_describe(world)?;

        // descriptor basics
        match descriptor {
            HarnessValue::Native(descriptor) => {
                assert_eq!(descriptor.handle, world);
                assert_eq!(descriptor.branch_id.0, 0);
                assert_eq!(descriptor.revision_id.0, 0);
                assert_eq!(descriptor.runtime_count, 0);
            }
            HarnessValue::Vm(descriptor) => {
                assert_eq!(descriptor.handle, world);
                assert_eq!(descriptor.branch_id.0, 0);
                assert_eq!(descriptor.revision_id.0, 0);
                assert_eq!(descriptor.runtime_count, 0);
            }
        }

        // release the world handle
        context.destack_runtime_world_close(world)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_runtime_observe_reports_scheduler_progress() {
    with_harness_context(|mut context| {
        // create one world and subscribe to scheduler events
        let world_options = world_create_options(&mut context)?;
        let world = context.destack_runtime_world_create(world_options)?;

        let observe_options = scheduler_observe_options(&mut context);
        let observe = context.destack_runtime_observation_open(world, observe_options)?;

        // queue one scheduled world event through the live handle
        let live_world = live_world(world)?;
        live_world
            .write_simulation()
            .schedule_event(WorldInstant::new(5_000));

        // tick through the low-level binding surface
        let outcome = context.destack_runtime_world_tick(world)?;
        assert_eq!(outcome, RuntimeTickOutcome::AdvancedTime);

        let records = context.destack_runtime_observation_next(observe, Some(8))?;
        let records = decode_observe_records(&context, records)?;
        assert_eq!(records, vec![(ObservationEventKind::Scheduler, 0)]);

        // close the feed and world
        context.destack_runtime_observation_close(observe)?;
        context.destack_runtime_world_close(world)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_runtime_lineage_controls_world_revisions() {
    with_harness_context(|mut context| {
        // create one low-level world
        let world_options = world_create_options(&mut context)?;
        let world = context.destack_runtime_world_create(world_options)?;

        // the fresh world starts on the root lineage
        let branch_id = context.destack_runtime_world_branch(world)?;
        let revision_id = context.destack_runtime_world_revision(world)?;
        assert_eq!(branch_id.0, 0);
        assert_eq!(revision_id.0, 0);

        // create one named checkpoint and inspect the new active revision
        let checkpoint_name = runtime_name(&mut context, "baseline");
        let checkpoint_labels = empty_runtime_labels(&mut context)?;
        let checkpoint_id =
            context.destack_runtime_checkpoint_create(world, checkpoint_name, checkpoint_labels)?;
        let revision_id = context.destack_runtime_world_revision(world)?;
        assert!(checkpoint_id.0 > 0);
        assert!(revision_id.0 > 0);

        let checkpoint_descriptor =
            context.destack_runtime_checkpoint_describe(world, checkpoint_id)?;
        let checkpoint = decode_checkpoint_descriptor(&context, checkpoint_descriptor)?;
        assert_eq!(checkpoint.0, checkpoint_id.0);
        assert_eq!(checkpoint.1, revision_id.0);
        assert_eq!(checkpoint.2, "baseline");

        // the captured revision should point at one materialized image
        let revision = decode_revision_descriptor(
            context.destack_runtime_revision_describe(world, revision_id)?,
        );
        assert_eq!(revision.0, revision_id.0);
        assert_eq!(revision.1, branch_id.0);
        assert!(revision.2.is_some());
        assert!(revision.3 > 0);

        // image capture should materialize one later revision and image pair
        let image_id = context.destack_runtime_image_capture(world)?;
        let image =
            decode_image_descriptor(context.destack_runtime_image_describe(world, image_id)?);
        assert_eq!(image.0, image_id.0);
        assert!(image.1 >= revision_id.0);
        assert!(image.2.is_some_and(|shared_bytes| shared_bytes > 0));

        // fork one child world from the checkpointed revision
        let child_name = runtime_name(&mut context, "child");
        let child_labels = empty_runtime_labels(&mut context)?;
        let child =
            context.destack_runtime_world_fork(world, revision_id, child_name, child_labels)?;
        let child_branch_id = context.destack_runtime_world_branch(child)?;
        let child_branch_descriptor =
            context.destack_runtime_branch_describe(child, child_branch_id)?;
        let child_branch = decode_branch_descriptor(&context, child_branch_descriptor)?;
        assert!(child_branch.0 > 0);
        assert_eq!(child_branch.1, revision_id.0);
        assert_eq!(child_branch.2, "child");

        // rewinding the parent to the checkpoint should preserve the anchored revision
        context.destack_runtime_world_rewind_checkpoint(world, checkpoint_id)?;
        let rewound_revision = context.destack_runtime_world_revision(world)?;
        assert_eq!(rewound_revision.0, checkpoint.1);

        // close both live worlds
        context.destack_runtime_world_close(child)?;
        context.destack_runtime_world_close(world)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_runtime_trace_cursor_reads_and_seeks_records() {
    with_harness_context(|mut context| {
        // create one record-mode world
        let world_options =
            world_create_options_with_execution(&mut context, RuntimeExecutionMode::Record)?;
        let world = context.destack_runtime_world_create(world_options)?;

        // open one trace cursor from the start
        let cursor_options = trace_cursor_options(&mut context);
        let cursor = context.destack_runtime_trace_open(world, cursor_options)?;

        // one scheduled deadline should record one control trace event
        let live_world = live_world(world)?;
        live_world
            .write_simulation()
            .schedule_event(WorldInstant::new(5_000));
        let outcome = context.destack_runtime_world_tick(world)?;
        assert_eq!(outcome, RuntimeTickOutcome::AdvancedTime);

        let descriptor = decode_trace_descriptor(context.destack_runtime_trace_describe(world)?);
        assert_eq!(descriptor.0, 0);
        assert_eq!(descriptor.1, 1);

        // explicit markers should append one marker event
        let marker = runtime_string(&mut context, "checkpoint-a");
        let marker_sequence = context.destack_runtime_trace_mark(world, marker)?;
        assert_eq!(marker_sequence.0, 1);

        let records = context.destack_runtime_trace_next(cursor, Some(8))?;
        let records = decode_trace_records(&context, records)?;
        assert_eq!(
            records,
            vec![(0, TraceEventKind::Control), (1, TraceEventKind::Marker)]
        );

        // seek back to the start and read the same record again
        context.destack_runtime_trace_seek_sequence(
            cursor,
            crate::platform::runtime::TraceSequence(0),
        )?;
        let tell = context.destack_runtime_trace_tell(cursor)?;
        assert_eq!(tell.0, 0);

        let replayed = context.destack_runtime_trace_next(cursor, Some(8))?;
        let replayed = decode_trace_records(&context, replayed)?;
        assert_eq!(
            replayed,
            vec![(0, TraceEventKind::Control), (1, TraceEventKind::Marker)]
        );

        // close the cursor and world
        context.destack_runtime_trace_close(cursor)?;
        context.destack_runtime_world_close(world)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_runtime_view_world_pins_one_revision() {
    with_harness_context(|mut context| {
        // create one low-level world and materialize one checkpointed revision
        let world_options = world_create_options(&mut context)?;
        let world = context.destack_runtime_world_create(world_options)?;
        let checkpoint_name = runtime_name(&mut context, "baseline");
        let checkpoint_labels = empty_runtime_labels(&mut context)?;
        let _checkpoint =
            context.destack_runtime_checkpoint_create(world, checkpoint_name, checkpoint_labels)?;
        let pinned_revision = context.destack_runtime_world_revision(world)?;

        // open one pinned view at the current revision
        let view_options = runtime_view_options(&mut context, Some(pinned_revision.0));
        let view = context.destack_runtime_world_view_open(world, view_options)?;
        let pinned_world =
            decode_trace_descriptor(context.destack_runtime_trace_describe(world)?).1;

        // advance the live world again so the current revision moves
        let _image_id = context.destack_runtime_image_capture(world)?;
        let live_revision = context.destack_runtime_world_revision(world)?;
        assert!(live_revision.0 > pinned_revision.0);

        // the pinned view should still report the original revision
        let view_world = context.destack_runtime_world_view(view)?;
        match view_world {
            HarnessValue::Native(descriptor) => {
                assert_eq!(descriptor.revision_id.0, pinned_revision.0);
                assert_eq!(descriptor.runtime_count, 0);
            }
            HarnessValue::Vm(descriptor) => {
                assert_eq!(descriptor.revision_id.0, pinned_revision.0);
                assert_eq!(descriptor.runtime_count, 0);
            }
        }

        // the pinned inspect views should resolve the same pinned revision backing
        let revision = decode_revision_descriptor(context.destack_runtime_revision_view(view)?);
        assert_eq!(revision.0, pinned_revision.0);

        let image = decode_image_descriptor(context.destack_runtime_image_view(view)?);
        assert_eq!(image.1, pinned_revision.0);

        let trace = decode_trace_descriptor(context.destack_runtime_trace_view(view)?);
        assert_eq!(trace.0, 0);
        assert_eq!(trace.1, revision.4);

        // the live trace has moved even though the view stayed pinned
        assert!(
            decode_trace_descriptor(context.destack_runtime_trace_describe(world)?).1
                >= pinned_world
        );

        // close the view and world
        context.destack_runtime_world_view_close(view)?;
        context.destack_runtime_world_close(world)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_runtime_view_lists_pinned_runtimes_and_agents() {
    with_harness_context(|mut context| {
        // create one world and attach one runtime with two agents
        let world_options = world_create_options(&mut context)?;
        let world = context.destack_runtime_world_create(world_options)?;
        let live_world = live_world(world)?;
        let runtime_id = live_world.spawn_runtime(
            Vec::new(),
            &RuntimeOptions::default(),
            inspect_vm_engine(),
        )?;
        let agent_id = live_world.spawn_agent(runtime_id, inspect_vm_engine())?;

        // materialize one revision that captures the new runtime state
        let _image_id = context.destack_runtime_image_capture(world)?;
        let pinned_revision = context.destack_runtime_world_revision(world)?;
        let view_options = runtime_view_options(&mut context, Some(pinned_revision.0));
        let view = context.destack_runtime_world_view_open(world, view_options)?;

        // runtime list and view
        let runtime_filter = runtime_list_filter(&mut context);
        let runtimes =
            context.destack_runtime_runtime_list(view, runtime_filter, None, Some(16))?;
        let runtimes = match runtimes {
            HarnessValue::Native(runtimes) => unsafe { runtimes.as_slice()? }
                .iter()
                .copied()
                .map(|descriptor| {
                    decode_runtime_descriptor(&context, HarnessValue::Native(descriptor))
                })
                .collect::<RuntimeResult<Vec<_>>>()?,
            HarnessValue::Vm(runtimes) => {
                let vm_context = unsafe {
                    &*(context.vm_context.expect("vm context") as *mut vm::ExternalCallContext<'_>)
                };
                let runtimes = runtimes.read_values(vm_context)?;
                runtimes
                    .into_iter()
                    .map(|descriptor| {
                        decode_runtime_descriptor(&context, HarnessValue::Vm(descriptor))
                    })
                    .collect::<RuntimeResult<Vec<_>>>()?
            }
        };
        assert_eq!(runtimes.len(), 1);
        assert_eq!(runtimes[0].0, runtime_id.0);
        assert!(runtimes[0].1 > 0);
        assert_eq!(runtimes[0].3, 2);

        let runtime = context.destack_runtime_runtime_view(view, RuntimeId(runtime_id.0))?;
        let runtime = decode_runtime_descriptor(&context, runtime)?;
        assert_eq!(runtime.0, runtime_id.0);
        assert_eq!(runtime.1, runtimes[0].1);
        assert_eq!(runtime.3, 2);

        // agent list and view
        let agent_filter = agent_list_filter(&mut context, Some(runtime_id.0));
        let agents = context.destack_runtime_agent_list(view, agent_filter, None, Some(16))?;
        let agents = match agents {
            HarnessValue::Native(agents) => unsafe { agents.as_slice()? }
                .iter()
                .copied()
                .map(|descriptor| {
                    decode_agent_descriptor(&context, HarnessValue::Native(descriptor))
                })
                .collect::<RuntimeResult<Vec<_>>>()?,
            HarnessValue::Vm(agents) => {
                let vm_context = unsafe {
                    &*(context.vm_context.expect("vm context") as *mut vm::ExternalCallContext<'_>)
                };
                let agents = agents.read_values(vm_context)?;
                agents
                    .into_iter()
                    .map(|descriptor| {
                        decode_agent_descriptor(&context, HarnessValue::Vm(descriptor))
                    })
                    .collect::<RuntimeResult<Vec<_>>>()?
            }
        };
        assert_eq!(agents.len(), 2);
        assert!(
            agents
                .iter()
                .any(|agent| agent.0 == runtime.1 && agent.1 == runtime_id.0)
        );
        assert!(
            agents
                .iter()
                .any(|agent| agent.0 == agent_id.0 && agent.1 == runtime_id.0)
        );
        assert!(agents.iter().all(|agent| !agent.3));

        let agent = context.destack_runtime_agent_view(view, AgentId(agent_id.0))?;
        let agent = decode_agent_descriptor(&context, agent)?;
        assert_eq!(agent.0, agent_id.0);
        assert_eq!(agent.1, runtime_id.0);
        assert!(!agent.3);
        assert_eq!(agent.4, 0);

        // event-loop, heap, and engine summaries
        let event_loop = context.destack_runtime_event_loop_view(view, AgentId(agent_id.0))?;
        match event_loop {
            HarnessValue::Native(descriptor) => {
                assert_eq!(descriptor.task_count, 0);
                assert_eq!(descriptor.microtask_count, 0);
                assert_eq!(descriptor.timer_count, 0);
                assert_eq!(descriptor.watch_count, 0);
                assert!(!descriptor.has_pending_work);
            }
            HarnessValue::Vm(descriptor) => {
                assert_eq!(descriptor.task_count, 0);
                assert_eq!(descriptor.microtask_count, 0);
                assert_eq!(descriptor.timer_count, 0);
                assert_eq!(descriptor.watch_count, 0);
                assert!(!descriptor.has_pending_work);
            }
        }

        let heap = context.destack_runtime_heap_view(view, AgentId(agent_id.0))?;
        match heap {
            HarnessValue::Native(descriptor) => {
                assert_eq!(descriptor.heap_bytes, 0);
                assert_eq!(descriptor.page_count, 2);
                assert_eq!(descriptor.shared_page_count, 2);
                assert_eq!(descriptor.gc_cycles, 0);
            }
            HarnessValue::Vm(descriptor) => {
                assert_eq!(descriptor.heap_bytes, 0);
                assert_eq!(descriptor.page_count, 2);
                assert_eq!(descriptor.shared_page_count, 2);
                assert_eq!(descriptor.gc_cycles, 0);
            }
        }

        let engine = context.destack_runtime_engine_view(view, AgentId(agent_id.0))?;
        match engine {
            HarnessValue::Native(descriptor) => {
                assert_eq!(descriptor.call_stack_depth, Some(0));
                assert_eq!(descriptor.value_stack_depth, Some(0));
                assert_eq!(descriptor.local_stack_depth, Some(0));
                assert!(
                    descriptor
                        .image_bytes
                        .is_some_and(|image_bytes| image_bytes > 0)
                );
            }
            HarnessValue::Vm(descriptor) => {
                assert_eq!(descriptor.call_stack_depth, Some(0));
                assert_eq!(descriptor.value_stack_depth, Some(0));
                assert_eq!(descriptor.local_stack_depth, Some(0));
                assert!(
                    descriptor
                        .image_bytes
                        .is_some_and(|image_bytes| image_bytes > 0)
                );
            }
        }

        // close the view and world
        context.destack_runtime_world_view_close(view)?;
        context.destack_runtime_world_close(world)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_runtime_lineage_lists_filter_pinned_world_state() {
    with_harness_context(|mut context| {
        // create one world and materialize multiple lineage records
        let world_options = world_create_options(&mut context)?;
        let world = context.destack_runtime_world_create(world_options)?;

        let baseline_name = runtime_name(&mut context, "baseline");
        let baseline_labels = empty_runtime_labels(&mut context)?;
        let baseline_checkpoint =
            context.destack_runtime_checkpoint_create(world, baseline_name, baseline_labels)?;
        let baseline_revision = context.destack_runtime_world_revision(world)?;

        let baseline_image = context.destack_runtime_image_capture(world)?;
        let baseline_image =
            decode_image_descriptor(context.destack_runtime_image_describe(world, baseline_image)?);

        let child_name = runtime_name(&mut context, "child");
        let child_labels = empty_runtime_labels(&mut context)?;
        let child = context.destack_runtime_world_fork(
            world,
            baseline_revision,
            child_name,
            child_labels,
        )?;
        let child_branch = context.destack_runtime_world_branch(child)?;

        // branch list filter
        let branch_filter = branch_list_filter(&mut context, Some("child"));
        let branches = context.destack_runtime_branch_list(world, branch_filter, None, Some(8))?;

        match branches {
            HarnessValue::Native(branches) => {
                let branches = unsafe { branches.as_slice()? };
                assert_eq!(branches.len(), 1);
                assert_eq!(branches[0].id.0, child_branch.0);
                assert_eq!(unsafe { branches[0].name.unwrap().as_str()? }, "child");
            }
            HarnessValue::Vm(branches) => {
                let vm_context = unsafe {
                    &*(context.vm_context.expect("vm context") as *mut vm::ExternalCallContext<'_>)
                };
                let branches = branches.read_values(vm_context)?;
                assert_eq!(branches.len(), 1);
                assert_eq!(branches[0].id.0, child_branch.0);
                let name = vm_context
                    .string_ref(branches[0].name.unwrap())
                    .map_err(Box::<crate::diagnostic::RuntimeError>::from)?;
                assert_eq!(name.as_str(), "child");
            }
        }

        // revision list filter
        let revision_filter = revision_list_filter(&mut context, Some(0));
        let revisions =
            context.destack_runtime_revision_list(world, revision_filter, None, Some(16))?;

        match revisions {
            HarnessValue::Native(revisions) => {
                let revisions = unsafe { revisions.as_slice()? };
                assert!(
                    revisions
                        .iter()
                        .any(|revision| revision.id.0 == baseline_revision.0
                            && revision.branch_id.0 == 0)
                );
            }
            HarnessValue::Vm(revisions) => {
                let vm_context = unsafe {
                    &*(context.vm_context.expect("vm context") as *mut vm::ExternalCallContext<'_>)
                };
                let revisions = revisions.read_values(vm_context)?;
                assert!(
                    revisions
                        .iter()
                        .any(|revision| revision.id.0 == baseline_revision.0
                            && revision.branch_id.0 == 0)
                );
            }
        }

        // checkpoint list filter
        let checkpoint_filter =
            checkpoint_list_filter(&mut context, Some(baseline_revision.0), Some("baseline"));
        let checkpoints =
            context.destack_runtime_checkpoint_list(world, checkpoint_filter, None, Some(8))?;

        match checkpoints {
            HarnessValue::Native(checkpoints) => {
                let checkpoints = unsafe { checkpoints.as_slice()? };
                assert_eq!(checkpoints.len(), 1);
                assert_eq!(checkpoints[0].id.0, baseline_checkpoint.0);
                assert_eq!(checkpoints[0].revision_id.0, baseline_revision.0);
                assert_eq!(
                    unsafe { checkpoints[0].name.unwrap().as_str()? },
                    "baseline"
                );
            }
            HarnessValue::Vm(checkpoints) => {
                let vm_context = unsafe {
                    &*(context.vm_context.expect("vm context") as *mut vm::ExternalCallContext<'_>)
                };
                let checkpoints = checkpoints.read_values(vm_context)?;
                assert_eq!(checkpoints.len(), 1);
                assert_eq!(checkpoints[0].id.0, baseline_checkpoint.0);
                assert_eq!(checkpoints[0].revision_id.0, baseline_revision.0);
                let name = vm_context
                    .string_ref(checkpoints[0].name.unwrap())
                    .map_err(Box::<crate::diagnostic::RuntimeError>::from)?;
                assert_eq!(name.as_str(), "baseline");
            }
        }

        // image list filter
        let image_filter = image_list_filter(&mut context, Some(baseline_image.1));
        let images = context.destack_runtime_image_list(world, image_filter, None, Some(8))?;

        match images {
            HarnessValue::Native(images) => {
                let images = unsafe { images.as_slice()? };
                assert_eq!(images.len(), 1);
                assert_eq!(images[0].id.0, baseline_image.0);
                assert_eq!(images[0].revision_id.0, baseline_image.1);
            }
            HarnessValue::Vm(images) => {
                let vm_context = unsafe {
                    &*(context.vm_context.expect("vm context") as *mut vm::ExternalCallContext<'_>)
                };
                let images = images.read_values(vm_context)?;
                assert_eq!(images.len(), 1);
                assert_eq!(images[0].id.0, baseline_image.0);
                assert_eq!(images[0].revision_id.0, baseline_image.1);
            }
        }

        // close the child and parent worlds
        context.destack_runtime_world_close(child)?;
        context.destack_runtime_world_close(world)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_runtime_view_lists_pinned_topology_and_resources() {
    with_harness_context(|mut context| {
        // create one world with one extra runtime agent and one logical resource
        let world_options = world_create_options(&mut context)?;
        let world = context.destack_runtime_world_create(world_options)?;
        let live_world = live_world(world)?;
        let runtime_id = live_world.spawn_runtime(
            Vec::new(),
            &RuntimeOptions::default(),
            inspect_vm_engine(),
        )?;
        let agent_id = live_world.spawn_agent(runtime_id, inspect_vm_engine())?;

        let resource_id = LogicalWorldResourceId::new(agent_id, ResourceId(1));
        let resource = WorldResource::new(
            resource_id,
            WorldEntityKind::from("resource.timer"),
            Some("test-timer".to_string()),
            ResourceBacking::Host,
            ResourceCapture::None,
            ResourcePortability::Local,
        );
        live_world.create_resource(resource)?;

        // materialize one pinned image containing the topology/resource state
        let _image_id = context.destack_runtime_image_capture(world)?;
        let pinned_revision = context.destack_runtime_world_revision(world)?;
        let view_options = runtime_view_options(&mut context, Some(pinned_revision.0));
        let view = context.destack_runtime_world_view_open(world, view_options)?;

        let runtime_resource_id = world_resource_id(&mut context, agent_id.0, 1);
        let resource_entity_id_value = format!("resource.{}.1", agent_id.0);
        let resource_entity_id = topology_entity_id(&mut context, &resource_entity_id_value);
        let ownership_edge_id_value = format!("agent.{}.owns.resource.1", agent_id.0);
        let ownership_edge_id = topology_edge_id(&mut context, &ownership_edge_id_value);

        // resource list and view
        let resource_filter = resource_list_filter(&mut context, Some(agent_id.0));
        let resource_after = no_resource_after(&mut context);
        let resources = context.destack_runtime_resource_list(
            view,
            resource_filter,
            resource_after,
            Some(8),
        )?;

        match resources {
            HarnessValue::Native(resources) => {
                let resources = unsafe { resources.as_slice()? };
                assert_eq!(resources.len(), 1);
                assert_eq!(resources[0].id.resource_id.0, 1);
                assert_eq!(resources[0].id.agent_id.0, agent_id.0);
                assert_eq!(
                    unsafe { resources[0].entity_id.0.as_str()? },
                    resource_entity_id_value
                );
                assert_eq!(unsafe { resources[0].kind.as_str()? }, "resource.timer");
                assert_eq!(
                    unsafe { resources[0].label.unwrap().as_str()? },
                    "test-timer"
                );
            }
            HarnessValue::Vm(resources) => {
                let vm_context = unsafe {
                    &*(context.vm_context.expect("vm context") as *mut vm::ExternalCallContext<'_>)
                };
                let resources = resources.read_values(vm_context)?;
                assert_eq!(resources.len(), 1);
                assert_eq!(resources[0].id.resource_id.0, 1);
                assert_eq!(resources[0].id.agent_id.0, agent_id.0);
                let entity_id = vm_context
                    .string_ref(resources[0].entity_id.0)
                    .map_err(Box::<crate::diagnostic::RuntimeError>::from)?;
                let kind = vm_context
                    .string_ref(resources[0].kind)
                    .map_err(Box::<crate::diagnostic::RuntimeError>::from)?;
                let label = vm_context
                    .string_ref(resources[0].label.unwrap())
                    .map_err(Box::<crate::diagnostic::RuntimeError>::from)?;
                assert_eq!(entity_id.as_str(), resource_entity_id_value);
                assert_eq!(kind.as_str(), "resource.timer");
                assert_eq!(label.as_str(), "test-timer");
            }
        }

        let resource = context.destack_runtime_resource_view(view, runtime_resource_id)?;
        match resource {
            HarnessValue::Native(resource) => {
                assert_eq!(resource.id.resource_id.0, 1);
                assert_eq!(
                    unsafe { resource.entity_id.0.as_str()? },
                    resource_entity_id_value
                );
            }
            HarnessValue::Vm(resource) => {
                let vm_context = unsafe {
                    &*(context.vm_context.expect("vm context") as *mut vm::ExternalCallContext<'_>)
                };
                let entity_id = vm_context
                    .string_ref(resource.entity_id.0)
                    .map_err(Box::<crate::diagnostic::RuntimeError>::from)?;
                assert_eq!(resource.id.resource_id.0, 1);
                assert_eq!(entity_id.as_str(), resource_entity_id_value);
            }
        }

        // entity list and view
        let entity_kind = topology_entity_kind(&mut context, "resource.timer");
        let entity_filter = match entity_kind {
            HarnessValue::Native(kind) => context.harness_value(Some(TopologyEntityFilter {
                kind: Some(kind),
                labels: None,
            })),
            HarnessValue::Vm(kind) => context.harness_value_vm(Some(TopologyEntityFilterVm {
                kind: Some(kind),
                labels: None,
            })),
        };
        let entity_after = no_entity_after(&mut context);
        let entities =
            context.destack_runtime_entity_list(view, entity_filter, entity_after, Some(8))?;

        match entities {
            HarnessValue::Native(entities) => {
                let entities = unsafe { entities.as_slice()? };
                assert_eq!(entities.len(), 1);
                assert_eq!(
                    unsafe { entities[0].id.0.as_str()? },
                    resource_entity_id_value
                );
                assert_eq!(unsafe { entities[0].kind.0.as_str()? }, "resource.timer");
            }
            HarnessValue::Vm(entities) => {
                let vm_context = unsafe {
                    &*(context.vm_context.expect("vm context") as *mut vm::ExternalCallContext<'_>)
                };
                let entities = entities.read_values(vm_context)?;
                assert_eq!(entities.len(), 1);
                let id = vm_context
                    .string_ref(entities[0].id.0)
                    .map_err(Box::<crate::diagnostic::RuntimeError>::from)?;
                let kind = vm_context
                    .string_ref(entities[0].kind.0)
                    .map_err(Box::<crate::diagnostic::RuntimeError>::from)?;
                assert_eq!(id.as_str(), resource_entity_id_value);
                assert_eq!(kind.as_str(), "resource.timer");
            }
        }

        let entity = context.destack_runtime_entity_view(view, resource_entity_id)?;
        match entity {
            HarnessValue::Native(entity) => {
                assert_eq!(unsafe { entity.id.0.as_str()? }, resource_entity_id_value);
                assert_eq!(unsafe { entity.kind.0.as_str()? }, "resource.timer");
            }
            HarnessValue::Vm(entity) => {
                let vm_context = unsafe {
                    &*(context.vm_context.expect("vm context") as *mut vm::ExternalCallContext<'_>)
                };
                let id = vm_context
                    .string_ref(entity.id.0)
                    .map_err(Box::<crate::diagnostic::RuntimeError>::from)?;
                let kind = vm_context
                    .string_ref(entity.kind.0)
                    .map_err(Box::<crate::diagnostic::RuntimeError>::from)?;
                assert_eq!(id.as_str(), resource_entity_id_value);
                assert_eq!(kind.as_str(), "resource.timer");
            }
        }

        // edge list and view
        let from_entity_id_value = format!("agent.{}", agent_id.0);
        let edge_kind = topology_edge_kind(&mut context, "runtime.agent.owns.resource");
        let from_entity_id = topology_entity_id(&mut context, &from_entity_id_value);
        let to_entity_id = topology_entity_id(&mut context, &resource_entity_id_value);
        let edge_filter = match (edge_kind, from_entity_id, to_entity_id) {
            (HarnessValue::Native(kind), HarnessValue::Native(from), HarnessValue::Native(to)) => {
                context.harness_value(Some(TopologyEdgeFilter {
                    kind: Some(kind),
                    from: Some(from),
                    to: Some(to),
                    labels: None,
                }))
            }
            (HarnessValue::Vm(kind), HarnessValue::Vm(from), HarnessValue::Vm(to)) => context
                .harness_value_vm(Some(TopologyEdgeFilterVm {
                    kind: Some(kind),
                    from: Some(from),
                    to: Some(to),
                    labels: None,
                })),
            _ => unreachable!("harness values must stay backend-aligned"),
        };
        let edge_after = no_edge_after(&mut context);
        let edges = context.destack_runtime_edge_list(view, edge_filter, edge_after, Some(8))?;

        match edges {
            HarnessValue::Native(edges) => {
                let edges = unsafe { edges.as_slice()? };
                assert_eq!(edges.len(), 1);
                assert_eq!(unsafe { edges[0].id.0.as_str()? }, ownership_edge_id_value);
                assert_eq!(
                    unsafe { edges[0].kind.0.as_str()? },
                    "runtime.agent.owns.resource"
                );
                assert_eq!(unsafe { edges[0].from.0.as_str()? }, from_entity_id_value);
                assert_eq!(unsafe { edges[0].to.0.as_str()? }, resource_entity_id_value);
            }
            HarnessValue::Vm(edges) => {
                let vm_context = unsafe {
                    &*(context.vm_context.expect("vm context") as *mut vm::ExternalCallContext<'_>)
                };
                let edges = edges.read_values(vm_context)?;
                assert_eq!(edges.len(), 1);
                let id = vm_context
                    .string_ref(edges[0].id.0)
                    .map_err(Box::<crate::diagnostic::RuntimeError>::from)?;
                let kind = vm_context
                    .string_ref(edges[0].kind.0)
                    .map_err(Box::<crate::diagnostic::RuntimeError>::from)?;
                let from = vm_context
                    .string_ref(edges[0].from.0)
                    .map_err(Box::<crate::diagnostic::RuntimeError>::from)?;
                let to = vm_context
                    .string_ref(edges[0].to.0)
                    .map_err(Box::<crate::diagnostic::RuntimeError>::from)?;
                assert_eq!(id.as_str(), ownership_edge_id_value);
                assert_eq!(kind.as_str(), "runtime.agent.owns.resource");
                assert_eq!(from.as_str(), from_entity_id_value);
                assert_eq!(to.as_str(), resource_entity_id_value);
            }
        }

        let edge = context.destack_runtime_edge_view(view, ownership_edge_id)?;
        match edge {
            HarnessValue::Native(edge) => {
                assert_eq!(unsafe { edge.id.0.as_str()? }, ownership_edge_id_value);
            }
            HarnessValue::Vm(edge) => {
                let vm_context = unsafe {
                    &*(context.vm_context.expect("vm context") as *mut vm::ExternalCallContext<'_>)
                };
                let id = vm_context
                    .string_ref(edge.id.0)
                    .map_err(Box::<crate::diagnostic::RuntimeError>::from)?;
                assert_eq!(id.as_str(), ownership_edge_id_value);
            }
        }

        // close the view and world
        context.destack_runtime_world_view_close(view)?;
        context.destack_runtime_world_close(world)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_runtime_snapshot_roundtrip_controls_world_state() {
    with_harness_context(|mut context| {
        // create one world with one stable captured image
        let world_options = world_create_options(&mut context)?;
        let world = context.destack_runtime_world_create(world_options)?;

        let checkpoint_name = runtime_name(&mut context, "baseline");
        let checkpoint_labels = empty_runtime_labels(&mut context)?;
        let _checkpoint =
            context.destack_runtime_checkpoint_create(world, checkpoint_name, checkpoint_labels)?;

        let image_id = context.destack_runtime_image_capture(world)?;
        let image =
            decode_image_descriptor(context.destack_runtime_image_describe(world, image_id)?);
        let baseline_revision = context.destack_runtime_world_revision(world)?;
        assert_eq!(image.1, baseline_revision.0);

        // export one snapshot and inspect its metadata
        let snapshot_id =
            context.destack_runtime_snapshot_create(world, image_id, SnapshotFormat::Portable)?;
        let snapshot = decode_snapshot_descriptor(
            context.destack_runtime_snapshot_describe(world, snapshot_id)?,
        );
        assert_eq!(snapshot.0, snapshot_id.0);
        assert_eq!(snapshot.1, image_id.0);
        assert_eq!(snapshot.2, SnapshotFormat::Portable);
        assert!(snapshot.3.is_some_and(|size_bytes| size_bytes > 0));

        let snapshots = context.destack_runtime_snapshot_list(world, None, Some(8))?;

        // the exported snapshot should be visible in the lineage list
        match snapshots {
            HarnessValue::Native(snapshots) => {
                let snapshots = unsafe { snapshots.as_slice()? };
                assert_eq!(snapshots.len(), 1);
                assert_eq!(snapshots[0].id.0, snapshot_id.0);
            }
            HarnessValue::Vm(snapshots) => {
                let vm_context = unsafe {
                    &*(context.vm_context.expect("vm context") as *mut vm::ExternalCallContext<'_>)
                };
                let snapshots = snapshots.read_values(vm_context)?;
                assert_eq!(snapshots.len(), 1);
                assert_eq!(snapshots[0].id.0, snapshot_id.0);
            }
        }

        // read and reimport the serialized snapshot payload
        let snapshot_bytes_value = context.destack_runtime_snapshot_read(world, snapshot_id)?;
        let snapshot_bytes = decode_snapshot_bytes(&context, snapshot_bytes_value)?;
        assert!(!snapshot_bytes.is_empty());
        let _decoded_snapshot = WorldSnapshot::decode(&snapshot_bytes)?;

        let payload = snapshot_payload(&mut context, &snapshot_bytes);
        let imported_snapshot = context.destack_runtime_snapshot_import(world, payload)?;
        let imported = decode_snapshot_descriptor(
            context.destack_runtime_snapshot_describe(world, imported_snapshot)?,
        );
        assert_eq!(imported.1, image_id.0);
        assert_eq!(imported.2, SnapshotFormat::Portable);

        let imported_payload = context.destack_runtime_snapshot_read(world, imported_snapshot)?;
        let imported_bytes = decode_snapshot_bytes(&context, imported_payload)?;
        assert_eq!(imported_bytes, snapshot_bytes);

        // advance the live world, then restore the earlier image and snapshot
        let later_name = runtime_name(&mut context, "later");
        let later_labels = empty_runtime_labels(&mut context)?;
        let _later_checkpoint =
            context.destack_runtime_checkpoint_create(world, later_name, later_labels)?;
        let later_revision = context.destack_runtime_world_revision(world)?;
        assert!(later_revision.0 > baseline_revision.0);

        context.destack_runtime_restore_image(world, image_id)?;
        let restored_from_image = context.destack_runtime_world_revision(world)?;
        assert_eq!(restored_from_image.0, baseline_revision.0);

        let latest_name = runtime_name(&mut context, "latest");
        let latest_labels = empty_runtime_labels(&mut context)?;
        let _latest_checkpoint =
            context.destack_runtime_checkpoint_create(world, latest_name, latest_labels)?;
        let latest_revision = context.destack_runtime_world_revision(world)?;
        assert!(latest_revision.0 > baseline_revision.0);

        context.destack_runtime_restore_snapshot(world, imported_snapshot)?;
        let restored_from_snapshot = context.destack_runtime_world_revision(world)?;
        assert_eq!(restored_from_snapshot.0, baseline_revision.0);

        // close the live world
        context.destack_runtime_world_close(world)?;

        Ok(())
    });
}
