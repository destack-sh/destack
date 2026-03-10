use std::collections::BTreeMap;
use std::sync::Arc;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::{ensure_out, io_not_found};
use crate::platform::runtime::abi_generated::{
    TopologyEdgeIdAbi, TopologyEdgeKindAbi, TopologyEntityIdAbi, TopologyEntityKindAbi,
};
use crate::platform::{NativeArray, NativeStringRef, PlatformError};
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
    ObservationOptions as WorldObservationOptions, ObservationRecord as WorldObservationRecord,
    Revision as WorldRevision, Snapshot as WorldSnapshot, World, WorldEdge, WorldEntity,
    WorldResource as LogicalWorldResource, WorldResourceId as LogicalWorldResourceId,
};
use crate::runtime::{
    Agent as LiveAgent, AgentImage as CapturedAgent, BindingCallContext, Runtime as LiveRuntime,
    RuntimeImage as CapturedRuntime, TickOutcome,
};
use destack_workspace::{ExecutionMode, RuntimeOptions, RuntimeWorld, TimeMode};
use postcard::to_allocvec;

use super::handle::{
    self as handle, SnapshotHandleEntry, decode_agent_id, decode_branch_id, decode_checkpoint_id,
    decode_image_id, decode_revision_id, decode_runtime_id, encode_agent_id, encode_branch_id,
    encode_checkpoint_id, encode_image_id, encode_revision_id, encode_runtime_id,
    encode_trace_event_kind,
};
use crate::platform::runtime::{
    AgentCreateOptions, AgentDescriptor, AgentFilter, AgentHandle, AgentId, BranchDescriptor,
    BranchFilter, BranchId, CheckpointDescriptor, CheckpointFilter, CheckpointId, EngineDescriptor,
    EngineDescriptorKind, EventLoopDescriptor, HeapDescriptor, ImageDescriptor, ImageFilter,
    ImageId, ObservationEventKind, ObservationHandle, ObservationOptions, ObservationRecord,
    ResourceDescriptor, ResourceFilter, RevisionDescriptor, RevisionFilter, RevisionId,
    RuntimeCreateOptions, RuntimeDescriptor, RuntimeExecutionMode, RuntimeFilter, RuntimeHandle,
    RuntimeId, RuntimeLabel, RuntimeLabelSelector, RuntimeTickOutcome, RuntimeWorldKind,
    SnapshotDescriptor, SnapshotFormat, SnapshotId, TopologyEdge as TopologyEdgeDescriptor,
    TopologyEdgeFilter, TopologyEdgeId, TopologyEdgeKind,
    TopologyEntity as TopologyEntityDescriptor, TopologyEntityFilter, TopologyEntityId,
    TopologyEntityKind, TraceCursorHandle, TraceCursorOptions, TraceDescriptor, TraceRecord,
    TraceSequence, WorldCreateOptions, WorldDescriptor, WorldHandle, WorldResourceId,
    WorldViewHandle, WorldViewOptions,
};

fn runtime_options_from_world_create(options: WorldCreateOptions) -> RuntimeOptions {
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

fn observation_options_from_abi(options: ObservationOptions) -> WorldObservationOptions {
    WorldObservationOptions {
        trace: options.trace.unwrap_or(false),
        topology: options.topology.unwrap_or(false),
        resource: options.resources.unwrap_or(false),
        scheduler: options.scheduler.unwrap_or(false),
        diagnostic: options.diagnostics.unwrap_or(false),
        profile: options.profiles.unwrap_or(false),
    }
}

unsafe fn decode_runtime_labels(
    labels: Option<NativeArray<RuntimeLabel>>,
) -> RuntimeResult<BTreeMap<String, String>> {
    // absent labels
    let Some(labels) = labels else {
        return Ok(BTreeMap::new());
    };

    let labels = unsafe { labels.as_slice()? };
    let mut decoded = BTreeMap::new();

    for label in labels {
        let key = unsafe { label.key.as_str()? }.to_string();
        let value = unsafe { label.value.as_str()? }.to_string();
        decoded.insert(key, value);
    }

    Ok(decoded)
}

unsafe fn decode_label_selectors(
    selectors: Option<NativeArray<RuntimeLabelSelector>>,
) -> RuntimeResult<Vec<(String, Option<String>)>> {
    // absent selectors
    let Some(selectors) = selectors else {
        return Ok(Vec::new());
    };

    let selectors = unsafe { selectors.as_slice()? };
    let mut decoded = Vec::with_capacity(selectors.len());

    for selector in selectors {
        let key = unsafe { selector.key.as_str()? }.to_string();
        let value = match selector.value {
            Some(value) => Some(unsafe { value.as_str()? }.to_string()),
            None => None,
        };
        decoded.push((key, value));
    }

    Ok(decoded)
}

fn encode_optional_runtime_name(
    binding: &BindingCallContext,
    name: &str,
) -> Option<NativeStringRef> {
    if name.is_empty() {
        return None;
    }

    Some(binding.store_string(name))
}

fn encode_runtime_labels(
    binding: &BindingCallContext,
    labels: BTreeMap<String, String>,
) -> NativeArray<RuntimeLabel> {
    let labels = labels
        .into_iter()
        .map(|(key, value)| RuntimeLabel {
            key: binding.store_string(&key),
            value: binding.store_string(&value),
        })
        .collect();

    binding.store_array(labels)
}

fn encode_optional_runtime_labels(
    binding: &BindingCallContext,
    labels: BTreeMap<String, String>,
) -> Option<NativeArray<RuntimeLabel>> {
    if labels.is_empty() {
        return None;
    }

    Some(encode_runtime_labels(binding, labels))
}

unsafe fn decode_runtime_name(name: Option<NativeStringRef>) -> RuntimeResult<Option<String>> {
    let Some(name) = name else {
        return Ok(None);
    };

    Ok(Some(unsafe { name.as_str()? }.to_string()))
}

unsafe fn decode_runtime_filter(filter: Option<RuntimeFilter>) -> RuntimeResult<RuntimeListFilter> {
    let Some(filter) = filter else {
        return Ok(RuntimeListFilter::default());
    };

    Ok(RuntimeListFilter {
        name: unsafe { decode_runtime_name(filter.name)? },
        labels: unsafe { decode_label_selectors(filter.labels)? },
    })
}

unsafe fn decode_agent_filter(filter: Option<AgentFilter>) -> RuntimeResult<AgentListFilter> {
    let Some(filter) = filter else {
        return Ok(AgentListFilter::default());
    };

    Ok(AgentListFilter {
        runtime_id: filter.runtime_id.map(decode_runtime_id),
        name: unsafe { decode_runtime_name(filter.name)? },
        has_pending_work: filter.has_pending_work,
        labels: unsafe { decode_label_selectors(filter.labels)? },
    })
}

unsafe fn decode_branch_filter(filter: Option<BranchFilter>) -> RuntimeResult<BranchListFilter> {
    let Some(filter) = filter else {
        return Ok(BranchListFilter::default());
    };

    Ok(BranchListFilter {
        name: unsafe { decode_runtime_name(filter.name)? },
        labels: unsafe { decode_label_selectors(filter.labels)? },
    })
}

fn decode_revision_filter(filter: Option<RevisionFilter>) -> RevisionListFilter {
    let Some(filter) = filter else {
        return RevisionListFilter::default();
    };

    RevisionListFilter {
        branch_id: filter.branch_id.map(decode_branch_id),
    }
}

unsafe fn decode_checkpoint_filter(
    filter: Option<CheckpointFilter>,
) -> RuntimeResult<CheckpointListFilter> {
    let Some(filter) = filter else {
        return Ok(CheckpointListFilter::default());
    };

    Ok(CheckpointListFilter {
        revision_id: filter.revision_id.map(decode_revision_id),
        name: unsafe { decode_runtime_name(filter.name)? },
        labels: unsafe { decode_label_selectors(filter.labels)? },
    })
}

fn decode_image_filter(filter: Option<ImageFilter>) -> ImageListFilter {
    let Some(filter) = filter else {
        return ImageListFilter::default();
    };

    ImageListFilter {
        revision_id: filter.revision_id.map(decode_revision_id),
    }
}

unsafe fn decode_resource_filter(
    filter: Option<ResourceFilter>,
) -> RuntimeResult<ResourceListFilter> {
    let Some(filter) = filter else {
        return Ok(ResourceListFilter::default());
    };

    Ok(ResourceListFilter {
        runtime_id: filter.runtime_id.map(decode_runtime_id),
        agent_id: filter.agent_id.map(decode_agent_id),
        kind: unsafe { decode_runtime_name(filter.kind)? },
        label: unsafe { decode_runtime_name(filter.label)? },
    })
}

unsafe fn decode_entity_filter(
    filter: Option<TopologyEntityFilter>,
) -> RuntimeResult<EntityListFilter> {
    let Some(filter) = filter else {
        return Ok(EntityListFilter::default());
    };

    Ok(EntityListFilter {
        kind: unsafe { decode_topology_entity_kind(filter.kind)? },
        labels: unsafe { decode_label_selectors(filter.labels)? },
    })
}

unsafe fn decode_edge_filter(filter: Option<TopologyEdgeFilter>) -> RuntimeResult<EdgeListFilter> {
    let Some(filter) = filter else {
        return Ok(EdgeListFilter::default());
    };

    Ok(EdgeListFilter {
        kind: unsafe { decode_topology_edge_kind(filter.kind)? },
        from: unsafe { decode_topology_entity_id_value(filter.from)? },
        to: unsafe { decode_topology_entity_id_value(filter.to)? },
        labels: unsafe { decode_label_selectors(filter.labels)? },
    })
}

unsafe fn decode_topology_entity_id_value(
    value: Option<TopologyEntityId>,
) -> RuntimeResult<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };

    Ok(Some(unsafe { value.0.as_str()? }.to_string()))
}

unsafe fn decode_topology_edge_id_value(
    value: Option<TopologyEdgeId>,
) -> RuntimeResult<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };

    Ok(Some(unsafe { value.0.as_str()? }.to_string()))
}

unsafe fn decode_topology_entity_kind(
    value: Option<TopologyEntityKind>,
) -> RuntimeResult<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };

    Ok(Some(unsafe { value.0.as_str()? }.to_string()))
}

unsafe fn decode_topology_edge_kind(
    value: Option<TopologyEdgeKind>,
) -> RuntimeResult<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };

    Ok(Some(unsafe { value.0.as_str()? }.to_string()))
}

fn encode_topology_entity_id(binding: &BindingCallContext, entity_id: &str) -> TopologyEntityId {
    TopologyEntityIdAbi(binding.store_string(entity_id))
}

fn encode_topology_edge_id(binding: &BindingCallContext, edge_id: &str) -> TopologyEdgeId {
    TopologyEdgeIdAbi(binding.store_string(edge_id))
}

fn encode_world_resource_id(resource_id: LogicalWorldResourceId) -> WorldResourceId {
    WorldResourceId {
        agent_id: AgentId(resource_id.agent_id.0),
        resource_id: resource_id.resource_id,
    }
}

fn encode_branch_descriptor(
    binding: &BindingCallContext,
    branch: WorldBranch,
) -> RuntimeResult<BranchDescriptor> {
    Ok(BranchDescriptor {
        id: encode_branch_id(branch.id)?,
        head_revision: encode_revision_id(branch.head_revision_id)?,
        name: encode_optional_runtime_name(binding, &branch.name),
        labels: encode_optional_runtime_labels(binding, branch.labels),
    })
}

fn encode_revision_descriptor(
    revision: WorldRevision,
    image: &WorldImage,
) -> RuntimeResult<RevisionDescriptor> {
    Ok(RevisionDescriptor {
        id: encode_revision_id(revision.id)?,
        branch_id: encode_branch_id(revision.branch_id)?,
        parent_revision: revision
            .parent_revision_id
            .map(encode_revision_id)
            .transpose()?,
        sequence: TraceSequence(revision.sequence.get()),
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
    binding: &BindingCallContext,
    image: &WorldImage,
    runtime: &CapturedRuntime,
) -> RuntimeResult<RuntimeDescriptor> {
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

    Ok(RuntimeDescriptor {
        id: encode_runtime_id(runtime.runtime_id)?,
        primary_agent_id: encode_agent_id(runtime.primary_agent_id)?,
        name: encode_optional_runtime_name(binding, &runtime.name),
        agent_count,
        labels: encode_optional_runtime_labels(binding, labels.clone()),
    })
}

fn encode_agent_descriptor_for_image(
    binding: &BindingCallContext,
    image: &WorldImage,
    agent: &CapturedAgent,
) -> RuntimeResult<AgentDescriptor> {
    let labels = image.agent_labels(agent.agent_id)?;
    let resource_count = u32::try_from(agent.resources.entries.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "resourceCount",
            "agent resource count exceeds uint32",
        ))
        .boxed()
    })?;

    Ok(AgentDescriptor {
        id: encode_agent_id(agent.agent_id)?,
        runtime_id: encode_runtime_id(agent.runtime_id)?,
        name: encode_optional_runtime_name(binding, &agent.name),
        has_pending_work: agent.has_pending_work(),
        resource_count,
        labels: encode_optional_runtime_labels(binding, labels.clone()),
    })
}

fn encode_runtime_descriptor_for_live(
    binding: &BindingCallContext,
    world: &World,
    runtime: &LiveRuntime,
) -> RuntimeResult<RuntimeDescriptor> {
    let labels = world.runtime_labels(runtime.runtime_id())?;
    let agent_count = checked_count(
        runtime.agent_count(),
        "agentCount",
        "runtime agent count exceeds uint32",
    )?;

    Ok(RuntimeDescriptor {
        id: encode_runtime_id(runtime.runtime_id())?,
        primary_agent_id: encode_agent_id(runtime.primary_agent_id())?,
        name: encode_optional_runtime_name(binding, runtime.name()),
        agent_count,
        labels: encode_optional_runtime_labels(binding, labels),
    })
}

fn encode_agent_descriptor_for_live(
    binding: &BindingCallContext,
    world: &World,
    agent: &LiveAgent,
) -> RuntimeResult<AgentDescriptor> {
    let labels = world.agent_labels(agent.agent_id())?;
    let resource_count = checked_count(
        agent.resource_count(),
        "resourceCount",
        "agent resource count exceeds uint32",
    )?;

    Ok(AgentDescriptor {
        id: encode_agent_id(agent.agent_id())?,
        runtime_id: encode_runtime_id(agent.runtime_id())?,
        name: encode_optional_runtime_name(binding, agent.name()),
        has_pending_work: agent.has_pending_work(),
        resource_count,
        labels: encode_optional_runtime_labels(binding, labels),
    })
}

fn encode_entity_descriptor(
    binding: &BindingCallContext,
    entity: &WorldEntity,
) -> TopologyEntityDescriptor {
    TopologyEntityDescriptor {
        id: encode_topology_entity_id(binding, entity.id.as_str()),
        kind: TopologyEntityKindAbi(binding.store_string(entity.kind.as_str())),
        labels: encode_optional_runtime_labels(binding, entity.labels.clone()),
    }
}

fn encode_edge_descriptor(
    binding: &BindingCallContext,
    edge: &WorldEdge,
) -> TopologyEdgeDescriptor {
    TopologyEdgeDescriptor {
        id: encode_topology_edge_id(binding, edge.id.as_str()),
        kind: TopologyEdgeKindAbi(binding.store_string(edge.kind.as_str())),
        from: encode_topology_entity_id(binding, edge.from.as_str()),
        to: encode_topology_entity_id(binding, edge.to.as_str()),
        labels: encode_optional_runtime_labels(binding, edge.labels.clone()),
    }
}

fn encode_resource_descriptor(
    binding: &BindingCallContext,
    resource: &LogicalWorldResource,
) -> ResourceDescriptor {
    ResourceDescriptor {
        id: encode_world_resource_id(resource.id),
        entity_id: encode_topology_entity_id(binding, resource.id.entity_id().as_str()),
        kind: binding.store_string(resource.kind.as_str()),
        label: resource
            .label
            .as_deref()
            .map(|label| binding.store_string(label)),
    }
}

fn decode_world_resource_id(resource_id: WorldResourceId) -> RuntimeResult<LogicalWorldResourceId> {
    Ok(LogicalWorldResourceId::new(
        decode_agent_id(resource_id.agent_id),
        resource_id.resource_id,
    ))
}

fn encode_event_loop_descriptor(
    snapshot: &EventLoopSnapshot,
) -> RuntimeResult<EventLoopDescriptor> {
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

    Ok(EventLoopDescriptor {
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

fn encode_heap_descriptor(agent: &CapturedAgent) -> RuntimeResult<HeapDescriptor> {
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

    Ok(HeapDescriptor {
        heap_bytes: agent.heap_image.managed.allocated_bytes,
        page_count,
        shared_page_count,
        gc_cycles: agent.heap_image.managed.gc_state.cycles,
    })
}

fn encode_engine_descriptor(agent: &CapturedAgent) -> RuntimeResult<EngineDescriptor> {
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

            Ok(EngineDescriptor {
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
    binding: &BindingCallContext,
    checkpoint: WorldCheckpoint,
) -> RuntimeResult<CheckpointDescriptor> {
    Ok(CheckpointDescriptor {
        id: encode_checkpoint_id(checkpoint.id)?,
        revision_id: encode_revision_id(checkpoint.revision_id)?,
        name: encode_optional_runtime_name(binding, &checkpoint.name),
        labels: encode_optional_runtime_labels(binding, checkpoint.labels),
    })
}

fn encode_image_descriptor(world: &World, image: &WorldImage) -> RuntimeResult<ImageDescriptor> {
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

    Ok(ImageDescriptor {
        id: encode_image_id(image.id)?,
        revision_id: encode_revision_id(revision_id)?,
        shared_bytes: Some(shared_bytes),
    })
}

fn encode_snapshot_descriptor(
    snapshot_id: SnapshotId,
    entry: &SnapshotHandleEntry,
) -> RuntimeResult<SnapshotDescriptor> {
    let size_bytes = u64::try_from(entry.bytes.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "snapshotId",
            "snapshot payload exceeds uint64",
        ))
        .boxed()
    })?;

    Ok(SnapshotDescriptor {
        id: snapshot_id,
        image_id: encode_image_id(entry.snapshot.image.id)?,
        format: entry.format,
        size_bytes: Some(size_bytes),
    })
}

fn encode_world_descriptor(
    binding: &BindingCallContext,
    handle: WorldHandle,
    world: &World,
    labels: BTreeMap<String, String>,
) -> RuntimeResult<WorldDescriptor> {
    Ok(WorldDescriptor {
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
        labels: encode_optional_runtime_labels(binding, labels),
    })
}

fn encode_world_descriptor_for_revision(
    binding: &BindingCallContext,
    handle: WorldHandle,
    revision: WorldRevision,
    image: &WorldImage,
    labels: BTreeMap<String, String>,
) -> RuntimeResult<WorldDescriptor> {
    Ok(WorldDescriptor {
        handle,
        branch_id: encode_branch_id(revision.branch_id)?,
        revision_id: encode_revision_id(revision.id)?,
        wall_ns: revision.wall.get(),
        mono_ns: revision.mono.get(),
        virtual_ns: image.clock.virtual_wall.get(),
        runtime_count: image.runtimes.len() as u32,
        labels: encode_optional_runtime_labels(binding, labels),
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
    binding: &BindingCallContext,
    records: Vec<WorldObservationRecord>,
) -> RuntimeResult<NativeArray<ObservationRecord>> {
    let encoded = records
        .into_iter()
        .map(|record| {
            let payload = to_allocvec(&record.event).map_err(|_| {
                RuntimeError::from(PlatformError::io("failed to encode observation payload"))
                    .boxed()
            })?;

            Ok(ObservationRecord {
                kind: encode_observation_kind(record.kind),
                sequence: Some(TraceSequence(record.sequence.get())),
                payload: Some(binding.store_array(payload)),
            })
        })
        .collect::<RuntimeResult<Vec<_>>>()?;

    Ok(binding.store_array(encoded))
}

fn encode_trace_records(
    binding: &BindingCallContext,
    records: Vec<(TraceSequence, TraceEvent)>,
) -> RuntimeResult<NativeArray<TraceRecord>> {
    let encoded = records
        .into_iter()
        .map(|(sequence, event)| {
            let payload = to_allocvec(&event).map_err(|_| {
                RuntimeError::TraceEncodeFailed {
                    name: "trace".to_string(),
                }
                .boxed()
            })?;

            Ok(TraceRecord {
                sequence,
                kind: encode_trace_event_kind(&event),
                payload: Some(binding.store_array(payload)),
            })
        })
        .collect::<RuntimeResult<Vec<_>>>()?;

    Ok(binding.store_array(encoded))
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
pub(crate) unsafe fn destack_runtime_agent_close(
    binding: &BindingCallContext,
    argument_agent: AgentHandle,
) -> RuntimeResult<()> {
    binding.clear_values();

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
pub(crate) unsafe fn destack_runtime_agent_create(
    binding: &BindingCallContext,
    out: *mut AgentHandle,
    runtimehandle: RuntimeHandle,
    options: Option<AgentCreateOptions>,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;

    // decode create options
    let options = options.unwrap_or(AgentCreateOptions {
        name: None,
        labels: None,
    });
    let mut runtime_options = RuntimeOptions::default();
    runtime_options.primary_agent.name = unsafe { decode_runtime_name(options.name)? };
    runtime_options.primary_agent.labels = unsafe { decode_runtime_labels(options.labels)? };

    // clear call-local output storage
    binding.clear_values();

    // spawn one agent in the live runtime
    let mut table = control_table().write();
    let entry = table.runtime_entry(handle::decode_runtime_handle(runtimehandle))?;
    let world = table.world(entry.world_handle_id)?;
    let runtime_id = entry.runtime_id;
    let agent_id =
        world.spawn_agent_with_options(runtime_id, &runtime_options, empty_vm_engine()?)?;
    let handle = handle::encode_agent_handle(table.register_agent(
        entry.world_handle_id,
        world,
        runtime_id,
        agent_id,
    ));

    unsafe { out.write(handle) };

    Ok(())
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
pub(crate) unsafe fn destack_runtime_agent_describe(
    binding: &BindingCallContext,
    out: *mut AgentDescriptor,
    argument_agent: AgentHandle,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve one live agent descriptor
    let table = control_table().read();
    let (world, runtime_id, agent_id) = table.agent(handle::decode_agent_handle(argument_agent))?;
    let descriptor = world.with_runtime(runtime_id, |runtime| {
        let agent = runtime.agent(agent_id).ok_or_else(|| {
            RuntimeError::AgentNotFound {
                agent_id: agent_id.0,
            }
            .boxed()
        })?;

        encode_agent_descriptor_for_live(binding, &world, agent)
    })?;

    unsafe { out.write(descriptor) };

    Ok(())
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
pub(crate) unsafe fn destack_runtime_runtime_close(
    binding: &BindingCallContext,
    argument_runtime: RuntimeHandle,
) -> RuntimeResult<()> {
    binding.clear_values();

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
pub(crate) unsafe fn destack_runtime_runtime_create(
    binding: &BindingCallContext,
    out: *mut RuntimeHandle,
    argument_world: WorldHandle,
    options: Option<RuntimeCreateOptions>,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;

    // decode create options
    let options = options.unwrap_or(RuntimeCreateOptions {
        name: None,
        labels: None,
    });
    let runtime_options = RuntimeOptions {
        name: unsafe { decode_runtime_name(options.name)? },
        labels: unsafe { decode_runtime_labels(options.labels)? },
        ..RuntimeOptions::default()
    };

    // clear call-local output storage
    binding.clear_values();

    // spawn one runtime in the live world
    let mut table = control_table().write();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let runtime_id =
        world.spawn_runtime(Vec::<String>::new(), &runtime_options, empty_vm_engine()?)?;
    let handle = handle::encode_runtime_handle(table.register_runtime(
        handle::decode_world_handle(argument_world),
        world,
        runtime_id,
    ));

    unsafe { out.write(handle) };

    Ok(())
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
pub(crate) unsafe fn destack_runtime_runtime_describe(
    binding: &BindingCallContext,
    out: *mut RuntimeDescriptor,
    argument_runtime: RuntimeHandle,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve one live runtime descriptor
    let table = control_table().read();
    let (world, runtime_id) = table.runtime(handle::decode_runtime_handle(argument_runtime))?;
    let descriptor = world.with_runtime(runtime_id, |runtime| {
        encode_runtime_descriptor_for_live(binding, &world, runtime)
    })?;

    unsafe { out.write(descriptor) };

    Ok(())
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
pub(crate) unsafe fn destack_runtime_world_close(
    binding: &BindingCallContext,
    argument_world: WorldHandle,
) -> RuntimeResult<()> {
    binding.clear_values();

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
pub(crate) unsafe fn destack_runtime_world_create(
    binding: &BindingCallContext,
    out: *mut WorldHandle,
    options: Option<WorldCreateOptions>,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;

    // decode labels
    let options = options.unwrap_or(WorldCreateOptions {
        engine: None,
        execution: None,
        world: None,
        labels: None,
    });
    let labels = unsafe { decode_runtime_labels(options.labels)? };
    let runtime_options = runtime_options_from_world_create(options);

    // clear call-local output storage
    binding.clear_values();

    let world = World::from_options(&runtime_options)?;
    let mut table = control_table().write();
    let handle = handle::encode_world_handle(table.register_world(world, labels));

    unsafe {
        *out = handle;
    }

    Ok(())
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
pub(crate) unsafe fn destack_runtime_world_describe(
    binding: &BindingCallContext,
    out: *mut WorldDescriptor,
    argument_world: WorldHandle,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve live world state
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let labels = table.world_labels(handle::decode_world_handle(argument_world))?;
    let descriptor = encode_world_descriptor(binding, argument_world, &world, labels)?;

    unsafe {
        *out = descriptor;
    }

    Ok(())
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
pub(crate) unsafe fn destack_runtime_world_tick(
    binding: &BindingCallContext,
    out: *mut RuntimeTickOutcome,
    argument_world: WorldHandle,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // tick one live world
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let outcome = match world.tick()? {
        TickOutcome::Idle => RuntimeTickOutcome::Idle,
        TickOutcome::Progressed => RuntimeTickOutcome::Progressed,
        TickOutcome::AdvancedTime => RuntimeTickOutcome::AdvancedTime,
    };

    unsafe {
        *out = outcome;
    }

    Ok(())
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
pub(crate) unsafe fn destack_runtime_world_view_close(
    binding: &BindingCallContext,
    view: WorldViewHandle,
) -> RuntimeResult<()> {
    binding.clear_values();

    // drop the external pinned view handle
    let mut table = control_table().write();
    table.close_world_view(handle::decode_world_view_handle(view))?;

    Ok(())
}

/// Open one pinned world view.
/// Create one read-consistent world view over one live world or one pinned revision.
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
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let options = options.unwrap_or(WorldViewOptions { revision_id: None });
    let revision_id = options
        .revision_id
        .map(decode_revision_id)
        .unwrap_or_else(|| world.revision_id());
    world.revision_info(revision_id)?;

    drop(table);

    // register the pinned view
    let mut table = control_table().write();
    let handle = handle::encode_world_view_handle(
        table.open_world_view(handle::decode_world_handle(argument_world), revision_id)?,
    );
    unsafe { out.write(handle) };

    Ok(())
}

/// Describe the world visible through one pinned world view.
/// Return one world descriptor as observed through one world view.
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
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let descriptor = encode_world_descriptor_for_revision(
        binding,
        world_view.world_handle,
        world_view.revision,
        &world_view.image,
        world_view.labels,
    )?;

    unsafe { out.write(descriptor) };

    Ok(())
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
pub(crate) unsafe fn destack_runtime_revision_view(
    binding: &BindingCallContext,
    out: *mut RevisionDescriptor,
    view: WorldViewHandle,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve one pinned revision
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let descriptor = encode_revision_descriptor(world_view.revision, &world_view.image)?;

    unsafe { out.write(descriptor) };

    Ok(())
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
pub(crate) unsafe fn destack_runtime_image_view(
    binding: &BindingCallContext,
    out: *mut ImageDescriptor,
    view: WorldViewHandle,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve one pinned image
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let descriptor = encode_image_descriptor(&world_view.world, &world_view.image)?;

    unsafe { out.write(descriptor) };

    Ok(())
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
pub(crate) unsafe fn destack_runtime_trace_view(
    binding: &BindingCallContext,
    out: *mut TraceDescriptor,
    view: WorldViewHandle,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve one pinned trace position
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let descriptor = TraceDescriptor {
        branch_id: encode_branch_id(world_view.revision.branch_id)?,
        sequence: TraceSequence(world_view.revision.sequence.get()),
    };

    unsafe { out.write(descriptor) };

    Ok(())
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
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let filter = unsafe { decode_runtime_filter(filter)? };
    let after = after.map(decode_runtime_id);
    let limit = limit.filter(|limit| *limit > 0).map(|limit| limit as usize);

    // clear call-local output storage
    binding.clear_values();

    let runtimes = list_runtimes(&world_view.image, &filter, after, limit)?;
    let mut descriptors = Vec::with_capacity(runtimes.len());

    for runtime in runtimes {
        descriptors.push(encode_runtime_descriptor_for_image(
            binding,
            &world_view.image,
            runtime,
        )?);
    }

    unsafe { out.write(binding.store_array(descriptors)) };

    Ok(())
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
pub(crate) unsafe fn destack_runtime_runtime_view(
    binding: &BindingCallContext,
    out: *mut RuntimeDescriptor,
    view: WorldViewHandle,
    runtime_id: RuntimeId,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve one pinned runtime
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let runtime = runtime_in_image(&world_view.image, decode_runtime_id(runtime_id))?;
    let descriptor = encode_runtime_descriptor_for_image(binding, &world_view.image, runtime)?;

    unsafe { out.write(descriptor) };

    Ok(())
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
pub(crate) unsafe fn destack_runtime_agent_list(
    binding: &BindingCallContext,
    out: *mut NativeArray<AgentDescriptor>,
    view: WorldViewHandle,
    filter: Option<AgentFilter>,
    after: Option<AgentId>,
    limit: Option<u32>,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;

    // enumerate pinned agents in stable order
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let filter = unsafe { decode_agent_filter(filter)? };
    let after = after.map(decode_agent_id);
    let limit = limit.filter(|limit| *limit > 0).map(|limit| limit as usize);

    // clear call-local output storage
    binding.clear_values();

    let agents = list_agents(&world_view.image, &filter, after, limit)?;
    let mut descriptors = Vec::with_capacity(agents.len());

    for agent in agents {
        descriptors.push(encode_agent_descriptor_for_image(
            binding,
            &world_view.image,
            agent,
        )?);
    }

    unsafe { out.write(binding.store_array(descriptors)) };

    Ok(())
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
pub(crate) unsafe fn destack_runtime_agent_view(
    binding: &BindingCallContext,
    out: *mut AgentDescriptor,
    view: WorldViewHandle,
    agent_id: AgentId,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve one pinned agent
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let agent = agent_in_image(&world_view.image, decode_agent_id(agent_id))?;
    let descriptor = encode_agent_descriptor_for_image(binding, &world_view.image, agent)?;

    unsafe { out.write(descriptor) };

    Ok(())
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
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let filter = unsafe { decode_resource_filter(filter)? };
    let after = after.map(decode_world_resource_id).transpose()?;
    let limit = limit.filter(|limit| *limit > 0).map(|limit| limit as usize);

    // clear call-local output storage
    binding.clear_values();

    let resources = list_resources(&world_view.image, &filter, after, limit);
    let mut descriptors = Vec::with_capacity(resources.len());

    for resource in resources {
        descriptors.push(encode_resource_descriptor(binding, resource));
    }

    unsafe { out.write(binding.store_array(descriptors)) };

    Ok(())
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
pub(crate) unsafe fn destack_runtime_resource_view(
    binding: &BindingCallContext,
    out: *mut ResourceDescriptor,
    view: WorldViewHandle,
    resource_id: WorldResourceId,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve one pinned logical resource
    let resource_id = decode_world_resource_id(resource_id)?;
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let resource = resource_in_image(&world_view.image, resource_id)?;

    unsafe { out.write(encode_resource_descriptor(binding, resource)) };

    Ok(())
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
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let filter = unsafe { decode_entity_filter(filter)? };
    let after = unsafe { decode_topology_entity_id_value(after)? };
    let limit = limit.filter(|limit| *limit > 0).map(|limit| limit as usize);

    // clear call-local output storage
    binding.clear_values();

    let entities = list_entities(&world_view.image, &filter, after.as_deref(), limit);
    let mut descriptors = Vec::with_capacity(entities.len());

    for entity in entities {
        descriptors.push(encode_entity_descriptor(binding, entity));
    }

    unsafe { out.write(binding.store_array(descriptors)) };

    Ok(())
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
pub(crate) unsafe fn destack_runtime_entity_view(
    binding: &BindingCallContext,
    out: *mut TopologyEntityDescriptor,
    view: WorldViewHandle,
    entity_id: TopologyEntityId,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;

    // resolve one pinned topology entity
    let entity_id = unsafe { entity_id.0.as_str()? };
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let entity = entity_in_image(&world_view.image, entity_id)?;

    // clear call-local output storage
    binding.clear_values();

    unsafe { out.write(encode_entity_descriptor(binding, entity)) };

    Ok(())
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
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let filter = unsafe { decode_edge_filter(filter)? };
    let after = unsafe { decode_topology_edge_id_value(after)? };
    let limit = limit.filter(|limit| *limit > 0).map(|limit| limit as usize);

    // clear call-local output storage
    binding.clear_values();

    let edges = list_edges(&world_view.image, &filter, after.as_deref(), limit);
    let mut descriptors = Vec::with_capacity(edges.len());

    for edge in edges {
        descriptors.push(encode_edge_descriptor(binding, edge));
    }

    unsafe { out.write(binding.store_array(descriptors)) };

    Ok(())
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
pub(crate) unsafe fn destack_runtime_edge_view(
    binding: &BindingCallContext,
    out: *mut TopologyEdgeDescriptor,
    view: WorldViewHandle,
    edge_id: TopologyEdgeId,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;

    // resolve one pinned topology edge
    let edge_id = unsafe { edge_id.0.as_str()? };
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let edge = edge_in_image(&world_view.image, edge_id)?;

    // clear call-local output storage
    binding.clear_values();

    unsafe { out.write(encode_edge_descriptor(binding, edge)) };

    Ok(())
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
pub(crate) unsafe fn destack_runtime_event_loop_view(
    binding: &BindingCallContext,
    out: *mut EventLoopDescriptor,
    view: WorldViewHandle,
    agent_id: AgentId,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve one pinned agent event loop
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let agent = agent_in_image(&world_view.image, decode_agent_id(agent_id))?;
    let descriptor = encode_event_loop_descriptor(&agent.event_loop)?;

    unsafe { out.write(descriptor) };

    Ok(())
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
pub(crate) unsafe fn destack_runtime_heap_view(
    binding: &BindingCallContext,
    out: *mut HeapDescriptor,
    view: WorldViewHandle,
    agent_id: AgentId,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve one pinned agent heap
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let agent = agent_in_image(&world_view.image, decode_agent_id(agent_id))?;
    let descriptor = encode_heap_descriptor(agent)?;

    unsafe { out.write(descriptor) };

    Ok(())
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
pub(crate) unsafe fn destack_runtime_engine_view(
    binding: &BindingCallContext,
    out: *mut EngineDescriptor,
    view: WorldViewHandle,
    agent_id: AgentId,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // resolve one pinned agent engine
    let table = control_table().read();
    let world_view =
        handle::encode_world_view_entry(table.world_view(handle::decode_world_view_handle(view))?);
    let agent = agent_in_image(&world_view.image, decode_agent_id(agent_id))?;
    let descriptor = encode_engine_descriptor(agent)?;

    unsafe { out.write(descriptor) };

    Ok(())
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
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let branch = world.branch_info(decode_branch_id(branchid))?;
    let descriptor = encode_branch_descriptor(binding, branch)?;

    unsafe { out.write(descriptor) };

    Ok(())
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
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let filter = unsafe { decode_branch_filter(filter)? };
    let limit = if limit.unwrap_or(0) == 0 {
        usize::MAX
    } else {
        limit.unwrap_or(0) as usize
    };
    let after = after.map(|value| value.0).unwrap_or(0);

    // clear call-local output storage
    binding.clear_values();

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
        .map(|branch| branch.and_then(|branch| encode_branch_descriptor(binding, branch)))
        .collect::<RuntimeResult<Vec<_>>>()?;

    unsafe { out.write(binding.store_array(descriptors)) };

    Ok(())
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
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let name = unsafe { decode_runtime_name(name)? };
    let labels = unsafe { decode_runtime_labels(labels)? };
    let checkpoint_name = name.as_deref().unwrap_or("checkpoint");

    // clear call-local output storage
    binding.clear_values();

    // capture the checkpoint through world lineage
    let checkpoint_id = world.checkpoint(checkpoint_name)?;

    // carry through low-level labels until checkpoint metadata grows a direct API
    if !labels.is_empty() {
        world.set_checkpoint_labels(checkpoint_id, labels)?;
    }

    unsafe { out.write(encode_checkpoint_id(checkpoint_id)?) };

    Ok(())
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
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let checkpoint = world.checkpoint_info(decode_checkpoint_id(checkpointid))?;
    let descriptor = encode_checkpoint_descriptor(binding, checkpoint)?;

    unsafe { out.write(descriptor) };

    Ok(())
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
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let filter = unsafe { decode_checkpoint_filter(filter)? };
    let limit = if limit.unwrap_or(0) == 0 {
        usize::MAX
    } else {
        limit.unwrap_or(0) as usize
    };
    let after = after.map(|value| value.0).unwrap_or(0);

    // clear call-local output storage
    binding.clear_values();

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
            checkpoint.and_then(|checkpoint| encode_checkpoint_descriptor(binding, checkpoint))
        })
        .collect::<RuntimeResult<Vec<_>>>()?;

    unsafe { out.write(binding.store_array(descriptors)) };

    Ok(())
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
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let revision_id = world.suspend()?;
    let revision = world.revision_info(revision_id)?;

    unsafe { out.write(encode_image_id(revision.image_id)?) };

    Ok(())
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
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let image = world.image_info(decode_image_id(imageid))?;
    let descriptor = encode_image_descriptor(&world, &image)?;

    unsafe { out.write(descriptor) };

    Ok(())
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

    unsafe { out.write(binding.store_array(descriptors)) };

    Ok(())
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
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let revision = world.revision_info(decode_revision_id(revisionid))?;
    let image = world.image_info(revision.image_id)?;
    let descriptor = encode_revision_descriptor(revision, &image)?;

    unsafe { out.write(descriptor) };

    Ok(())
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

    unsafe { out.write(binding.store_array(descriptors)) };

    Ok(())
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
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;

    unsafe { out.write(encode_branch_id(world.branch_id())?) };

    Ok(())
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
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let revision_id = decode_revision_id(revisionid);
    let name = unsafe { decode_runtime_name(name)? };
    let labels = unsafe { decode_runtime_labels(labels)? };
    let branch_name = name.as_deref().unwrap_or("fork");

    // clear call-local output storage
    binding.clear_values();

    // fork one child world from one explicit revision
    let child = world.fork_revision(revision_id, branch_name)?;

    // carry branch labels through the child lineage until branch create grows them directly
    if !labels.is_empty() {
        child.set_branch_labels(child.branch_id(), labels)?;
    }

    drop(table);

    // register the child world
    let mut table = control_table().write();
    let child_handle = handle::encode_world_handle(table.register_world(child, BTreeMap::new()));
    unsafe { out.write(child_handle) };

    Ok(())
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
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;

    unsafe { out.write(encode_revision_id(world.revision_id())?) };

    Ok(())
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
pub(crate) unsafe fn destack_runtime_world_rewind_checkpoint(
    binding: &BindingCallContext,
    argument_world: WorldHandle,
    checkpointid: CheckpointId,
) -> RuntimeResult<()> {
    binding.clear_values();

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
pub(crate) unsafe fn destack_runtime_world_rewind_revision(
    binding: &BindingCallContext,
    argument_world: WorldHandle,
    revisionid: RevisionId,
) -> RuntimeResult<()> {
    binding.clear_values();

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
pub(crate) unsafe fn destack_runtime_observation_close(
    binding: &BindingCallContext,
    handle: ObservationHandle,
) -> RuntimeResult<()> {
    binding.clear_values();

    // drop the external observe handle first
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
pub(crate) unsafe fn destack_runtime_observation_next(
    binding: &BindingCallContext,
    out: *mut NativeArray<ObservationRecord>,
    handle: ObservationHandle,
    limit: Option<u32>,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // read the next batch from the live subscription
    let table = control_table().read();
    let (world, subscription_id) = table.observation(handle::decode_observation_handle(handle))?;
    let limit = if limit.unwrap_or(0) == 0 {
        usize::MAX
    } else {
        limit.unwrap_or(0) as usize
    };
    let records = world.observation().next(subscription_id, limit)?;
    let records = encode_observation_records(binding, records)?;

    unsafe {
        *out = records;
    }

    Ok(())
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
pub(crate) unsafe fn destack_runtime_observation_open(
    binding: &BindingCallContext,
    out: *mut ObservationHandle,
    argument_world: WorldHandle,
    options: Option<ObservationOptions>,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

    // open one live observation subscription
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let options = options.unwrap_or(ObservationOptions {
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
    let handle =
        handle::encode_observation_handle(table.open_observation_handle(
            handle::decode_world_handle(argument_world),
            subscription_id,
        )?);

    unsafe {
        *out = handle;
    }

    Ok(())
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
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let snapshot = world.snapshot(decode_image_id(imageid))?;
    let bytes = Arc::<[u8]>::from(snapshot.encode()?);

    drop(table);

    // store the exported snapshot
    let mut table = control_table().write();
    let snapshot_id = handle::encode_snapshot_id(table.store_snapshot_handle(
        handle::decode_world_handle(argument_world),
        handle::decode_snapshot_format(format),
        snapshot,
        bytes,
    )?);

    unsafe { out.write(snapshot_id) };

    Ok(())
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
pub(crate) unsafe fn destack_runtime_snapshot_describe(
    binding: &BindingCallContext,
    out: *mut SnapshotDescriptor,
    argument_world: WorldHandle,
    snapshotid: SnapshotId,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

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
    let descriptor = encode_snapshot_descriptor(snapshotid, &entry)?;

    unsafe { out.write(descriptor) };

    Ok(())
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
pub(crate) unsafe fn destack_runtime_snapshot_import(
    binding: &BindingCallContext,
    out: *mut SnapshotId,
    argument_world: WorldHandle,
    argument_payload: NativeArray<u8>,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;

    // resolve the live world before storing the imported snapshot
    let table = control_table().read();
    let _world = table.world(handle::decode_world_handle(argument_world))?;

    // decode one stored snapshot payload
    let payload = unsafe { argument_payload.as_slice()? }.to_vec();
    let snapshot = WorldSnapshot::decode(&payload)?;
    let bytes = Arc::<[u8]>::from(payload);

    // clear call-local output storage
    binding.clear_values();
    drop(table);

    // store the imported snapshot
    let snapshot_id = handle::encode_snapshot_id(control_table().write().store_snapshot_handle(
        handle::decode_world_handle(argument_world),
        handle::decode_snapshot_format(SnapshotFormat::Portable),
        snapshot,
        bytes,
    )?);

    unsafe { out.write(snapshot_id) };

    Ok(())
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
        .map(|(snapshot_id, entry)| encode_snapshot_descriptor(snapshot_id, &entry))
        .collect::<RuntimeResult<Vec<_>>>()?;

    unsafe { out.write(binding.store_array(descriptors)) };

    Ok(())
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
pub(crate) unsafe fn destack_runtime_snapshot_read(
    binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    argument_world: WorldHandle,
    snapshotid: SnapshotId,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    binding.clear_values();

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

    unsafe { out.write(binding.store_array(entry.bytes.as_ref().to_vec())) };

    Ok(())
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
pub(crate) unsafe fn destack_runtime_restore_image(
    binding: &BindingCallContext,
    argument_world: WorldHandle,
    imageid: ImageId,
) -> RuntimeResult<()> {
    binding.clear_values();

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
pub(crate) unsafe fn destack_runtime_restore_snapshot(
    binding: &BindingCallContext,
    argument_world: WorldHandle,
    snapshotid: SnapshotId,
) -> RuntimeResult<()> {
    binding.clear_values();

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
pub(crate) unsafe fn destack_runtime_trace_close(
    binding: &BindingCallContext,
    cursor: TraceCursorHandle,
) -> RuntimeResult<()> {
    binding.clear_values();

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
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let descriptor = TraceDescriptor {
        branch_id: encode_branch_id(world.branch_id())?,
        sequence: TraceSequence(world.trace().log().next_sequence().get()),
    };

    unsafe { out.write(descriptor) };

    Ok(())
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
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    drop(table);
    let sequence = world.trace().record_marker(label)?;

    unsafe { out.write(TraceSequence(sequence.get())) };

    Ok(())
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

        records.push((TraceSequence(sequence.get()), event));
    }

    unsafe { out.write(encode_trace_records(binding, records)?) };

    Ok(())
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
    let table = control_table().read();
    let world = table.world(handle::decode_world_handle(argument_world))?;
    let cursor = Arc::new(world.trace().log().reader());
    let options = options.unwrap_or(TraceCursorOptions {
        start_sequence: None,
    });
    if let Some(start_sequence) = options.start_sequence {
        cursor.seek_sequence(RuntimeTraceSequence::new(start_sequence.0))?;
    }

    drop(table);

    // register the trace cursor
    let mut table = control_table().write();
    let handle = handle::encode_trace_cursor_handle(
        table.open_trace_cursor_handle(handle::decode_world_handle(argument_world), cursor)?,
    );
    unsafe { out.write(handle) };

    Ok(())
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
pub(crate) unsafe fn destack_runtime_trace_seek_checkpoint(
    binding: &BindingCallContext,
    cursor: TraceCursorHandle,
    checkpointid: CheckpointId,
) -> RuntimeResult<()> {
    binding.clear_values();

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
pub(crate) unsafe fn destack_runtime_trace_seek_revision(
    binding: &BindingCallContext,
    cursor: TraceCursorHandle,
    revisionid: RevisionId,
) -> RuntimeResult<()> {
    binding.clear_values();

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
pub(crate) unsafe fn destack_runtime_trace_seek_sequence(
    binding: &BindingCallContext,
    cursor: TraceCursorHandle,
    sequence: TraceSequence,
) -> RuntimeResult<()> {
    binding.clear_values();

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
    let table = control_table().read();
    let (_world, cursor) = table.trace_cursor(handle::decode_trace_cursor_handle(cursor))?;
    let sequence = cursor.tell();
    let sequence = TraceSequence(sequence.get());

    unsafe { out.write(sequence) };

    Ok(())
}
