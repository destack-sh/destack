use std::collections::BTreeMap;
use std::sync::Arc;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::resource::ResourceId;
use crate::platform::runtime::{
    AgentHandle, AgentId, BranchId, CheckpointId, ImageId, ObservationHandle, RevisionId,
    RuntimeHandle, RuntimeId, SnapshotFormat, SnapshotId, TraceCursorHandle, TraceEventKind,
    WorldHandle, WorldViewHandle,
};
use crate::runtime::AgentId as WorldAgentId;
use crate::runtime::control::{
    ControlHandleId, ControlSnapshotFormat, ObservationEntry, SnapshotEntry, WorldViewEntry,
};
use crate::runtime::replay::TraceEvent;
use crate::runtime::world::{
    BranchId as WorldBranchId, CheckpointId as WorldCheckpointId, Image, ImageId as WorldImageId,
    ObservationSubscriptionId, Revision, RevisionId as WorldRevisionId,
    RuntimeId as WorldRuntimeId, Snapshot, World,
};

/// One observation-handle entry resolved through the low-level runtime binding surface.
#[derive(Debug, Clone)]
pub(crate) struct ObservationHandleEntry {
    /// The owning world handle.
    pub world: WorldHandle,
    /// The observation subscription id.
    pub subscription_id: ObservationSubscriptionId,
}

/// One snapshot-handle entry resolved through the low-level runtime binding surface.
#[derive(Debug, Clone)]
pub(crate) struct SnapshotHandleEntry {
    /// The owning world handle.
    pub world: WorldHandle,
    /// The requested snapshot format.
    pub format: SnapshotFormat,
    /// The stored snapshot payload.
    pub snapshot: Snapshot,
    /// The encoded snapshot bytes.
    pub bytes: Arc<[u8]>,
}

/// One pinned world view resolved through the low-level runtime binding surface.
#[derive(Debug, Clone)]
pub(crate) struct PinnedWorldView {
    /// The owning world handle.
    pub world_handle: WorldHandle,
    /// The live world backing this view.
    pub world: Arc<World>,
    /// The stored world labels.
    pub labels: BTreeMap<String, String>,
    /// The pinned revision metadata.
    pub revision: Revision,
    /// The pinned world image.
    pub image: Arc<Image>,
}

/// Decode one world handle into its control-table id.
pub(crate) fn decode_world_handle(handle: WorldHandle) -> ControlHandleId {
    ControlHandleId::new(handle.0.0)
}

/// Encode one world control-table id as one low-level world handle.
pub(crate) fn encode_world_handle(handle_id: ControlHandleId) -> WorldHandle {
    WorldHandle(ResourceId(handle_id.get()))
}

/// Decode one runtime handle into its control-table id.
pub(crate) fn decode_runtime_handle(handle: RuntimeHandle) -> ControlHandleId {
    ControlHandleId::new(handle.0.0)
}

/// Encode one runtime control-table id as one low-level runtime handle.
pub(crate) fn encode_runtime_handle(handle_id: ControlHandleId) -> RuntimeHandle {
    RuntimeHandle(ResourceId(handle_id.get()))
}

/// Decode one agent handle into its control-table id.
pub(crate) fn decode_agent_handle(handle: AgentHandle) -> ControlHandleId {
    ControlHandleId::new(handle.0.0)
}

/// Encode one agent control-table id as one low-level agent handle.
pub(crate) fn encode_agent_handle(handle_id: ControlHandleId) -> AgentHandle {
    AgentHandle(ResourceId(handle_id.get()))
}

/// Decode one observation handle into its control-table id.
pub(crate) fn decode_observation_handle(handle: ObservationHandle) -> ControlHandleId {
    ControlHandleId::new(handle.0.0)
}

/// Encode one observation control-table id as one low-level observation handle.
pub(crate) fn encode_observation_handle(handle_id: ControlHandleId) -> ObservationHandle {
    ObservationHandle(ResourceId(handle_id.get()))
}

/// Decode one trace cursor handle into its control-table id.
pub(crate) fn decode_trace_cursor_handle(handle: TraceCursorHandle) -> ControlHandleId {
    ControlHandleId::new(handle.0.0)
}

/// Encode one trace cursor control-table id as one low-level trace cursor handle.
pub(crate) fn encode_trace_cursor_handle(handle_id: ControlHandleId) -> TraceCursorHandle {
    TraceCursorHandle(ResourceId(handle_id.get()))
}

/// Decode one pinned world-view handle into its control-table id.
pub(crate) fn decode_world_view_handle(handle: WorldViewHandle) -> ControlHandleId {
    ControlHandleId::new(handle.0.0)
}

/// Encode one control-table id as one pinned world-view handle.
pub(crate) fn encode_world_view_handle(handle_id: ControlHandleId) -> WorldViewHandle {
    WorldViewHandle(ResourceId(handle_id.get()))
}

/// Decode one snapshot id into its control-table id.
pub(crate) fn decode_snapshot_id(snapshot_id: SnapshotId) -> ControlHandleId {
    ControlHandleId::new(snapshot_id.0)
}

/// Encode one control-table id as one snapshot id.
pub(crate) fn encode_snapshot_id(handle_id: ControlHandleId) -> SnapshotId {
    SnapshotId(handle_id.get())
}

/// Decode one snapshot format into the runtime control-table representation.
pub(crate) fn decode_snapshot_format(format: SnapshotFormat) -> ControlSnapshotFormat {
    match format {
        SnapshotFormat::Fast => ControlSnapshotFormat::Fast,
        SnapshotFormat::Portable => ControlSnapshotFormat::Portable,
    }
}

/// Encode one control-table snapshot format into the low-level snapshot enum.
pub(crate) fn encode_snapshot_format(format: ControlSnapshotFormat) -> SnapshotFormat {
    match format {
        ControlSnapshotFormat::Fast => SnapshotFormat::Fast,
        ControlSnapshotFormat::Portable => SnapshotFormat::Portable,
    }
}

/// Encode one world branch id as one low-level branch id.
pub(crate) fn encode_branch_id(branch_id: WorldBranchId) -> RuntimeResult<BranchId> {
    Ok(BranchId(u64::try_from(branch_id.get()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "branchId",
            "world branch id exceeds uint64",
        ))
        .boxed()
    })?))
}

/// Decode one low-level branch id into one world branch id.
pub(crate) fn decode_branch_id(branch_id: BranchId) -> WorldBranchId {
    WorldBranchId::new(u128::from(branch_id.0))
}

/// Encode one world revision id as one low-level revision id.
pub(crate) fn encode_revision_id(revision_id: WorldRevisionId) -> RuntimeResult<RevisionId> {
    Ok(RevisionId(u64::try_from(revision_id.get()).map_err(
        |_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "revisionId",
                "world revision id exceeds uint64",
            ))
            .boxed()
        },
    )?))
}

/// Decode one low-level revision id into one world revision id.
pub(crate) fn decode_revision_id(revision_id: RevisionId) -> WorldRevisionId {
    WorldRevisionId::new(u128::from(revision_id.0))
}

/// Encode one world checkpoint id as one low-level checkpoint id.
pub(crate) fn encode_checkpoint_id(
    checkpoint_id: WorldCheckpointId,
) -> RuntimeResult<CheckpointId> {
    Ok(CheckpointId(u64::try_from(checkpoint_id.get()).map_err(
        |_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "checkpointId",
                "world checkpoint id exceeds uint64",
            ))
            .boxed()
        },
    )?))
}

/// Decode one low-level checkpoint id into one world checkpoint id.
pub(crate) fn decode_checkpoint_id(checkpoint_id: CheckpointId) -> WorldCheckpointId {
    WorldCheckpointId::new(u128::from(checkpoint_id.0))
}

/// Encode one world image id as one low-level image id.
pub(crate) fn encode_image_id(image_id: WorldImageId) -> RuntimeResult<ImageId> {
    Ok(ImageId(u64::try_from(image_id.get()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "imageId",
            "world image id exceeds uint64",
        ))
        .boxed()
    })?))
}

/// Decode one low-level image id into one world image id.
pub(crate) fn decode_image_id(image_id: ImageId) -> WorldImageId {
    WorldImageId::new(u128::from(image_id.0))
}

/// Encode one world runtime id as one low-level runtime id.
pub(crate) fn encode_runtime_id(runtime_id: WorldRuntimeId) -> RuntimeResult<RuntimeId> {
    Ok(RuntimeId(runtime_id.0))
}

/// Decode one low-level runtime id into one world runtime id.
pub(crate) fn decode_runtime_id(runtime_id: RuntimeId) -> WorldRuntimeId {
    WorldRuntimeId(runtime_id.0)
}

/// Encode one world agent id as one low-level agent id.
pub(crate) fn encode_agent_id(agent_id: WorldAgentId) -> RuntimeResult<AgentId> {
    Ok(AgentId(agent_id.0))
}

/// Decode one low-level agent id into one world agent id.
pub(crate) fn decode_agent_id(agent_id: AgentId) -> WorldAgentId {
    WorldAgentId(agent_id.0)
}

/// Encode one runtime trace event into one low-level event kind.
pub(crate) fn encode_trace_event_kind(event: &TraceEvent) -> TraceEventKind {
    match event {
        TraceEvent::Entropy(_) => TraceEventKind::Entropy,
        TraceEvent::BindingCall(_) => TraceEventKind::BindingCall,
        TraceEvent::Tick(_) | TraceEvent::WorldCommand(_) => TraceEventKind::Control,
        TraceEvent::Marker(_) => TraceEventKind::Marker,
    }
}

/// Encode one observation control-table entry for the low-level binding surface.
pub(crate) fn encode_observation_entry(entry: ObservationEntry) -> ObservationHandleEntry {
    ObservationHandleEntry {
        world: encode_world_handle(entry.world_handle_id),
        subscription_id: entry.subscription_id,
    }
}

/// Encode one snapshot control-table entry for the low-level binding surface.
pub(crate) fn encode_snapshot_entry(entry: SnapshotEntry) -> SnapshotHandleEntry {
    SnapshotHandleEntry {
        world: encode_world_handle(entry.world_handle_id),
        format: encode_snapshot_format(entry.format),
        snapshot: entry.snapshot,
        bytes: entry.bytes,
    }
}

/// Encode one pinned world-view entry for the low-level binding surface.
pub(crate) fn encode_world_view_entry(entry: WorldViewEntry) -> PinnedWorldView {
    PinnedWorldView {
        world_handle: encode_world_handle(entry.world_handle_id),
        world: entry.world,
        labels: entry.labels.labels,
        revision: entry.revision,
        image: entry.image,
    }
}
