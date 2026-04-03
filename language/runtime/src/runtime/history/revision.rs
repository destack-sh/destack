use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::time::WorldInstant;
use crate::runtime::trace::{TraceImage, TraceSequence};
use crate::runtime::world::World;

use super::{BranchId, Image, ImageId};

/// Revision identifier for one world lineage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RevisionId(u128);

impl RevisionId {
    /// Create a new revision identifier.
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    /// Return the raw revision identifier value.
    pub const fn get(self) -> u128 {
        self.0
    }
}

/// Revision metadata for one world lineage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Revision {
    /// The revision identifier.
    pub id: RevisionId,
    /// The branch that owns this revision.
    pub branch_id: BranchId,
    /// The parent revision in this branch lineage.
    pub parent_revision_id: Option<RevisionId>,
    /// The trace sequence captured by this revision.
    pub sequence: TraceSequence,
    /// The captured world image for this revision.
    pub image_id: ImageId,
    /// The wall-clock instant captured by this revision.
    pub wall: WorldInstant,
    /// The monotonic instant captured by this revision.
    pub mono: WorldInstant,
    /// The revision labels.
    pub labels: BTreeMap<String, String>,
}

impl World {
    /// Return the active branch revision identifier for this world.
    pub fn revision_id(&self) -> RevisionId {
        let lineage = self.lineage.read();
        let branch = lineage
            .branches
            .get(&self.branch_id)
            .expect("world lineage must contain the active branch");

        branch.head_revision_id
    }

    /// Return the active branch revision metadata for this world.
    pub fn revision(&self) -> Revision {
        let lineage = self.lineage.read();
        let revision = lineage
            .revisions
            .get(&self.revision_id())
            .expect("world lineage must contain the active revision");

        revision.clone()
    }

    /// Return metadata for one specific revision.
    pub fn revision_info(&self, revision_id: RevisionId) -> RuntimeResult<Revision> {
        let lineage = self.lineage.read();
        let revision = lineage.revisions.get(&revision_id).ok_or_else(|| {
            RuntimeError::RevisionNotFound {
                revision_id: revision_id.get(),
            }
            .boxed()
        })?;

        Ok(revision.clone())
    }

    /// Resolve one revision and its retained data.
    pub(crate) fn revision_data(
        &self,
        revision_id: RevisionId,
    ) -> RuntimeResult<(Revision, Arc<Image>, Arc<TraceImage>)> {
        let revision = {
            let lineage = self.lineage.read();
            lineage.revision(revision_id)?
        };

        let images = self.images.read();
        let image = images
            .image(revision.image_id)
            .map_err(|error| match *error {
                RuntimeError::ImageNotFound { .. } => RuntimeError::RevisionImageMissing {
                    revision_id: revision.id.get(),
                    image_id: revision.image_id.get(),
                }
                .boxed(),
                _ => error,
            })?;
        let trace_image = images.trace_image(revision.id)?;

        Ok((revision, image, trace_image))
    }

    /// Return the nearest retained base revision for one target revision.
    pub(crate) fn nearest_image_revision_id(
        &self,
        revision_id: RevisionId,
    ) -> RuntimeResult<RevisionId> {
        let lineage = self.lineage.read();
        let images = self.images.read();

        lineage.nearest_image_revision_id(revision_id, |image_id| images.contains_image(image_id))
    }

    /// Return one retained trace image by revision identifier.
    pub(crate) fn trace_image(&self, revision_id: RevisionId) -> RuntimeResult<Arc<TraceImage>> {
        self.images.read().trace_image(revision_id)
    }

    /// Return identifiers for all known revisions in stable order.
    pub fn revision_ids(&self) -> Vec<RevisionId> {
        self.lineage.read().revisions.keys().copied().collect()
    }
}
