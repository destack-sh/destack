use std::collections::BTreeMap;
use std::sync::Arc;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::io_not_found;
use crate::platform::runtime::abi_generated::{
    TopologyEdgeIdAbi, TopologyEdgeKindAbi, TopologyEntityIdAbi, TopologyEntityKindAbi,
};
use crate::platform::runtime::{
    AgentCreateOptionsVm, AgentDescriptorVm, AgentFilterVm, AgentHandle, AgentId,
    BranchDescriptorVm, BranchFilterVm, BranchId, CheckpointDescriptorVm, CheckpointFilterVm,
    CheckpointId, EngineDescriptorKind, EngineDescriptorVm, EventLoopDescriptorVm,
    HeapDescriptorVm, ImageDescriptorVm, ImageFilterVm, ImageId, ObservationEventKind,
    ObservationHandle, ObservationOptionsVm, ObservationRecordVm, ResourceDescriptorVm,
    ResourceFilterVm, RevisionDescriptorVm, RevisionFilterVm, RevisionId, RuntimeCreateOptionsVm,
    RuntimeDescriptorVm, RuntimeExecutionMode, RuntimeFilterVm, RuntimeHandle, RuntimeId,
    RuntimeLabelSelectorVm, RuntimeLabelVm, RuntimeTickOutcome, RuntimeWorldKind,
    SnapshotDescriptorVm, SnapshotFormat, SnapshotId, TopologyEdgeFilterVm, TopologyEdgeIdVm,
    TopologyEdgeKindVm, TopologyEdgeVm, TopologyEntityFilterVm, TopologyEntityIdVm,
    TopologyEntityKindVm, TopologyEntityVm, TraceCursorHandle, TraceCursorOptionsVm,
    TraceDescriptorVm, TraceRecordVm, TraceSequence, WorldCreateOptionsVm, WorldDescriptorVm,
    WorldHandle, WorldResourceIdVm, WorldViewHandle, WorldViewOptionsVm,
};
use crate::platform::{PlatformError, VmArray};
use crate::runtime::control::inspect::{
    AgentListFilter, BranchListFilter, CheckpointListFilter, EdgeListFilter, EntityListFilter,
    ImageListFilter, ResourceListFilter, RevisionListFilter, RuntimeListFilter, agent_in_image,
    edge_in_image, entity_in_image, labels_match_selectors, list_agents, list_edges, list_entities,
    list_resources, list_runtimes, resource_in_image, runtime_in_image,
};
use crate::runtime::control::{control_table, empty_vm_engine};
use crate::runtime::engine::EngineImage;
use crate::runtime::replay::{TraceEvent, TraceSequence as RuntimeTraceSequence};
use crate::runtime::scheduler::EventLoopSnapshot;
use crate::runtime::world::{
    Branch as WorldBranch, Checkpoint as WorldCheckpoint, Image as WorldImage, ObservationKind,
    ObservationOptions, ObservationRecord as WorldObservationRecord, Revision as WorldRevision,
    Snapshot as WorldSnapshot, World, WorldEdge, WorldEntity,
    WorldResource as LogicalWorldResource, WorldResourceId as LogicalWorldResourceId,
};
use crate::runtime::{
    Agent as LiveAgent, AgentImage as CapturedAgent, BindingCallContext, Runtime as LiveRuntime,
    RuntimeImage as CapturedRuntime, TickOutcome,
};
use destack_vm as vm;
use destack_workspace::{ExecutionMode, RuntimeOptions, RuntimeWorld, TimeMode};
use postcard::to_allocvec;

use super::handle::{
    self as handle, SnapshotHandleEntry, decode_agent_id, decode_branch_id, decode_checkpoint_id,
    decode_image_id, decode_revision_id, decode_runtime_id, encode_agent_id, encode_branch_id,
    encode_checkpoint_id, encode_image_id, encode_revision_id, encode_runtime_id,
    encode_trace_event_kind,
};

fn runtime_options_from_world_create(options: WorldCreateOptionsVm) -> RuntimeOptions {
    let mut runtime_options = RuntimeOptions::default();

    // optional overrides
    if let Some(execution) = options.execution {
        runtime_options.execution = match execution {
            RuntimeExecutionMode::Fast => ExecutionMode::Fast,
            RuntimeExecutionMode::Deterministic => ExecutionMode::Deterministic,
            RuntimeExecutionMode::Record => ExecutionMode::Record,
            RuntimeExecutionMode::Replay => ExecutionMode::Replay,
        };
    }

    if let Some(world) = options.world {
        runtime_options.world = match world {
            RuntimeWorldKind::Host => RuntimeWorld::Host,
            RuntimeWorldKind::Simulation => RuntimeWorld::Simulation,
        };
    }

    // simulation worlds default to virtual time
    if runtime_options.world == RuntimeWorld::Simulation {
        runtime_options.time.mode = TimeMode::Virtual;
    }

    runtime_options
}

fn observation_options_from_abi(options: ObservationOptionsVm) -> ObservationOptions {
    ObservationOptions {
        trace: options.trace.unwrap_or(false),
        topology: options.topology.unwrap_or(false),
        resource: options.resources.unwrap_or(false),
        scheduler: options.scheduler.unwrap_or(false),
        diagnostic: options.diagnostics.unwrap_or(false),
        profile: options.profiles.unwrap_or(false),
    }
}

fn decode_runtime_labels(
    context: &vm::ExternalCallContext<'_>,
    labels: Option<VmArray<RuntimeLabelVm>>,
) -> RuntimeResult<BTreeMap<String, String>> {
    // absent labels
    let Some(labels) = labels else {
        return Ok(BTreeMap::new());
    };

    let labels = labels.read_values(context)?;
    let mut decoded = BTreeMap::new();

    for label in labels {
        let key = context
            .string_ref(label.key)
            .map_err(Box::<RuntimeError>::from)?
            .to_string();
        let value = context
            .string_ref(label.value)
            .map_err(Box::<RuntimeError>::from)?
            .to_string();
        decoded.insert(key, value);
    }

    Ok(decoded)
}

fn decode_label_selectors(
    context: &vm::ExternalCallContext<'_>,
    selectors: Option<VmArray<RuntimeLabelSelectorVm>>,
) -> RuntimeResult<Vec<(String, Option<String>)>> {
    let Some(selectors) = selectors else {
        return Ok(Vec::new());
    };

    let selectors = selectors.read_values(context)?;
    let mut decoded = Vec::with_capacity(selectors.len());

    for selector in selectors {
        let key = context
            .string_ref(selector.key)
            .map_err(Box::<RuntimeError>::from)?
            .as_str()
            .to_string();
        let value = match selector.value {
            Some(value) => Some(
                context
                    .string_ref(value)
                    .map_err(Box::<RuntimeError>::from)?
                    .as_str()
                    .to_string(),
            ),
            None => None,
        };
        decoded.push((key, value));
    }

    Ok(decoded)
}

fn encode_optional_runtime_name(
    context: &mut vm::ExternalCallContext<'_>,
    name: &str,
) -> Option<vm::StringHandle> {
    if name.is_empty() {
        return None;
    }

    Some(vm::StringHandle::new(context.intern_string(name)))
}

fn encode_runtime_labels(
    context: &mut vm::ExternalCallContext<'_>,
    labels: BTreeMap<String, String>,
) -> RuntimeResult<VmArray<RuntimeLabelVm>> {
    let labels = labels
        .into_iter()
        .map(|(key, value)| RuntimeLabelVm {
            key: vm::StringHandle::new(context.intern_string(&key)),
            value: vm::StringHandle::new(context.intern_string(&value)),
        })
        .collect::<Vec<_>>();

    VmArray::from_values(context, &labels)
}

fn encode_optional_runtime_labels(
    context: &mut vm::ExternalCallContext<'_>,
    labels: BTreeMap<String, String>,
) -> RuntimeResult<Option<VmArray<RuntimeLabelVm>>> {
    if labels.is_empty() {
        return Ok(None);
    }

    Ok(Some(encode_runtime_labels(context, labels)?))
}

fn decode_runtime_name(
    context: &vm::ExternalCallContext<'_>,
    name: Option<vm::StringHandle>,
) -> RuntimeResult<Option<String>> {
    let Some(name) = name else {
        return Ok(None);
    };

    Ok(Some(
        context
            .string_ref(name)
            .map_err(Box::<RuntimeError>::from)?
            .as_str()
            .to_string(),
    ))
}

fn decode_runtime_filter(
    context: &vm::ExternalCallContext<'_>,
    filter: Option<RuntimeFilterVm>,
) -> RuntimeResult<RuntimeListFilter> {
    let Some(filter) = filter else {
        return Ok(RuntimeListFilter::default());
    };

    Ok(RuntimeListFilter {
        name: decode_runtime_name(context, filter.name)?,
        labels: decode_label_selectors(context, filter.labels)?,
    })
}

fn decode_agent_filter(
    context: &vm::ExternalCallContext<'_>,
    filter: Option<AgentFilterVm>,
) -> RuntimeResult<AgentListFilter> {
    let Some(filter) = filter else {
        return Ok(AgentListFilter::default());
    };

    Ok(AgentListFilter {
        runtime_id: filter.runtime_id.map(decode_runtime_id),
        name: decode_runtime_name(context, filter.name)?,
        has_pending_work: filter.has_pending_work,
        labels: decode_label_selectors(context, filter.labels)?,
    })
}

fn decode_branch_filter(
    context: &vm::ExternalCallContext<'_>,
    filter: Option<BranchFilterVm>,
) -> RuntimeResult<BranchListFilter> {
    let Some(filter) = filter else {
        return Ok(BranchListFilter::default());
    };

    Ok(BranchListFilter {
        name: decode_runtime_name(context, filter.name)?,
        labels: decode_label_selectors(context, filter.labels)?,
    })
}

fn decode_revision_filter(filter: Option<RevisionFilterVm>) -> RevisionListFilter {
    let Some(filter) = filter else {
        return RevisionListFilter::default();
    };

    RevisionListFilter {
        branch_id: filter.branch_id.map(decode_branch_id),
    }
}

fn decode_checkpoint_filter(
    context: &vm::ExternalCallContext<'_>,
    filter: Option<CheckpointFilterVm>,
) -> RuntimeResult<CheckpointListFilter> {
    let Some(filter) = filter else {
        return Ok(CheckpointListFilter::default());
    };

    Ok(CheckpointListFilter {
        revision_id: filter.revision_id.map(decode_revision_id),
        name: decode_runtime_name(context, filter.name)?,
        labels: decode_label_selectors(context, filter.labels)?,
    })
}

fn decode_image_filter(filter: Option<ImageFilterVm>) -> ImageListFilter {
    let Some(filter) = filter else {
        return ImageListFilter::default();
    };

    ImageListFilter {
        revision_id: filter.revision_id.map(decode_revision_id),
    }
}

fn decode_resource_filter(
    context: &vm::ExternalCallContext<'_>,
    filter: Option<ResourceFilterVm>,
) -> RuntimeResult<ResourceListFilter> {
    let Some(filter) = filter else {
        return Ok(ResourceListFilter::default());
    };

    Ok(ResourceListFilter {
        runtime_id: filter.runtime_id.map(decode_runtime_id),
        agent_id: filter.agent_id.map(decode_agent_id),
        kind: decode_runtime_name(context, filter.kind)?,
        label: decode_runtime_name(context, filter.label)?,
    })
}

fn decode_entity_filter(
    context: &vm::ExternalCallContext<'_>,
    filter: Option<TopologyEntityFilterVm>,
) -> RuntimeResult<EntityListFilter> {
    let Some(filter) = filter else {
        return Ok(EntityListFilter::default());
    };

    Ok(EntityListFilter {
        kind: decode_topology_entity_kind(context, filter.kind)?,
        labels: decode_label_selectors(context, filter.labels)?,
    })
}

fn decode_edge_filter(
    context: &vm::ExternalCallContext<'_>,
    filter: Option<TopologyEdgeFilterVm>,
) -> RuntimeResult<EdgeListFilter> {
    let Some(filter) = filter else {
        return Ok(EdgeListFilter::default());
    };

    Ok(EdgeListFilter {
        kind: decode_topology_edge_kind(context, filter.kind)?,
        from: decode_topology_entity_id_value(context, filter.from)?,
        to: decode_topology_entity_id_value(context, filter.to)?,
        labels: decode_label_selectors(context, filter.labels)?,
    })
}

fn decode_topology_entity_id_value(
    context: &vm::ExternalCallContext<'_>,
    value: Option<TopologyEntityIdVm>,
) -> RuntimeResult<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };

    Ok(Some(
        context
            .string_ref(value.0)
            .map_err(Box::<RuntimeError>::from)?
            .as_str()
            .to_string(),
    ))
}

fn decode_topology_edge_id_value(
    context: &vm::ExternalCallContext<'_>,
    value: Option<TopologyEdgeIdVm>,
) -> RuntimeResult<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };

    Ok(Some(
        context
            .string_ref(value.0)
            .map_err(Box::<RuntimeError>::from)?
            .as_str()
            .to_string(),
    ))
}

fn decode_topology_entity_kind(
    context: &vm::ExternalCallContext<'_>,
    value: Option<TopologyEntityKindVm>,
) -> RuntimeResult<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };

    Ok(Some(
        context
            .string_ref(value.0)
            .map_err(Box::<RuntimeError>::from)?
            .as_str()
            .to_string(),
    ))
}

fn decode_topology_edge_kind(
    context: &vm::ExternalCallContext<'_>,
    value: Option<TopologyEdgeKindVm>,
) -> RuntimeResult<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };

    Ok(Some(
        context
            .string_ref(value.0)
            .map_err(Box::<RuntimeError>::from)?
            .as_str()
            .to_string(),
    ))
}

fn encode_topology_entity_id(
    context: &mut vm::ExternalCallContext<'_>,
    entity_id: &str,
) -> TopologyEntityIdVm {
    TopologyEntityIdAbi(vm::StringHandle::new(context.intern_string(entity_id)))
}

fn encode_topology_edge_id(
    context: &mut vm::ExternalCallContext<'_>,
    edge_id: &str,
) -> TopologyEdgeIdVm {
    TopologyEdgeIdAbi(vm::StringHandle::new(context.intern_string(edge_id)))
}

fn encode_world_resource_id(resource_id: LogicalWorldResourceId) -> WorldResourceIdVm {
    WorldResourceIdVm {
        agent_id: AgentId(resource_id.agent_id.0),
        resource_id: resource_id.resource_id,
    }
}

fn encode_branch_descriptor(
    context: &mut vm::ExternalCallContext<'_>,
    branch: WorldBranch,
) -> RuntimeResult<BranchDescriptorVm> {
    Ok(BranchDescriptorVm {
        id: encode_branch_id(branch.id)?,
        head_revision: encode_revision_id(branch.head_revision_id)?,
        name: encode_optional_runtime_name(context, &branch.name),
        labels: encode_optional_runtime_labels(context, branch.labels)?,
    })
}

fn encode_revision_descriptor(
    revision: WorldRevision,
    image: &WorldImage,
) -> RuntimeResult<RevisionDescriptorVm> {
    Ok(RevisionDescriptorVm {
        id: encode_revision_id(revision.id)?,
        branch_id: encode_branch_id(revision.branch_id)?,
        parent_revision: revision
            .parent_revision_id
            .map(encode_revision_id)
            .transpose()?,
        sequence: TraceSequence(u64::try_from(revision.sequence.get()).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "sequence",
                "world trace sequence exceeds uint64",
            ))
            .boxed()
        })?),
        wall_ns: revision.wall.get(),
        mono_ns: revision.mono.get(),
        virtual_ns: image.clock.virtual_wall.get(),
        image_id: encode_image_id(revision.image_id)?,
    })
}

fn checked_count(value: usize, field: &'static str, detail: &'static str) -> RuntimeResult<u32> {
    u32::try_from(value).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(field, detail)).boxed()
    })
}

fn count_shared_items<T>(items: &[std::sync::Arc<T>]) -> RuntimeResult<u32> {
    let shared_item_count = items
        .iter()
        .filter(|item| std::sync::Arc::strong_count(item) > 1)
        .count();

    checked_count(
        shared_item_count,
        "sharedPageCount",
        "heap shared page count exceeds uint32",
    )
}

fn encode_runtime_descriptor_for_image(
    context: &mut vm::ExternalCallContext<'_>,
    image: &WorldImage,
    runtime: &CapturedRuntime,
) -> RuntimeResult<RuntimeDescriptorVm> {
    let labels = image.runtime_labels(runtime.runtime_id)?;
    let agent_count = image
        .agents
        .values()
        .filter(|agent| agent.runtime_id == runtime.runtime_id)
        .count();
    let agent_count = u32::try_from(agent_count).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "agentCount",
            "runtime agent count exceeds uint32",
        ))
        .boxed()
    })?;

    Ok(RuntimeDescriptorVm {
        id: encode_runtime_id(runtime.runtime_id)?,
        primary_agent_id: encode_agent_id(runtime.primary_agent_id)?,
        name: encode_optional_runtime_name(context, &runtime.name),
        agent_count,
        labels: encode_optional_runtime_labels(context, labels.clone())?,
    })
}

fn encode_agent_descriptor_for_image(
    context: &mut vm::ExternalCallContext<'_>,
    image: &WorldImage,
    agent: &CapturedAgent,
) -> RuntimeResult<AgentDescriptorVm> {
    let labels = image.agent_labels(agent.agent_id)?;
    let resource_count = u32::try_from(agent.resources.entries.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "resourceCount",
            "agent resource count exceeds uint32",
        ))
        .boxed()
    })?;

    Ok(AgentDescriptorVm {
        id: encode_agent_id(agent.agent_id)?,
        runtime_id: encode_runtime_id(agent.runtime_id)?,
        name: encode_optional_runtime_name(context, &agent.name),
        has_pending_work: agent.has_pending_work(),
        resource_count,
        labels: encode_optional_runtime_labels(context, labels.clone())?,
    })
}

fn encode_runtime_descriptor_for_live(
    context: &mut vm::ExternalCallContext<'_>,
    world: &World,
    runtime: &LiveRuntime,
) -> RuntimeResult<RuntimeDescriptorVm> {
    let labels = world.runtime_labels(runtime.runtime_id())?;
    let agent_count = checked_count(
        runtime.agent_count(),
        "agentCount",
        "runtime agent count exceeds uint32",
    )?;

    Ok(RuntimeDescriptorVm {
        id: encode_runtime_id(runtime.runtime_id())?,
        primary_agent_id: encode_agent_id(runtime.primary_agent_id())?,
        name: encode_optional_runtime_name(context, runtime.name()),
        agent_count,
        labels: encode_optional_runtime_labels(context, labels)?,
    })
}

fn encode_agent_descriptor_for_live(
    context: &mut vm::ExternalCallContext<'_>,
    world: &World,
    agent: &LiveAgent,
) -> RuntimeResult<AgentDescriptorVm> {
    let labels = world.agent_labels(agent.agent_id())?;
    let resource_count = checked_count(
        agent.resource_count(),
        "resourceCount",
        "agent resource count exceeds uint32",
    )?;

    Ok(AgentDescriptorVm {
        id: encode_agent_id(agent.agent_id())?,
        runtime_id: encode_runtime_id(agent.runtime_id())?,
        name: encode_optional_runtime_name(context, agent.name()),
        has_pending_work: agent.has_pending_work(),
        resource_count,
        labels: encode_optional_runtime_labels(context, labels)?,
    })
}

fn encode_entity_descriptor(
    context: &mut vm::ExternalCallContext<'_>,
    entity: &WorldEntity,
) -> RuntimeResult<TopologyEntityVm> {
    Ok(TopologyEntityVm {
        id: encode_topology_entity_id(context, entity.id.as_str()),
        kind: TopologyEntityKindAbi(vm::StringHandle::new(
            context.intern_string(entity.kind.as_str()),
        )),
        labels: encode_optional_runtime_labels(context, entity.labels.clone())?,
    })
}

fn encode_edge_descriptor(
    context: &mut vm::ExternalCallContext<'_>,
    edge: &WorldEdge,
) -> RuntimeResult<TopologyEdgeVm> {
    Ok(TopologyEdgeVm {
        id: encode_topology_edge_id(context, edge.id.as_str()),
        kind: TopologyEdgeKindAbi(vm::StringHandle::new(
            context.intern_string(edge.kind.as_str()),
        )),
        from: encode_topology_entity_id(context, edge.from.as_str()),
        to: encode_topology_entity_id(context, edge.to.as_str()),
        labels: encode_optional_runtime_labels(context, edge.labels.clone())?,
    })
}

fn encode_resource_descriptor(
    context: &mut vm::ExternalCallContext<'_>,
    resource: &LogicalWorldResource,
) -> ResourceDescriptorVm {
    ResourceDescriptorVm {
        id: encode_world_resource_id(resource.id),
        entity_id: encode_topology_entity_id(context, resource.id.entity_id().as_str()),
        kind: vm::StringHandle::new(context.intern_string(resource.kind.as_str())),
        label: resource
            .label
            .as_deref()
            .map(|label| vm::StringHandle::new(context.intern_string(label))),
    }
}

fn decode_world_resource_id(
    resource_id: WorldResourceIdVm,
) -> RuntimeResult<LogicalWorldResourceId> {
    Ok(LogicalWorldResourceId::new(
        decode_agent_id(resource_id.agent_id),
        resource_id.resource_id,
    ))
}

fn encode_event_loop_descriptor(
    snapshot: &EventLoopSnapshot,
) -> RuntimeResult<EventLoopDescriptorVm> {
    // queue and timer counts
    let task_count = checked_count(
        snapshot.tasks.len(),
        "taskCount",
        "event loop task count exceeds uint32",
    )?;
    let microtask_count = checked_count(
        snapshot.microtasks.len(),
        "microtaskCount",
        "event loop microtask count exceeds uint32",
    )?;
    let timer_count = checked_count(
        snapshot.ready_timers.len() + snapshot.timers.len(),
        "timerCount",
        "event loop timer count exceeds uint32",
    )?;

    // watch counts
    let watch_count = checked_count(
        snapshot.timer_watches.len()
            + snapshot.poller_event_watches.len()
            + snapshot.host_event_watches.len(),
        "watchCount",
        "event loop watch count exceeds uint32",
    )?;

    Ok(EventLoopDescriptorVm {
        task_count,
        microtask_count,
        timer_count,
        watch_count,
        has_pending_work: !snapshot.tasks.is_empty()
            || !snapshot.microtasks.is_empty()
            || !snapshot.events.is_empty()
            || !snapshot.host_events.is_empty()
            || !snapshot.ready_timers.is_empty()
            || !snapshot.timers.is_empty()
            || !snapshot.timer_watches.is_empty()
            || !snapshot.poller_event_watches.is_empty()
            || !snapshot.host_event_watches.is_empty(),
    })
}

fn encode_heap_descriptor(agent: &CapturedAgent) -> RuntimeResult<HeapDescriptorVm> {
    // page sharing
    let managed_pages = agent.heap_image.managed.pages.as_ref();
    let raw_pages = agent.heap_image.raw.pages.as_ref();
    let page_count = checked_count(
        managed_pages.len() + raw_pages.len(),
        "pageCount",
        "heap page count exceeds uint32",
    )?;
    let shared_page_count =
        count_shared_items(managed_pages)?.saturating_add(count_shared_items(raw_pages)?);

    Ok(HeapDescriptorVm {
        heap_bytes: agent.heap_image.managed.allocated_bytes,
        page_count,
        shared_page_count,
        gc_cycles: agent.heap_image.managed.gc_state.cycles,
    })
}

fn encode_engine_descriptor(agent: &CapturedAgent) -> RuntimeResult<EngineDescriptorVm> {
    match &agent.engine_image {
        EngineImage::Vm(image) => {
            // interpreter state
            let call_stack_depth = checked_count(
                image.interpreter.call_stack.len(),
                "callStackDepth",
                "engine call stack depth exceeds uint32",
            )?;
            let value_stack_depth = checked_count(
                image.interpreter.value_stack.len(),
                "valueStackDepth",
                "engine value stack depth exceeds uint32",
            )?;
            let local_stack_depth = checked_count(
                image.interpreter.local_stack.len(),
                "localStackDepth",
                "engine local stack depth exceeds uint32",
            )?;

            // serialized image size
            let image_bytes = u64::try_from(
                to_allocvec(&**image)
                    .map_err(|_| {
                        RuntimeError::InconsistentImage {
                            detail: "failed to encode engine image".to_string(),
                        }
                        .boxed()
                    })?
                    .len(),
            )
            .map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "imageBytes",
                    "engine image size exceeds uint64",
                ))
                .boxed()
            })?;

            Ok(EngineDescriptorVm {
                kind: EngineDescriptorKind::Isolate,
                call_stack_depth: Some(call_stack_depth),
                value_stack_depth: Some(value_stack_depth),
                local_stack_depth: Some(local_stack_depth),
                image_bytes: Some(image_bytes),
            })
        }
    }
}

fn encode_checkpoint_descriptor(
    context: &mut vm::ExternalCallContext<'_>,
    checkpoint: WorldCheckpoint,
) -> RuntimeResult<CheckpointDescriptorVm> {
    Ok(CheckpointDescriptorVm {
        id: encode_checkpoint_id(checkpoint.id)?,
        revision_id: encode_revision_id(checkpoint.revision_id)?,
        name: encode_optional_runtime_name(context, &checkpoint.name),
        labels: encode_optional_runtime_labels(context, checkpoint.labels)?,
    })
}

fn encode_image_descriptor(world: &World, image: &WorldImage) -> RuntimeResult<ImageDescriptorVm> {
    let revision_id = world
        .revision_ids()
        .into_iter()
        .find_map(|revision_id| {
            world
                .revision_info(revision_id)
                .ok()
                .filter(|revision| revision.image_id == image.id)
                .map(|_| revision_id)
        })
        .ok_or_else(|| {
            RuntimeError::InconsistentImage {
                detail: format!("image {} is not linked to one revision", image.id.get()),
            }
            .boxed()
        })?;
    let (shared_bytes, _) = World::image_size_and_hash(image)?;

    Ok(ImageDescriptorVm {
        id: encode_image_id(image.id)?,
        revision_id: encode_revision_id(revision_id)?,
        shared_bytes: Some(shared_bytes),
    })
}

fn encode_snapshot_descriptor(
    snapshot_id: SnapshotId,
    entry: &SnapshotHandleEntry,
) -> RuntimeResult<SnapshotDescriptorVm> {
    let size_bytes = u64::try_from(entry.bytes.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "snapshotId",
            "snapshot payload exceeds uint64",
        ))
        .boxed()
    })?;

    Ok(SnapshotDescriptorVm {
        id: snapshot_id,
        image_id: encode_image_id(entry.snapshot.image.id)?,
        format: entry.format,
        size_bytes: Some(size_bytes),
    })
}

fn encode_world_descriptor(
    context: &mut vm::ExternalCallContext<'_>,
    handle: WorldHandle,
    world: &World,
    labels: BTreeMap<String, String>,
) -> RuntimeResult<WorldDescriptorVm> {
    let labels = encode_optional_runtime_labels(context, labels)?;

    Ok(WorldDescriptorVm {
        handle,
        branch_id: BranchId(u64::try_from(world.branch_id().get()).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "branchId",
                "world branch id exceeds uint64",
            ))
            .boxed()
        })?),
        revision_id: RevisionId(u64::try_from(world.revision_id().get()).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "revisionId",
                "world revision id exceeds uint64",
            ))
            .boxed()
        })?),
        wall_ns: world.wall().get(),
        mono_ns: world.mono().get(),
        virtual_ns: world.clock().virtual_wall().get(),
        runtime_count: world.runtime_ids().len() as u32,
        labels,
    })
}

fn encode_world_descriptor_for_revision(
    context: &mut vm::ExternalCallContext<'_>,
    handle: WorldHandle,
    revision: WorldRevision,
    image: &WorldImage,
    labels: BTreeMap<String, String>,
) -> RuntimeResult<WorldDescriptorVm> {
    Ok(WorldDescriptorVm {
        handle,
        branch_id: encode_branch_id(revision.branch_id)?,
        revision_id: encode_revision_id(revision.id)?,
        wall_ns: revision.wall.get(),
        mono_ns: revision.mono.get(),
        virtual_ns: image.clock.virtual_wall.get(),
        runtime_count: image.runtimes.len() as u32,
        labels: encode_optional_runtime_labels(context, labels)?,
    })
}

fn encode_observation_kind(kind: ObservationKind) -> ObservationEventKind {
    match kind {
        ObservationKind::Trace => ObservationEventKind::Trace,
        ObservationKind::Topology => ObservationEventKind::Topology,
        ObservationKind::Resource => ObservationEventKind::Resource,
        ObservationKind::Scheduler => ObservationEventKind::Scheduler,
        ObservationKind::Diagnostic => ObservationEventKind::Diagnostic,
        ObservationKind::Profile => ObservationEventKind::Profile,
    }
}

fn encode_observation_records(
    context: &mut vm::ExternalCallContext<'_>,
    records: Vec<WorldObservationRecord>,
) -> RuntimeResult<VmArray<ObservationRecordVm>> {
    let encoded = records
        .into_iter()
        .map(|record| {
            let payload = to_allocvec(&record.event).map_err(|_| {
                RuntimeError::from(PlatformError::io("failed to encode observation payload"))
                    .boxed()
            })?;

            Ok(ObservationRecordVm {
                kind: encode_observation_kind(record.kind),
                sequence: Some(TraceSequence(record.sequence.get())),
                payload: Some(VmArray::from_bytes(context, &payload)),
            })
        })
        .collect::<RuntimeResult<Vec<_>>>()?;

    VmArray::from_values(context, &encoded)
}

fn encode_trace_records(
    context: &mut vm::ExternalCallContext<'_>,
    records: Vec<(TraceSequence, TraceEvent)>,
) -> RuntimeResult<VmArray<TraceRecordVm>> {
    let encoded = records
        .into_iter()
        .map(|(sequence, event)| {
            let payload = to_allocvec(&event).map_err(|_| {
                RuntimeError::TraceEncodeFailed {
                    name: "trace".to_string(),
                }
                .boxed()
            })?;

            Ok(TraceRecordVm {
                sequence,
                kind: encode_trace_event_kind(&event),
                payload: Some(VmArray::from_bytes(context, &payload)),
            })
        })
        .collect::<RuntimeResult<Vec<_>>>()?;

    VmArray::from_values(context, &encoded)
}

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
    argument_agent: AgentHandle,
) -> RuntimeResult<()> {
    // remove the external handle first
    let mut table = control_table().write();
    let entry = table.close_agent(handle::decode_agent_handle(argument_agent))?;
    let world = table.world(entry.world_handle_id)?;

    world.remove_agent(entry.agent_id)
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
    context: &mut vm::ExternalCallContext<'_>,
    runtimehandle: RuntimeHandle,
    options: Option<AgentCreateOptionsVm>,
) -> RuntimeResult<AgentHandle> {
    // decode create options
    let options = options.unwrap_or(AgentCreateOptionsVm {
        name: None,
        labels: None,
    });
    let mut runtime_options = RuntimeOptions::default();
    runtime_options.primary_agent.name = decode_runtime_name(context, options.name)?;
    runtime_options.primary_agent.labels = decode_runtime_labels(context, options.labels)?;

    // spawn one agent in the live runtime
    let mut table = control_table().write();
    let entry = table.runtime_entry(handle::decode_runtime_handle(runtimehandle))?;
    let world = table.world(entry.world_handle_id)?;
    let agent_id =
        world.spawn_agent_with_options(entry.runtime_id, &runtime_options, empty_vm_engine()?)?;

    Ok(handle::encode_agent_handle(table.register_agent(
        entry.world_handle_id,
        world,
        entry.runtime_id,
        agent_id,
    )))
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
    context: &mut vm::ExternalCallContext<'_>,
    argument_agent: AgentHandle,
) -> RuntimeResult<AgentDescriptorVm> {
    // resolve one live agent descriptor
    let table = control_table().read();
    let (world, runtime_id, agent_id) = table.agent(handle::decode_agent_handle(argument_agent))?;
    world.with_runtime(runtime_id, |runtime| {
        let agent = runtime.agent(agent_id).ok_or_else(|| {
            RuntimeError::AgentNotFound {
                agent_id: agent_id.0,
            }
            .boxed()
        })?;

        encode_agent_descriptor_for_live(context, &world, agent)
    })
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
    argument_runtime: RuntimeHandle,
) -> RuntimeResult<()> {
    // remove the external handle first
    let mut table = control_table().write();
    let entry = table.close_runtime(handle::decode_runtime_handle(argument_runtime))?;
    let world = table.world(entry.world_handle_id)?;
    let runtime = world.remove_runtime(entry.runtime_id)?;
    let agent_ids = runtime.agent_ids();

    table.close_agent_handles(entry.world_handle_id, &agent_ids)?;

    Ok(())
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
    context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    options: Option<RuntimeCreateOptionsVm>,
) -> RuntimeResult<RuntimeHandle> {
    // decode create options
    let options = options.unwrap_or(RuntimeCreateOptionsVm {
        name: None,
        labels: None,
    });
    let mut runtime_options = RuntimeOptions::default();
    runtime_options.name = decode_runtime_name(context, options.name)?;
    runtime_options.labels = decode_runtime_labels(context, options.labels)?;

    // spawn one runtime in the live world
    let mut table = control_table().write();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let runtime_id =
        world.spawn_runtime(Vec::<String>::new(), &runtime_options, empty_vm_engine()?)?;

    Ok(handle::encode_runtime_handle(table.register_runtime(
        handle::decode_world_handle(argument_world),
        world,
        runtime_id,
    )))
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
    context: &mut vm::ExternalCallContext<'_>,
    argument_runtime: RuntimeHandle,
) -> RuntimeResult<RuntimeDescriptorVm> {
    // resolve one live runtime descriptor
    let table = control_table().read();
    let (world, runtime_id) = table.runtime(handle::decode_runtime_handle(argument_runtime))?;
    world.with_runtime(runtime_id, |runtime| {
        encode_runtime_descriptor_for_live(context, &world, runtime)
    })
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
    let mut table = control_table().write();
    table.close_world(handle::decode_world_handle(argument_world))
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
    context: &mut vm::ExternalCallContext<'_>,
    options: Option<WorldCreateOptionsVm>,
) -> RuntimeResult<WorldHandle> {
    let options = options.unwrap_or(WorldCreateOptionsVm {
        engine: None,
        execution: None,
        world: None,
        labels: None,
    });
    let labels = decode_runtime_labels(context, options.labels)?;
    let runtime_options = runtime_options_from_world_create(options);
    let world = World::from_options(&runtime_options)?;

    let mut table = control_table().write();
    Ok(handle::encode_world_handle(
        table.register_world(world, labels),
    ))
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
    context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
) -> RuntimeResult<WorldDescriptorVm> {
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let labels = table.world_labels(handle::decode_world_handle(argument_world))?;

    encode_world_descriptor(context, argument_world, &world, labels)
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
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let outcome = world.tick()?;

    Ok(match outcome {
        TickOutcome::Idle => RuntimeTickOutcome::Idle,
        TickOutcome::Progressed => RuntimeTickOutcome::Progressed,
        TickOutcome::AdvancedTime => RuntimeTickOutcome::AdvancedTime,
    })
}

/// Close one pinned runtime view.
/// Release one previously opened runtime view and its pinned state.
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
    // drop the external pinned view handle
    let mut table = control_table().write();
    table.close_world_view(handle::decode_world_view_handle(view))?;

    Ok(())
}

/// Open one pinned runtime view.
/// Create one read-consistent runtime view over one live world or one pinned revision.
/// Views provide stable reads across topology, state, trace metadata, and lineage without racing live mutation.
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
    // pin one explicit or current revision
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let options = options.unwrap_or(WorldViewOptionsVm { revision_id: None });
    let revision_id = options
        .revision_id
        .map(decode_revision_id)
        .unwrap_or_else(|| world.revision_id());
    world.revision_info(revision_id)?;

    drop(table);

    // register the pinned view
    let mut table = control_table().write();
    Ok(handle::encode_world_view_handle(table.open_world_view(
        handle::decode_world_handle(argument_world),
        revision_id,
    )?))
}

/// Describe the world visible through one pinned runtime view.
/// Return one world descriptor as observed through one runtime view.
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
    context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
) -> RuntimeResult<WorldDescriptorVm> {
    // resolve the pinned revision backing for this view
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let table = control_table().read();
    let labels = table.world_labels(handle::decode_world_handle(world_view.world_handle))?;

    encode_world_descriptor_for_revision(
        context,
        world_view.world_handle,
        world_view.revision,
        &world_view.image,
        labels,
    )
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
    // resolve one pinned revision
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);

    encode_revision_descriptor(world_view.revision, &world_view.image)
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
    // resolve one pinned image
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);

    encode_image_descriptor(&world_view.world, &world_view.image)
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
    // resolve one pinned trace position
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);

    Ok(TraceDescriptorVm {
        branch_id: encode_branch_id(world_view.revision.branch_id)?,
        sequence: TraceSequence(u64::try_from(world_view.revision.sequence.get()).map_err(
            |_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "sequence",
                    "world trace sequence exceeds uint64",
                ))
                .boxed()
            },
        )?),
    })
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
    context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    filter: Option<RuntimeFilterVm>,
    after: Option<RuntimeId>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<RuntimeDescriptorVm>> {
    // enumerate pinned runtimes in stable order
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let filter = decode_runtime_filter(context, filter)?;
    let after = after.map(decode_runtime_id);
    let limit = limit.filter(|limit| *limit > 0).map(|limit| limit as usize);
    let runtimes = list_runtimes(&world_view.image, &filter, after, limit)?;
    let mut descriptors = Vec::with_capacity(runtimes.len());

    for runtime in runtimes {
        descriptors.push(encode_runtime_descriptor_for_image(
            context,
            &world_view.image,
            runtime,
        )?);
    }

    VmArray::from_values(context, &descriptors)
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
    context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    runtime_id: RuntimeId,
) -> RuntimeResult<RuntimeDescriptorVm> {
    // resolve one pinned runtime
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let runtime = runtime_in_image(&world_view.image, decode_runtime_id(runtime_id))?;

    encode_runtime_descriptor_for_image(context, &world_view.image, runtime)
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
    context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    filter: Option<AgentFilterVm>,
    after: Option<AgentId>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<AgentDescriptorVm>> {
    // enumerate pinned agents in stable order
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let filter = decode_agent_filter(context, filter)?;
    let after = after.map(decode_agent_id);
    let limit = limit.filter(|limit| *limit > 0).map(|limit| limit as usize);
    let agents = list_agents(&world_view.image, &filter, after, limit)?;
    let mut descriptors = Vec::with_capacity(agents.len());

    for agent in agents {
        descriptors.push(encode_agent_descriptor_for_image(
            context,
            &world_view.image,
            agent,
        )?);
    }

    VmArray::from_values(context, &descriptors)
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
    context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    agent_id: AgentId,
) -> RuntimeResult<AgentDescriptorVm> {
    // resolve one pinned agent
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let agent = agent_in_image(&world_view.image, decode_agent_id(agent_id))?;

    encode_agent_descriptor_for_image(context, &world_view.image, agent)
}

/// List logical world resources visible through one pinned world view.
/// Return resource descriptors from the image and topology backing one pinned world view.
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
    context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    filter: Option<ResourceFilterVm>,
    after: Option<WorldResourceIdVm>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<ResourceDescriptorVm>> {
    // enumerate pinned logical resources in stable order
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let filter = decode_resource_filter(context, filter)?;
    let after = after.map(decode_world_resource_id).transpose()?;
    let limit = limit.filter(|limit| *limit > 0).map(|limit| limit as usize);
    let resources = list_resources(&world_view.image, &filter, after, limit);
    let mut descriptors = Vec::with_capacity(resources.len());

    for resource in resources {
        descriptors.push(encode_resource_descriptor(context, resource));
    }

    VmArray::from_values(context, &descriptors)
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
    context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    resource_id: WorldResourceIdVm,
) -> RuntimeResult<ResourceDescriptorVm> {
    // resolve one pinned logical resource
    let resource_id = decode_world_resource_id(resource_id)?;
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let resource = resource_in_image(&world_view.image, resource_id)?;

    Ok(encode_resource_descriptor(context, resource))
}

/// List topology entities visible through one pinned world view.
/// Return topology entities from the image backing one pinned world view.
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
    context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    filter: Option<TopologyEntityFilterVm>,
    after: Option<TopologyEntityIdVm>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<TopologyEntityVm>> {
    // enumerate pinned topology entities in stable order
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let filter = decode_entity_filter(context, filter)?;
    let after = decode_topology_entity_id_value(context, after)?;
    let limit = limit.filter(|limit| *limit > 0).map(|limit| limit as usize);
    let entities = list_entities(&world_view.image, &filter, after.as_deref(), limit);
    let mut descriptors = Vec::with_capacity(entities.len());

    for entity in entities {
        descriptors.push(encode_entity_descriptor(context, entity)?);
    }

    VmArray::from_values(context, &descriptors)
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
    context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    entity_id: TopologyEntityIdVm,
) -> RuntimeResult<TopologyEntityVm> {
    // resolve one pinned topology entity
    let entity_id = context
        .string_ref(entity_id.0)
        .map_err(Box::<RuntimeError>::from)?
        .as_str()
        .to_string();
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let entity = entity_in_image(&world_view.image, entity_id.as_str())?;

    encode_entity_descriptor(context, entity)
}

/// List topology edges visible through one pinned world view.
/// Return topology edges from the image backing one pinned world view.
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
    context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    filter: Option<TopologyEdgeFilterVm>,
    after: Option<TopologyEdgeIdVm>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<TopologyEdgeVm>> {
    // enumerate pinned topology edges in stable order
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let filter = decode_edge_filter(context, filter)?;
    let after = decode_topology_edge_id_value(context, after)?;
    let limit = limit.filter(|limit| *limit > 0).map(|limit| limit as usize);
    let edges = list_edges(&world_view.image, &filter, after.as_deref(), limit);
    let mut descriptors = Vec::with_capacity(edges.len());

    for edge in edges {
        descriptors.push(encode_edge_descriptor(context, edge)?);
    }

    VmArray::from_values(context, &descriptors)
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
    context: &mut vm::ExternalCallContext<'_>,
    view: WorldViewHandle,
    edge_id: TopologyEdgeIdVm,
) -> RuntimeResult<TopologyEdgeVm> {
    // resolve one pinned topology edge
    let edge_id = context
        .string_ref(edge_id.0)
        .map_err(Box::<RuntimeError>::from)?
        .as_str()
        .to_string();
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let edge = edge_in_image(&world_view.image, edge_id.as_str())?;

    encode_edge_descriptor(context, edge)
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
    agent_id: AgentId,
) -> RuntimeResult<EventLoopDescriptorVm> {
    // resolve one pinned agent event loop
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let agent = agent_in_image(&world_view.image, decode_agent_id(agent_id))?;

    encode_event_loop_descriptor(&agent.event_loop)
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
    agent_id: AgentId,
) -> RuntimeResult<HeapDescriptorVm> {
    // resolve one pinned agent heap
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let agent = agent_in_image(&world_view.image, decode_agent_id(agent_id))?;

    encode_heap_descriptor(agent)
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
    agent_id: AgentId,
) -> RuntimeResult<EngineDescriptorVm> {
    // resolve one pinned agent engine
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let agent = agent_in_image(&world_view.image, decode_agent_id(agent_id))?;

    encode_engine_descriptor(agent)
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
    context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    branchid: BranchId,
) -> RuntimeResult<BranchDescriptorVm> {
    // load one live world and branch record
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let branch = world.branch_info(decode_branch_id(branchid))?;

    encode_branch_descriptor(context, branch)
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
    context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    filter: Option<BranchFilterVm>,
    after: Option<BranchId>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<BranchDescriptorVm>> {
    // enumerate branch heads in stable order
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let filter = decode_branch_filter(context, filter)?;
    let limit = if limit.unwrap_or(0) == 0 {
        usize::MAX
    } else {
        limit.unwrap_or(0) as usize
    };
    let after = after.map(|value| value.0).unwrap_or(0);
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
        .map(|branch| branch.and_then(|branch| encode_branch_descriptor(context, branch)))
        .collect::<RuntimeResult<Vec<_>>>()?;

    VmArray::from_values(context, &descriptors)
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
    context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    name: Option<vm::StringHandle>,
    labels: Option<VmArray<RuntimeLabelVm>>,
) -> RuntimeResult<CheckpointId> {
    // decode one checkpoint request
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let name = decode_runtime_name(context, name)?;
    let labels = decode_runtime_labels(context, labels)?;
    let checkpoint_name = name.as_deref().unwrap_or("checkpoint");

    // capture the checkpoint through world lineage
    let checkpoint_id = world.checkpoint(checkpoint_name)?;

    // carry through low-level labels until checkpoint metadata grows a direct API
    if !labels.is_empty() {
        world.set_checkpoint_labels(checkpoint_id, labels)?;
    }

    encode_checkpoint_id(checkpoint_id)
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
    context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    checkpointid: CheckpointId,
) -> RuntimeResult<CheckpointDescriptorVm> {
    // load one live checkpoint record
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let checkpoint = world.checkpoint_info(decode_checkpoint_id(checkpointid))?;

    encode_checkpoint_descriptor(context, checkpoint)
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
    context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    filter: Option<CheckpointFilterVm>,
    after: Option<CheckpointId>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<CheckpointDescriptorVm>> {
    // enumerate checkpoints in stable order
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let filter = decode_checkpoint_filter(context, filter)?;
    let limit = if limit.unwrap_or(0) == 0 {
        usize::MAX
    } else {
        limit.unwrap_or(0) as usize
    };
    let after = after.map(|value| value.0).unwrap_or(0);
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
            checkpoint.and_then(|checkpoint| encode_checkpoint_descriptor(context, checkpoint))
        })
        .collect::<RuntimeResult<Vec<_>>>()?;

    VmArray::from_values(context, &descriptors)
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
    // materialize one live image through suspend capture
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let revision_id = world.suspend()?;
    let revision = world.revision_info(revision_id)?;

    encode_image_id(revision.image_id)
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
    // load one stored image descriptor
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let image = world.image_info(decode_image_id(imageid))?;

    encode_image_descriptor(&world, &image)
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
    context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    filter: Option<ImageFilterVm>,
    after: Option<ImageId>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<ImageDescriptorVm>> {
    // enumerate materialized images in stable order
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let filter = decode_image_filter(filter);
    let limit = if limit.unwrap_or(0) == 0 {
        usize::MAX
    } else {
        limit.unwrap_or(0) as usize
    };
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
        .map(|image| image.and_then(|image| encode_image_descriptor(&world, &image)))
        .collect::<RuntimeResult<Vec<_>>>()?;

    VmArray::from_values(context, &descriptors)
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
    // load one stored revision descriptor
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let revision = world.revision_info(decode_revision_id(revisionid))?;
    let image = world.image_info(revision.image_id)?;

    encode_revision_descriptor(revision, &image)
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
    context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    filter: Option<RevisionFilterVm>,
    after: Option<RevisionId>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<RevisionDescriptorVm>> {
    // enumerate revisions in stable order
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let filter = decode_revision_filter(filter);
    let limit = if limit.unwrap_or(0) == 0 {
        usize::MAX
    } else {
        limit.unwrap_or(0) as usize
    };
    let after = after.map(|value| value.0).unwrap_or(0);
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

            encode_revision_descriptor(revision, &image)
        })
        .collect::<RuntimeResult<Vec<_>>>()?;

    VmArray::from_values(context, &descriptors)
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
    // read the active branch directly from the live world
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;

    encode_branch_id(world.branch_id())
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
    context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    revisionid: RevisionId,
    name: Option<vm::StringHandle>,
    labels: Option<VmArray<RuntimeLabelVm>>,
) -> RuntimeResult<WorldHandle> {
    // decode one fork request
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let revision_id = decode_revision_id(revisionid);
    let name = decode_runtime_name(context, name)?;
    let labels = decode_runtime_labels(context, labels)?;
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
    Ok(handle::encode_world_handle(
        table.register_world(child, BTreeMap::new()),
    ))
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
    // read the active revision directly from the live world
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;

    encode_revision_id(world.revision_id())
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
    // restore one live world to one checkpointed revision
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    world.rewind(decode_checkpoint_id(checkpointid))
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
    // restore one live world to one explicit revision
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    world.rewind_revision(decode_revision_id(revisionid))
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
    let mut table = control_table().write();
    let entry = handle::encode_observation_entry(
        table.close_observation(handle::decode_observation_handle(handle))?,
    );
    let world = table.world(handle::decode_world_handle(entry.world))?;

    world.observation().close(entry.subscription_id)
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
    context: &mut vm::ExternalCallContext<'_>,
    handle: ObservationHandle,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<ObservationRecordVm>> {
    let table = control_table().read();
    let (world, subscription_id) = table.observation(handle::decode_observation_handle(handle))?;
    let limit = if limit.unwrap_or(0) == 0 {
        usize::MAX
    } else {
        limit.unwrap_or(0) as usize
    };
    let records = world.observation().next(subscription_id, limit)?;

    encode_observation_records(context, records)
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
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let options = options.unwrap_or(ObservationOptionsVm {
        trace: None,
        topology: None,
        resources: None,
        scheduler: None,
        diagnostics: None,
        profiles: None,
    });
    let subscription_id = world
        .observation()
        .open(observation_options_from_abi(options));

    drop(table);

    // register the observation handle
    let mut table = control_table().write();
    Ok(handle::encode_observation_handle(
        table.open_observation_handle(
            handle::decode_world_handle(argument_world),
            subscription_id,
        )?,
    ))
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
    // resolve the live world and export one snapshot
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let snapshot = world.snapshot(decode_image_id(imageid))?;
    let bytes = Arc::<[u8]>::from(snapshot.encode()?);

    drop(table);

    // store the exported snapshot
    let mut table = control_table().write();

    Ok(handle::encode_snapshot_id(table.store_snapshot_handle(
        handle::decode_world_handle(argument_world),
        handle::decode_snapshot_format(format),
        snapshot,
        bytes,
    )?))
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
    // resolve the stored snapshot for this world
    let table = control_table().read();
    let entry =
        handle::encode_snapshot_entry(table.snapshot(handle::decode_snapshot_id(snapshotid))?);
    if entry.world != argument_world {
        return Err(io_not_found(
            "destack.runtime.snapshot.describe",
            format!("unknown snapshot {}", snapshotid.0),
        ));
    }

    encode_snapshot_descriptor(snapshotid, &entry)
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
    context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    argument_payload: VmArray<u8>,
) -> RuntimeResult<SnapshotId> {
    // resolve the live world before storing the imported snapshot
    let table = control_table().read();
    let _world = table.world(handle::decode_world_handle(argument_world))?;

    // decode one stored snapshot payload
    let payload = argument_payload.read_bytes(context)?;
    let snapshot = WorldSnapshot::decode(&payload)?;
    let bytes = Arc::<[u8]>::from(payload);

    // release the read lock before storing the imported snapshot
    drop(table);

    Ok(handle::encode_snapshot_id(
        control_table().write().store_snapshot_handle(
            handle::decode_world_handle(argument_world),
            handle::decode_snapshot_format(SnapshotFormat::Portable),
            snapshot,
            bytes,
        )?,
    ))
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
    context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    after: Option<SnapshotId>,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<SnapshotDescriptorVm>> {
    // enumerate stored snapshots for this world
    let limit = limit.map(|limit| limit as usize);
    let table = control_table().read();
    let descriptors = table
        .snapshots_for_world(
            handle::decode_world_handle(argument_world),
            after.map(handle::decode_snapshot_id),
            limit,
        )
        .into_iter()
        .map(|(snapshot_id, entry)| {
            (
                handle::encode_snapshot_id(snapshot_id),
                handle::encode_snapshot_entry(entry),
            )
        })
        .into_iter()
        .map(|(snapshot_id, entry)| encode_snapshot_descriptor(snapshot_id, &entry))
        .collect::<RuntimeResult<Vec<_>>>()?;

    VmArray::from_values(context, &descriptors)
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
    context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    snapshotid: SnapshotId,
) -> RuntimeResult<VmArray<u8>> {
    // resolve the stored snapshot payload for this world
    let table = control_table().read();
    let entry =
        handle::encode_snapshot_entry(table.snapshot(handle::decode_snapshot_id(snapshotid))?);
    if entry.world != argument_world {
        return Err(io_not_found(
            "destack.runtime.snapshot.read",
            format!("unknown snapshot {}", snapshotid.0),
        ));
    }

    Ok(VmArray::from_bytes(context, entry.bytes.as_ref()))
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
    // restore the requested image into the live world
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;

    // release the read lock before mutating the world
    drop(table);

    world.restore_image_id(decode_image_id(imageid), None)
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
    // resolve the snapshot and restore it into the live world
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let entry =
        handle::encode_snapshot_entry(table.snapshot(handle::decode_snapshot_id(snapshotid))?);
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
    // drop the external cursor handle
    let mut table = control_table().write();
    table.close_trace_cursor(handle::decode_trace_cursor_handle(cursor))?;

    Ok(())
}

/// Describe one world's causal trace.
/// Return one summary of the active branch, next sequence, and checkpoint count for one world trace.
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
    // summarize the active world trace
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;

    Ok(TraceDescriptorVm {
        branch_id: encode_branch_id(world.branch_id())?,
        sequence: TraceSequence(
            u64::try_from(world.trace().log().next_sequence().get()).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "sequence",
                    "world trace sequence exceeds uint64",
                ))
                .boxed()
            })?,
        ),
    })
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
    context: &mut vm::ExternalCallContext<'_>,
    argument_world: WorldHandle,
    label: vm::StringHandle,
) -> RuntimeResult<TraceSequence> {
    // decode the marker label from the vm call context
    let label = context
        .string_ref(label)
        .map_err(Box::<RuntimeError>::from)?
        .as_str()
        .to_string();

    // resolve the live world and record the marker
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    drop(table);
    let sequence = world.trace().record_marker(label)?;

    Ok(TraceSequence(sequence.get()))
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
    context: &mut vm::ExternalCallContext<'_>,
    cursor: TraceCursorHandle,
    limit: Option<u32>,
) -> RuntimeResult<VmArray<TraceRecordVm>> {
    // read the next batch from one live cursor
    let table = control_table().read();
    let (_world, cursor) = table.trace_cursor(handle::decode_trace_cursor_handle(cursor))?;
    let limit = if limit.unwrap_or(0) == 0 {
        usize::MAX
    } else {
        limit.unwrap_or(0) as usize
    };
    let mut records = Vec::new();

    for _ in 0..limit {
        let sequence = cursor.tell();
        let Some(event) = cursor.next_event()? else {
            break;
        };

        records.push((
            TraceSequence(u64::try_from(sequence.get()).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "sequence",
                    "world trace sequence exceeds uint64",
                ))
                .boxed()
            })?),
            event,
        ));
    }

    encode_trace_records(context, records)
}

/// Open one causal trace cursor.
/// Create one cursor for reading structured causal trace records from one world.
/// Cursors start at the optional sequence boundary and advance independently from the live trace head.
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
    // open one live cursor at the requested sequence
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let cursor = Arc::new(world.trace().log().reader());
    let options = options.unwrap_or(TraceCursorOptionsVm {
        start_sequence: None,
    });
    if let Some(start_sequence) = options.start_sequence {
        cursor.seek_sequence(RuntimeTraceSequence::new(start_sequence.0))?;
    }

    drop(table);

    // register the trace cursor
    let mut table = control_table().write();
    Ok(handle::encode_trace_cursor_handle(
        table.open_trace_cursor_handle(handle::decode_world_handle(argument_world), cursor)?,
    ))
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
    // seek to the revision sequence anchored by one checkpoint
    let table = control_table().read();
    let (world, cursor) = table.trace_cursor(handle::decode_trace_cursor_handle(cursor))?;
    let checkpoint = world.checkpoint_info(decode_checkpoint_id(checkpointid))?;
    let revision = world.revision_info(checkpoint.revision_id)?;

    cursor.seek_sequence(revision.sequence)
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
    // seek to the sequence captured by one revision
    let table = control_table().read();
    let (world, cursor) = table.trace_cursor(handle::decode_trace_cursor_handle(cursor))?;
    let revision = world.revision_info(decode_revision_id(revisionid))?;

    cursor.seek_sequence(revision.sequence)
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
    // seek one live cursor directly to one sequence boundary
    let table = control_table().read();
    let (_world, cursor) = table.trace_cursor(handle::decode_trace_cursor_handle(cursor))?;
    cursor.seek_sequence(RuntimeTraceSequence::new(sequence.0))
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
    // expose the next visible sequence for one live cursor
    let table = control_table().read();
    let (_world, cursor) = table.trace_cursor(handle::decode_trace_cursor_handle(cursor))?;
    let sequence = cursor.tell();

    Ok(TraceSequence(u64::try_from(sequence.get()).map_err(
        |_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "sequence",
                "world trace sequence exceeds uint64",
            ))
            .boxed()
        },
    )?))
}
