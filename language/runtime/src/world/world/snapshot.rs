use tspp_core::Blob;
use tspp_program as program;
use tspp_repository::{ExecutionMode, ReplayPayloadMode, WorldOptions};
use tspp_serde as serde;
use tspp_serde::Reflect;

use ::serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::lineage::{
    Image, ImageEntry, ImageId, Lineage, LineageSnapshot, Revision, RevisionId,
};
use crate::world::topology::LabelSet;

use super::{RestoreContext, WORLD_SNAPSHOT_FORMAT_VERSION, World, WorldImage};

/// Serialized World image stored as one Blob.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Snapshot {
    /// Encoded snapshot payload.
    pub blob: Blob,
}

/// Serialized snapshot for one world image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    /// The snapshot format version.
    pub format_version: u32,
    /// World options for the restored World.
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
            format_version: WORLD_SNAPSHOT_FORMAT_VERSION,
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

    /// Return the selected captured World image.
    pub fn world_image(&self) -> RuntimeResult<&WorldImage> {
        let revision = self.revision()?;

        self.lineage
            .images
            .get(&revision.image_id)
            .map(|entry| entry.world_image.as_ref())
            .ok_or_else(|| {
                RuntimeError::revision_image_missing(
                    self.revision_id.get(),
                    revision.image_id.get(),
                )
                .boxed()
            })
    }

    /// Iterate over Programs retained by every World image in this Snapshot.
    pub fn programs(&self) -> impl Iterator<Item = &program::Program> {
        self.lineage
            .images
            .values()
            .flat_map(|entry| entry.world_image.runtimes().values())
            .map(|runtime| runtime.program.as_ref())
    }

    /// Encode one snapshot into bytes.
    pub fn encode(&self) -> RuntimeResult<Vec<u8>> {
        serde::to_vec(self).map_err(|_| {
            RuntimeError::inconsistent_image("failed to encode world snapshot".to_string()).boxed()
        })
    }

    /// Decode one snapshot from bytes.
    pub fn decode(bytes: &[u8]) -> RuntimeResult<Self> {
        let snapshot: Self = serde::from_slice(bytes).map_err(|_| {
            RuntimeError::inconsistent_image("failed to decode world snapshot".to_string()).boxed()
        })?;

        // reject snapshots with incompatible runtime structure
        if snapshot.format_version != WORLD_SNAPSHOT_FORMAT_VERSION {
            return Err(RuntimeError::inconsistent_image(format!(
                "unsupported world snapshot format version {}",
                snapshot.format_version
            ))
            .boxed());
        }

        Ok(snapshot)
    }
}

/// Options needed to rebuild one World from a snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotOptions {
    /// Execution mode for the rebuilt world.
    pub mode: ExecutionMode,
    /// Replay payload policy used for the trace.
    pub replay_payload: ReplayPayloadMode,
}

impl World {
    /// Build snapshot restore options from the live world.
    fn snapshot_options(&self) -> SnapshotOptions {
        let header = self.state.trace.store().header();
        SnapshotOptions {
            mode: self.state.trace.mode(),
            replay_payload: header.replay_payload.into(),
        }
    }

    /// Build world options for one serialized world snapshot.
    fn world_options_from_snapshot(snapshot: &WorldSnapshot) -> WorldOptions {
        WorldOptions {
            mode: snapshot.options.mode,
            replay_payload: snapshot.options.replay_payload,
            ..Default::default()
        }
    }

    /// Return the revision that owns one stored image.
    pub fn revision_for_image(&self, image_id: ImageId) -> RuntimeResult<RevisionId> {
        let lineage = self.lineage.read();
        lineage.image_revision(image_id)
    }

    /// Create one lineage-wide serialized snapshot from one stored image.
    pub fn snapshot_lineage(&self, image_id: ImageId) -> RuntimeResult<WorldSnapshot> {
        let (revision, lineage_snapshot) = {
            let revision = self.revision_for_image(image_id)?;
            let lineage = self.lineage.read();

            (revision, lineage.full_snapshot())
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
        restore: RestoreContext<'_>,
    ) -> RuntimeResult<()> {
        // resolve the owning revision first
        let (revision, image, trace_image) = {
            let revision = self.revision_for_image(image_id)?;
            let (_, image, trace_image) = self.revision_data(revision)?;

            (revision, image, trace_image)
        };

        self.restore_revision_image(revision, &image, &trace_image, restore)
    }

    /// Create one exact serialized snapshot for one stored image.
    pub fn snapshot(&self, image_id: ImageId) -> RuntimeResult<WorldSnapshot> {
        let revision = self.revision_for_image(image_id)?;

        self.snapshot_revision(revision, RestoreContext::empty())
    }

    /// Create one lineage-wide serialized snapshot from one specific revision.
    pub fn snapshot_lineage_revision(
        &self,
        revision_id: RevisionId,
        restore: RestoreContext<'_>,
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
        let image = self.revision_image(
            &target_revision,
            &base_revision,
            &image,
            &trace_image,
            restore,
        )?;
        let mut lineage_snapshot = self.lineage.read().full_snapshot();
        lineage_snapshot.images.insert(
            revision.image_id,
            ImageEntry::new(
                Image {
                    id: revision.image_id,
                    moment: revision.moment(),
                    name: None,
                    labels: LabelSet::new(),
                },
                revision_id,
                image.into(),
            ),
        );

        Ok(WorldSnapshot::new(
            self.snapshot_options(),
            revision_id,
            lineage_snapshot,
        ))
    }

    /// Create one exact serialized snapshot from one specific revision.
    pub fn snapshot_revision(
        &self,
        revision_id: RevisionId,
        restore: RestoreContext<'_>,
    ) -> RuntimeResult<WorldSnapshot> {
        let (image, trace_image) = {
            let lineage = self.lineage.read();
            let revision = lineage.revision(revision_id)?;

            if lineage.contains_image(revision.image_id) {
                let image = lineage.world_image(revision.image_id)?;
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
                    restore,
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

    /// Create one lineage-wide serialized Snapshot from one retained Image.
    pub fn snapshot_lineage_image(&self, image_id: ImageId) -> RuntimeResult<WorldSnapshot> {
        let revision = self.revision_for_image(image_id)?;

        self.snapshot_lineage_revision(revision, RestoreContext::empty())
    }

    /// Create one exact serialized Snapshot from one retained Image.
    pub fn snapshot_image(&self, image_id: ImageId) -> RuntimeResult<WorldSnapshot> {
        let revision = self.revision_for_image(image_id)?;

        self.snapshot_revision(revision, RestoreContext::empty())
    }

    /// Build one fresh world from one serialized snapshot.
    pub fn from_snapshot(
        snapshot: &WorldSnapshot,
        restore: RestoreContext<'_>,
    ) -> RuntimeResult<Self> {
        let options = Self::world_options_from_snapshot(snapshot);
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
        world.restore_snapshot(snapshot, restore)?;

        Ok(world)
    }

    /// Build one fresh world from one encoded snapshot payload.
    pub fn from_snapshot_bytes(bytes: &[u8], restore: RestoreContext<'_>) -> RuntimeResult<Self> {
        let snapshot = WorldSnapshot::decode(bytes)?;

        Self::from_snapshot(&snapshot, restore)
    }

    /// Restore one serialized snapshot into this world.
    pub fn restore_snapshot(
        &mut self,
        snapshot: &WorldSnapshot,
        restore: RestoreContext<'_>,
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

        self.restore_revision_image(snapshot.revision_id, &image, &trace_image, restore)
    }
}
