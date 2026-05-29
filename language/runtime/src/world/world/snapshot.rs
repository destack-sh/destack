use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::binding::BindingReplayPayload;
use crate::host::resource::ResourceRebinders;
use crate::world::lineage::{
    CheckpointId, ImageId, Lineage, LineageSnapshot, Revision, RevisionId,
};
use destack_workspace::{ExecutionMode, ReplayPayloadMode, RuntimeOptions};
use postcard::to_allocvec;

use super::{World, WorldImage};

/// Serialized snapshot for one world image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    /// The snapshot format version.
    pub format_version: u32,
    /// Runtime options for the restored world.
    pub options: SnapshotOptions,
    /// The revision selected for restore from this snapshot.
    pub revision_id: RevisionId,
    /// The captured lineage metadata.
    pub lineage: LineageSnapshot,
}

impl WorldSnapshot {
    /// Create one serialized snapshot for one materialized revision.
    pub const fn new(
        options: SnapshotOptions,
        revision_id: RevisionId,
        lineage: LineageSnapshot,
    ) -> Self {
        Self {
            format_version: 1,
            options,
            revision_id,
            lineage,
        }
    }

    /// Return the captured revision metadata.
    pub fn revision(&self) -> RuntimeResult<&Revision> {
        self.lineage
            .revisions
            .get(&self.revision_id)
            .ok_or_else(|| RuntimeError::revision_not_found(self.revision_id.get()).boxed())
    }

    /// Return the captured image metadata.
    pub fn image(&self) -> RuntimeResult<&WorldImage> {
        let revision = self.revision()?;

        self.lineage.images.get(&revision.image_id).ok_or_else(|| {
            RuntimeError::revision_image_missing(self.revision_id.get(), revision.image_id.get())
                .boxed()
        })
    }

    /// Encode one snapshot into bytes.
    pub fn encode(&self) -> RuntimeResult<Vec<u8>> {
        to_allocvec(self).map_err(|_| {
            RuntimeError::inconsistent_image("failed to encode world snapshot".to_string()).boxed()
        })
    }

    /// Decode one snapshot from bytes.
    pub fn decode(bytes: &[u8]) -> RuntimeResult<Self> {
        postcard::from_bytes(bytes).map_err(|_| {
            RuntimeError::inconsistent_image("failed to decode world snapshot".to_string()).boxed()
        })
    }
}

/// Runtime options needed to rebuild one world from a snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotOptions {
    /// Execution mode for the rebuilt world.
    pub execution: ExecutionMode,
    /// Replay payload policy used for the trace.
    pub replay_payload: ReplayPayloadMode,
    /// Trace chunk sizing in megabytes, when configured.
    pub replay_chunk_size_mb: Option<u64>,
}

impl World {
    /// Build snapshot restore options from the live world.
    fn snapshot_options(&self) -> SnapshotOptions {
        let header = self.state.trace.log().header();
        let replay_payload = match header.replay_payload {
            BindingReplayPayload::Results => ReplayPayloadMode::ResultsOnly,
            BindingReplayPayload::ArgumentsAndResults => ReplayPayloadMode::ArgumentsAndResults,
        };
        let replay_chunk_size_mb = if header.max_chunk_size_bytes == 0 {
            None
        } else {
            Some(header.max_chunk_size_bytes / (1024 * 1024))
        };

        SnapshotOptions {
            execution: self.state.trace.mode(),
            replay_payload,
            replay_chunk_size_mb,
        }
    }

    /// Build runtime options for one serialized world snapshot.
    fn runtime_options_from_snapshot(snapshot: &WorldSnapshot) -> RuntimeOptions {
        let mut options = RuntimeOptions::default();

        options.execution.mode = snapshot.options.execution;
        options.trace.chunk_size_mb = snapshot.options.replay_chunk_size_mb;
        options.trace.payload = snapshot.options.replay_payload;

        options
    }

    /// Return metadata for one stored image.
    pub fn image_info(&self, image_id: ImageId) -> RuntimeResult<WorldImage> {
        let image = self.lineage.read().image(image_id)?;

        Ok(image.as_ref().clone())
    }

    /// Return identifiers for all stored images in stable order.
    pub fn image_ids(&self) -> Vec<ImageId> {
        self.lineage.read().image_ids()
    }

    /// Return the revision that owns one stored image.
    pub fn revision_for_image(&self, image_id: ImageId) -> RuntimeResult<RevisionId> {
        let lineage = self.lineage.read();
        lineage.image(image_id)?;
        let revision = lineage.revision_for_image_id(image_id).ok_or_else(|| {
            RuntimeError::inconsistent_image(format!(
                "image {} does not belong to one revision",
                image_id.get()
            ))
            .boxed()
        })?;

        Ok(revision)
    }

    /// Create one lineage-wide serialized snapshot from one stored image.
    pub fn snapshot_lineage(&self, image_id: ImageId) -> RuntimeResult<WorldSnapshot> {
        let (revision, lineage_snapshot) = {
            let revision = self.revision_for_image(image_id)?;
            let lineage = self.lineage.read();

            (revision, lineage.full_snapshot()?)
        };

        Ok(WorldSnapshot::new(
            self.snapshot_options(),
            revision,
            lineage_snapshot,
        ))
    }

    /// Restore one stored image into the active world.
    pub fn restore_image_id(
        &mut self,
        image_id: ImageId,
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<()> {
        // resolve the owning revision first
        let (revision, image, trace_image) = {
            let revision = self.revision_for_image(image_id)?;
            let (_, image, trace_image) = self.revision_data(revision)?;

            (revision, image, trace_image)
        };

        self.restore_revision_image(revision, &image, &trace_image, rebind_context)
    }

    /// Create one exact serialized snapshot for one stored image.
    pub fn snapshot(&self, image_id: ImageId) -> RuntimeResult<WorldSnapshot> {
        let revision = self.revision_for_image(image_id)?;

        self.snapshot_revision(revision)
    }

    /// Create one lineage-wide serialized snapshot from one specific revision.
    pub fn snapshot_lineage_revision(
        &self,
        revision_id: RevisionId,
    ) -> RuntimeResult<WorldSnapshot> {
        let revision = {
            let lineage = self.lineage.read();
            lineage.revision(revision_id)?
        };

        if self.lineage.read().contains_image(revision.image_id) {
            return self.snapshot_lineage(revision.image_id);
        }

        let (target_revision, base_revision, image, trace_image) = {
            let lineage = self.lineage.read();
            let target_revision = lineage.revision(revision_id)?;
            let base_revision = self.nearest_image_revision(revision_id)?;
            let (base_revision, image, _) = self.revision_data(base_revision)?;
            let trace_image = self.trace_image(revision_id)?;

            (target_revision, base_revision, image, trace_image)
        };
        let image =
            self.revision_image(&target_revision, &base_revision, &image, &trace_image, None)?;
        let mut lineage_snapshot = self.lineage.read().full_snapshot()?;
        lineage_snapshot.images.insert(revision.image_id, image);

        Ok(WorldSnapshot::new(
            self.snapshot_options(),
            revision_id,
            lineage_snapshot,
        ))
    }

    /// Create one exact serialized snapshot from one specific revision.
    pub fn snapshot_revision(&self, revision_id: RevisionId) -> RuntimeResult<WorldSnapshot> {
        let (image, trace_image) = {
            let lineage = self.lineage.read();
            let revision = lineage.revision(revision_id)?;

            if lineage.contains_image(revision.image_id) {
                let image = lineage.image(revision.image_id)?;
                let trace_image = lineage.trace_image(revision_id)?;

                (image.as_ref().clone(), trace_image.as_ref().clone())
            } else {
                drop(lineage);

                let (target_revision, base_revision, image, trace_image) = {
                    let lineage = self.lineage.read();
                    let target_revision = lineage.revision(revision_id)?;
                    let base_revision = self.nearest_image_revision(revision_id)?;
                    let (base_revision, image, _) = self.revision_data(base_revision)?;
                    let trace_image = self.trace_image(revision_id)?;

                    (target_revision, base_revision, image, trace_image)
                };
                let image = self.revision_image(
                    &target_revision,
                    &base_revision,
                    &image,
                    &trace_image,
                    None,
                )?;

                (image, trace_image.as_ref().clone())
            }
        };
        let lineage_snapshot =
            self.lineage
                .read()
                .exact_snapshot(revision_id, &image, &trace_image)?;

        Ok(WorldSnapshot::new(
            self.snapshot_options(),
            revision_id,
            lineage_snapshot,
        ))
    }

    /// Create one lineage-wide serialized snapshot from one stored checkpoint.
    pub fn snapshot_lineage_checkpoint(
        &self,
        checkpoint_id: CheckpointId,
    ) -> RuntimeResult<WorldSnapshot> {
        let revision = {
            let lineage = self.lineage.read();
            let checkpoint = lineage
                .checkpoints
                .get(&checkpoint_id)
                .ok_or_else(|| RuntimeError::checkpoint_not_found(checkpoint_id.get()).boxed())?;

            checkpoint.revision_id
        };

        self.snapshot_lineage_revision(revision)
    }

    /// Create one exact serialized snapshot from one stored checkpoint.
    pub fn snapshot_checkpoint(&self, checkpoint_id: CheckpointId) -> RuntimeResult<WorldSnapshot> {
        let revision = {
            let lineage = self.lineage.read();
            let checkpoint = lineage
                .checkpoints
                .get(&checkpoint_id)
                .ok_or_else(|| RuntimeError::checkpoint_not_found(checkpoint_id.get()).boxed())?;

            checkpoint.revision_id
        };

        self.snapshot_revision(revision)
    }

    /// Build one fresh world from one serialized snapshot.
    pub fn from_snapshot(
        snapshot: &WorldSnapshot,
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<Self> {
        let options = Self::runtime_options_from_snapshot(snapshot);
        let revision = snapshot.revision()?;
        let trace_image = snapshot
            .lineage
            .trace_images
            .get(&snapshot.revision_id)
            .ok_or_else(|| {
                RuntimeError::revision_trace_image_missing(snapshot.revision_id.get()).boxed()
            })?;
        let environment = trace_image.header().environment.clone();
        let mut world = Self::empty(revision.branch_id, &options, environment, None)?;
        world.restore_snapshot(snapshot, rebind_context)?;

        Ok(world)
    }

    /// Build one fresh world from one encoded snapshot payload.
    pub fn from_snapshot_bytes(
        bytes: &[u8],
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<Self> {
        let snapshot = WorldSnapshot::decode(bytes)?;

        Self::from_snapshot(&snapshot, rebind_context)
    }

    /// Restore one serialized snapshot into this world.
    pub fn restore_snapshot(
        &mut self,
        snapshot: &WorldSnapshot,
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<()> {
        let revision = snapshot.revision()?;

        if revision.branch_id != self.state.branch_id {
            return Err(RuntimeError::snapshot_branch_mismatch(
                revision.branch_id.get(),
                self.state.branch_id.get(),
            )
            .boxed());
        }

        *self.lineage.write() = Lineage::from_snapshot(snapshot.lineage.clone())?;
        let (image, trace_image) = {
            let (_, image, trace_image) = self.revision_data(snapshot.revision_id)?;

            (image, trace_image)
        };

        self.restore_revision_image(snapshot.revision_id, &image, &trace_image, rebind_context)
    }
}
