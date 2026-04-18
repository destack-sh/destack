use std::sync::Arc;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::resource::ResourceId;
use crate::platform::runtime::{
    BranchId, CheckpointId, ImageId, ObservationHandle, RevisionId, RuntimeHandle, RuntimeId,
    SnapshotFormat, SnapshotId, TraceCursorHandle, WorkerHandle, WorkerId, WorldHandle,
    WorldResourceId, WorldResourceIdVm, WorldViewHandle,
};
use crate::runtime::control::{ControlHandleId, ControlSnapshotFormat};
use crate::runtime::observe::ObservationSubscriptionId;
use crate::runtime::world::{
    BranchId as WorldBranchId, CheckpointId as WorldCheckpointId, ImageId as WorldImageId,
    Revision as WorldRevision, WorldResourceId as LogicalWorldResourceId, WorldSnapshot,
};
use crate::runtime::{RuntimeId as WorldRuntimeId, WorkerId as WorldWorkerId};

/// One observation handle exposed through the low-level runtime binding surface.
#[derive(Debug, Clone)]
pub(crate) struct ObservationHandleEntry {
    /// The owning world handle.
    pub world: WorldHandle,
    /// The observation subscription id.
    pub subscription_id: ObservationSubscriptionId,
}

/// One snapshot handle exposed through the low-level runtime binding surface.
#[derive(Debug, Clone)]
pub(crate) struct SnapshotHandleEntry {
    /// The owning world handle.
    pub world: WorldHandle,
    /// The requested snapshot format.
    pub format: SnapshotFormat,
    /// The stored snapshot payload.
    pub snapshot: WorldSnapshot,
    /// The encoded snapshot bytes.
    pub bytes: Arc<[u8]>,
}

/// Mechanical handle and identifier conversion for low-level runtime bindings.
pub(crate) struct RuntimeHandleCodec;

impl RuntimeHandleCodec {
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

    /// Decode one worker handle into its control-table id.
    pub(crate) fn decode_worker_handle(handle: WorkerHandle) -> ControlHandleId {
        ControlHandleId::new(handle.0.0)
    }

    /// Encode one worker control-table id as one low-level worker handle.
    pub(crate) fn encode_worker_handle(handle_id: ControlHandleId) -> WorkerHandle {
        WorkerHandle(ResourceId(handle_id.get()))
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
        let branch_id = u64::try_from(branch_id.get()).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "branchId",
                "world branch id exceeds uint64",
            ))
            .boxed()
        })?;

        Ok(BranchId(branch_id))
    }

    /// Decode one low-level branch id into one world branch id.
    pub(crate) fn decode_branch_id(branch_id: BranchId) -> WorldBranchId {
        WorldBranchId::new(u128::from(branch_id.0))
    }

    /// Encode one world revision id as one low-level revision id.
    pub(crate) fn encode_revision_id(revision: WorldRevision) -> RuntimeResult<RevisionId> {
        let revision_id = u64::try_from(revision.get()).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "revisionId",
                "world revision id exceeds uint64",
            ))
            .boxed()
        })?;

        Ok(RevisionId(revision_id))
    }

    /// Decode one low-level revision id into one world revision id.
    pub(crate) fn decode_revision_id(revision_id: RevisionId) -> WorldRevision {
        WorldRevision::new(u128::from(revision_id.0))
    }

    /// Encode one world checkpoint id as one low-level checkpoint id.
    pub(crate) fn encode_checkpoint_id(
        checkpoint_id: WorldCheckpointId,
    ) -> RuntimeResult<CheckpointId> {
        let checkpoint_id = u64::try_from(checkpoint_id.get()).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "checkpointId",
                "world checkpoint id exceeds uint64",
            ))
            .boxed()
        })?;

        Ok(CheckpointId(checkpoint_id))
    }

    /// Decode one low-level checkpoint id into one world checkpoint id.
    pub(crate) fn decode_checkpoint_id(checkpoint_id: CheckpointId) -> WorldCheckpointId {
        WorldCheckpointId::new(u128::from(checkpoint_id.0))
    }

    /// Encode one world image id as one low-level image id.
    pub(crate) fn encode_image_id(image_id: WorldImageId) -> RuntimeResult<ImageId> {
        let image_id = u64::try_from(image_id.get()).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "imageId",
                "world image id exceeds uint64",
            ))
            .boxed()
        })?;

        Ok(ImageId(image_id))
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

    /// Encode one world worker id as one low-level worker id.
    pub(crate) fn encode_worker_id(worker_id: WorldWorkerId) -> RuntimeResult<WorkerId> {
        Ok(WorkerId(worker_id.0))
    }

    /// Decode one low-level worker id into one world worker id.
    pub(crate) fn decode_worker_id(worker_id: WorkerId) -> WorldWorkerId {
        WorldWorkerId(worker_id.0)
    }

    /// Decode one low-level world resource id into one logical world resource id.
    pub(crate) fn decode_world_resource_id(
        resource_id: WorldResourceId,
    ) -> RuntimeResult<LogicalWorldResourceId> {
        Ok(LogicalWorldResourceId::new(
            Self::decode_worker_id(resource_id.worker_id),
            resource_id.resource_id,
        ))
    }

    /// Decode one VM world resource id into one logical world resource id.
    pub(crate) fn decode_world_resource_id_vm(
        resource_id: WorldResourceIdVm,
    ) -> RuntimeResult<LogicalWorldResourceId> {
        Ok(LogicalWorldResourceId::new(
            Self::decode_worker_id(resource_id.worker_id),
            resource_id.resource_id,
        ))
    }

    /// Encode one logical world resource id into the low-level binding type.
    pub(crate) fn encode_world_resource_id(resource_id: LogicalWorldResourceId) -> WorldResourceId {
        WorldResourceId {
            worker_id: WorkerId(resource_id.worker_id.0),
            resource_id: resource_id.resource_id,
        }
    }
}
