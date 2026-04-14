use std::collections::BTreeMap;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::runtime::{
    AgentDescriptorValue, BranchDescriptorValue, CheckpointDescriptorValue, EngineDescriptor,
    EngineDescriptorKind, EventLoopDescriptor, HeapDescriptor, ImageDescriptor,
    ObservationEventKind, ObservationRecordValue, ResourceDescriptorValue, RevisionDescriptor,
    RuntimeDescriptorValue, RuntimeLabelValue, SnapshotDescriptor, SnapshotId, TopologyEdgeIdValue,
    TopologyEdgeKindValue, TopologyEdgeValue, TopologyEntityIdValue, TopologyEntityKindValue,
    TopologyEntityValue, TraceEventKind, TraceRecordValue, TraceSequence, WorldDescriptorValue,
    WorldHandle,
};
use crate::runtime;
use crate::runtime::control::{ObservationEntry, SnapshotEntry, WorldViewEntry};
use crate::runtime::engine::EngineImage;
use crate::runtime::scheduler::EventLoopSnapshot;
use crate::runtime::trace::{Outcome, TraceRecord};
use crate::runtime::world::{
    Image, ObservationCategory, ObservationRecord, Revision, World, WorldEdge, WorldEntity,
};
use postcard::to_allocvec;

use super::{ObservationHandleEntry, PinnedWorldView, RuntimeHandleCodec, SnapshotHandleEntry};

/// Descriptor and record building for low-level runtime bindings.
pub(crate) struct RuntimeDescriptorCodec;

impl RuntimeDescriptorCodec {
    /// Convert one optional runtime name into one owned binding value.
    pub(crate) fn owned_name(name: &str) -> Option<String> {
        if name.is_empty() {
            return None;
        }

        Some(name.to_string())
    }

    /// Convert runtime labels into one owned binding value.
    pub(crate) fn owned_labels(labels: BTreeMap<String, String>) -> Option<Vec<RuntimeLabelValue>> {
        if labels.is_empty() {
            return None;
        }

        Some(
            labels
                .into_iter()
                .map(|(key, value)| RuntimeLabelValue { key, value })
                .collect(),
        )
    }

    /// Encode one runtime trace record into one low-level event kind.
    pub(crate) fn encode_trace_event_kind(event: &TraceRecord) -> TraceEventKind {
        match event {
            TraceRecord::Outcome(Outcome::Entropy(_)) => TraceEventKind::Entropy,
            TraceRecord::Outcome(Outcome::BindingCall(_)) => TraceEventKind::BindingCall,
            TraceRecord::Input(_)
            | TraceRecord::Outcome(Outcome::TimeAdvance(_))
            | TraceRecord::Outcome(Outcome::RuntimeSpawned { .. })
            | TraceRecord::Outcome(Outcome::AgentSpawned { .. }) => TraceEventKind::Control,
            TraceRecord::Anchor(_) => TraceEventKind::Marker,
        }
    }

    /// Encode one observation control-table entry for the low-level binding surface.
    pub(crate) fn observation_entry(entry: ObservationEntry) -> ObservationHandleEntry {
        ObservationHandleEntry {
            world: RuntimeHandleCodec::encode_world_handle(entry.world_handle_id),
            subscription_id: entry.subscription_id,
        }
    }

    /// Encode one snapshot control-table entry for the low-level binding surface.
    pub(crate) fn snapshot_entry(entry: SnapshotEntry) -> SnapshotHandleEntry {
        SnapshotHandleEntry {
            world: RuntimeHandleCodec::encode_world_handle(entry.world_handle_id),
            format: RuntimeHandleCodec::encode_snapshot_format(entry.format),
            snapshot: entry.snapshot,
            bytes: entry.bytes,
        }
    }

    /// Encode one pinned world-view entry for the low-level binding surface.
    pub(crate) fn world_view_entry(entry: WorldViewEntry) -> PinnedWorldView {
        PinnedWorldView {
            world_handle: RuntimeHandleCodec::encode_world_handle(entry.world_handle_id),
            labels: entry.labels.labels,
            revision: entry.revision,
            image: entry.image,
        }
    }

    /// Build one owned branch descriptor from runtime state.
    pub(crate) fn branch_descriptor(
        branch: runtime::world::Branch,
    ) -> RuntimeResult<BranchDescriptorValue> {
        Ok(BranchDescriptorValue {
            id: RuntimeHandleCodec::encode_branch_id(branch.id)?,
            head_revision: RuntimeHandleCodec::encode_revision_id(branch.head_revision_id)?,
            name: Self::owned_name(&branch.name),
            labels: Self::owned_labels(branch.labels),
        })
    }

    /// Build one owned revision descriptor from runtime state.
    pub(crate) fn revision_descriptor(
        revision: Revision,
        image: &Image,
    ) -> RuntimeResult<RevisionDescriptor> {
        Ok(RevisionDescriptor {
            id: RuntimeHandleCodec::encode_revision_id(revision.id)?,
            branch_id: RuntimeHandleCodec::encode_branch_id(revision.branch_id)?,
            parent_revision: revision
                .parent_revision_id
                .map(RuntimeHandleCodec::encode_revision_id)
                .transpose()?,
            sequence: TraceSequence(revision.sequence.get()),
            wall_ns: revision.wall.get(),
            mono_ns: revision.mono.get(),
            virtual_ns: image.clock.virtual_wall.get(),
            image_id: RuntimeHandleCodec::encode_image_id(revision.image_id)?,
        })
    }

    /// Build one owned runtime descriptor from one captured runtime.
    pub(crate) fn runtime_descriptor_for_image(
        image: &Image,
        runtime: &runtime::RuntimeImage,
    ) -> RuntimeResult<RuntimeDescriptorValue> {
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

        Ok(RuntimeDescriptorValue {
            id: RuntimeHandleCodec::encode_runtime_id(runtime.runtime_id)?,
            primary_agent_id: RuntimeHandleCodec::encode_agent_id(runtime.primary_agent_id)?,
            name: Self::owned_name(&runtime.name),
            agent_count,
            labels: Self::owned_labels(labels.clone()),
        })
    }

    /// Build one owned agent descriptor from one captured agent.
    pub(crate) fn agent_descriptor_for_image(
        image: &Image,
        agent: &runtime::AgentImage,
    ) -> RuntimeResult<AgentDescriptorValue> {
        let labels = image.agent_labels(agent.agent_id)?;
        let resource_count = u32::try_from(agent.resources.entries.len()).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "resourceCount",
                "agent resource count exceeds uint32",
            ))
            .boxed()
        })?;

        Ok(AgentDescriptorValue {
            id: RuntimeHandleCodec::encode_agent_id(agent.agent_id)?,
            runtime_id: RuntimeHandleCodec::encode_runtime_id(agent.runtime_id)?,
            name: Self::owned_name(&agent.name),
            has_pending_work: agent.has_pending_work(),
            resource_count,
            labels: Self::owned_labels(labels.clone()),
        })
    }

    /// Build one owned runtime descriptor from one live runtime.
    pub(crate) fn runtime_descriptor_for_live(
        world: &World,
        runtime: &runtime::Runtime,
    ) -> RuntimeResult<RuntimeDescriptorValue> {
        let labels = world.runtime_labels(runtime.runtime_id())?;
        let agent_count = u32::try_from(runtime.agent_count()).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "agentCount",
                "runtime agent count exceeds uint32",
            ))
            .boxed()
        })?;

        Ok(RuntimeDescriptorValue {
            id: RuntimeHandleCodec::encode_runtime_id(runtime.runtime_id())?,
            primary_agent_id: RuntimeHandleCodec::encode_agent_id(runtime.primary_agent_id())?,
            name: Self::owned_name(runtime.name()),
            agent_count,
            labels: Self::owned_labels(labels),
        })
    }

    /// Build one owned agent descriptor from one live agent.
    pub(crate) fn agent_descriptor_for_live(
        world: &World,
        agent: &runtime::Agent,
    ) -> RuntimeResult<AgentDescriptorValue> {
        let labels = world.agent_labels(agent.agent_id())?;
        let resource_count = u32::try_from(agent.resource_count()).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "resourceCount",
                "agent resource count exceeds uint32",
            ))
            .boxed()
        })?;

        Ok(AgentDescriptorValue {
            id: RuntimeHandleCodec::encode_agent_id(agent.agent_id())?,
            runtime_id: RuntimeHandleCodec::encode_runtime_id(agent.runtime_id())?,
            name: Self::owned_name(agent.name()),
            has_pending_work: agent.has_pending_work(),
            resource_count,
            labels: Self::owned_labels(labels),
        })
    }

    /// Build one owned topology entity descriptor from runtime state.
    pub(crate) fn entity_descriptor(entity: &WorldEntity) -> TopologyEntityValue {
        TopologyEntityValue {
            id: TopologyEntityIdValue(entity.id.to_string()),
            kind: TopologyEntityKindValue(entity.kind.to_string()),
            labels: Self::owned_labels(entity.labels.clone()),
        }
    }

    /// Build one owned topology edge descriptor from runtime state.
    pub(crate) fn edge_descriptor(edge: &WorldEdge) -> TopologyEdgeValue {
        TopologyEdgeValue {
            id: TopologyEdgeIdValue(edge.id.to_string()),
            kind: TopologyEdgeKindValue(edge.kind.to_string()),
            from: TopologyEntityIdValue(edge.from.to_string()),
            to: TopologyEntityIdValue(edge.to.to_string()),
            labels: Self::owned_labels(edge.labels.clone()),
        }
    }

    /// Build one owned resource descriptor from runtime state.
    pub(crate) fn resource_descriptor(
        resource: &runtime::world::WorldResource,
    ) -> ResourceDescriptorValue {
        ResourceDescriptorValue {
            id: RuntimeHandleCodec::encode_world_resource_id(resource.id),
            entity_id: TopologyEntityIdValue(resource.id.entity_id().to_string()),
            kind: resource.kind.to_string(),
            label: resource.label.clone(),
        }
    }

    /// Build one owned event-loop descriptor from one captured scheduler image.
    pub(crate) fn event_loop_descriptor(
        snapshot: &EventLoopSnapshot,
    ) -> RuntimeResult<EventLoopDescriptor> {
        let task_count = u32::try_from(snapshot.tasks.len()).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "taskCount",
                "event loop task count exceeds uint32",
            ))
            .boxed()
        })?;
        let microtask_count = u32::try_from(snapshot.microtasks.len()).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "microtaskCount",
                "event loop microtask count exceeds uint32",
            ))
            .boxed()
        })?;
        let timer_count = u32::try_from(snapshot.ready_timers.len() + snapshot.timers.len())
            .map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "timerCount",
                    "event loop timer count exceeds uint32",
                ))
                .boxed()
            })?;
        let watch_count = u32::try_from(
            snapshot.timer_watches.len()
                + snapshot.poller_event_watches.len()
                + snapshot.host_event_watches.len(),
        )
        .map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "watchCount",
                "event loop watch count exceeds uint32",
            ))
            .boxed()
        })?;

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

    /// Build one owned heap descriptor from one captured agent image.
    pub(crate) fn heap_descriptor(agent: &runtime::AgentImage) -> RuntimeResult<HeapDescriptor> {
        let page_count = u32::try_from(agent.heap_image.page_count()).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "pageCount",
                "heap page count exceeds uint32",
            ))
            .boxed()
        })?;
        let shared_page_count = u32::try_from(agent.heap_image.raw_page_count()).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "sharedPageCount",
                "shared heap page count exceeds uint32",
            ))
            .boxed()
        })?;

        Ok(HeapDescriptor {
            heap_bytes: agent.heap_image.local_allocated_bytes(),
            page_count,
            shared_page_count,
            gc_cycles: agent.heap_image.managed_gc_state().cycles,
        })
    }

    /// Build one owned engine descriptor from one captured agent image.
    pub(crate) fn engine_descriptor(
        agent: &runtime::AgentImage,
    ) -> RuntimeResult<EngineDescriptor> {
        match &agent.engine_image {
            EngineImage::Vm(image) => {
                let call_stack_depth =
                    u32::try_from(image.interpreter.call_stack.len()).map_err(|_| {
                        RuntimeError::from(PlatformError::invalid_argument_value(
                            "callStackDepth",
                            "engine call stack depth exceeds uint32",
                        ))
                        .boxed()
                    })?;
                let value_stack_depth = u32::try_from(image.interpreter.value_stack.len())
                    .map_err(|_| {
                        RuntimeError::from(PlatformError::invalid_argument_value(
                            "valueStackDepth",
                            "engine value stack depth exceeds uint32",
                        ))
                        .boxed()
                    })?;
                let local_stack_depth = u32::try_from(image.interpreter.local_stack.len())
                    .map_err(|_| {
                        RuntimeError::from(PlatformError::invalid_argument_value(
                            "localStackDepth",
                            "engine local stack depth exceeds uint32",
                        ))
                        .boxed()
                    })?;

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

    /// Build one owned checkpoint descriptor from runtime state.
    pub(crate) fn checkpoint_descriptor(
        checkpoint: runtime::world::Checkpoint,
    ) -> RuntimeResult<CheckpointDescriptorValue> {
        Ok(CheckpointDescriptorValue {
            id: RuntimeHandleCodec::encode_checkpoint_id(checkpoint.id)?,
            revision_id: RuntimeHandleCodec::encode_revision_id(checkpoint.revision_id)?,
            name: Self::owned_name(&checkpoint.name),
            labels: Self::owned_labels(checkpoint.labels),
        })
    }

    /// Build one owned image descriptor from runtime state.
    pub(crate) fn image_descriptor(
        revision_id: runtime::world::RevisionId,
        image: &Image,
    ) -> RuntimeResult<ImageDescriptor> {
        let (shared_bytes, _) = World::image_size_and_hash(image)?;

        Ok(ImageDescriptor {
            id: RuntimeHandleCodec::encode_image_id(image.id)?,
            revision_id: RuntimeHandleCodec::encode_revision_id(revision_id)?,
            shared_bytes: Some(shared_bytes),
        })
    }

    /// Build one owned snapshot descriptor from one stored snapshot entry.
    pub(crate) fn snapshot_descriptor(
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

        let image = entry.snapshot.image()?;

        Ok(SnapshotDescriptor {
            id: snapshot_id,
            image_id: RuntimeHandleCodec::encode_image_id(image.id)?,
            format: entry.format,
            size_bytes: Some(size_bytes),
        })
    }

    /// Build one owned world descriptor from one live world.
    pub(crate) fn world_descriptor(
        handle: WorldHandle,
        world: &World,
        labels: BTreeMap<String, String>,
    ) -> RuntimeResult<WorldDescriptorValue> {
        Ok(WorldDescriptorValue {
            handle,
            branch_id: RuntimeHandleCodec::encode_branch_id(world.branch_id())?,
            revision_id: RuntimeHandleCodec::encode_revision_id(world.revision_id())?,
            wall_ns: world.wall().get(),
            mono_ns: world.mono().get(),
            virtual_ns: world.clock().virtual_wall().get(),
            runtime_count: u32::try_from(world.runtime_ids().len()).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "runtimeCount",
                    "world runtime count exceeds uint32",
                ))
                .boxed()
            })?,
            labels: Self::owned_labels(labels),
        })
    }

    /// Build one owned world descriptor from one pinned revision.
    pub(crate) fn world_descriptor_for_revision(
        handle: WorldHandle,
        revision: Revision,
        image: &Image,
        labels: BTreeMap<String, String>,
    ) -> RuntimeResult<WorldDescriptorValue> {
        Ok(WorldDescriptorValue {
            handle,
            branch_id: RuntimeHandleCodec::encode_branch_id(revision.branch_id)?,
            revision_id: RuntimeHandleCodec::encode_revision_id(revision.id)?,
            wall_ns: revision.wall.get(),
            mono_ns: revision.mono.get(),
            virtual_ns: image.clock.virtual_wall.get(),
            runtime_count: u32::try_from(image.runtimes.len()).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "runtimeCount",
                    "world runtime count exceeds uint32",
                ))
                .boxed()
            })?,
            labels: Self::owned_labels(labels),
        })
    }

    /// Encode one observation category into the low-level observation enum.
    pub(crate) fn observation_kind(category: ObservationCategory) -> ObservationEventKind {
        match category {
            ObservationCategory::Runtime => ObservationEventKind::Trace,
            ObservationCategory::Topology => ObservationEventKind::Topology,
            ObservationCategory::Resource => ObservationEventKind::Resource,
            ObservationCategory::Scheduler => ObservationEventKind::Scheduler,
            ObservationCategory::Diagnostic => ObservationEventKind::Diagnostic,
            ObservationCategory::Telemetry => ObservationEventKind::Profile,
            ObservationCategory::Domain => ObservationEventKind::Trace,
        }
    }

    /// Build owned observation records from world observation events.
    pub(crate) fn observation_records(
        records: Vec<ObservationRecord>,
    ) -> RuntimeResult<Vec<ObservationRecordValue>> {
        records
            .into_iter()
            .map(|record| {
                let payload = to_allocvec(&record.observation).map_err(|_| {
                    RuntimeError::from(PlatformError::io("failed to encode observation payload"))
                        .boxed()
                })?;

                Ok(ObservationRecordValue {
                    kind: Self::observation_kind(record.observation.category),
                    sequence: Some(TraceSequence(record.sequence.get())),
                    payload: Some(payload),
                })
            })
            .collect()
    }

    /// Build owned trace records from runtime trace records.
    pub(crate) fn trace_records(
        records: Vec<(runtime::trace::TraceSequence, TraceRecord)>,
    ) -> RuntimeResult<Vec<TraceRecordValue>> {
        records
            .into_iter()
            .map(|(sequence, event)| {
                let payload = to_allocvec(&event).map_err(|_| {
                    RuntimeError::TraceEncodeFailed {
                        name: "trace".to_string(),
                    }
                    .boxed()
                })?;

                Ok(TraceRecordValue {
                    sequence: TraceSequence(sequence.get()),
                    kind: Self::encode_trace_event_kind(&event),
                    payload: Some(payload),
                })
            })
            .collect()
    }
}
