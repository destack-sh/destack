use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::diagnostic::{Entity, EntityError, RuntimeError, RuntimeResult};
use crate::world::time::Instant;
use crate::world::topology::LabelSet;
use crate::world::trace::{TraceImage, TraceSequence};
use crate::world::{Moment, World, WorldImage};

use super::{BranchId, ImageId, MomentSequence};

/// Revision metadata for one world lineage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Revision {
    /// The branch that owns this revision.
    pub branch_id: BranchId,
    /// The parent revision in this branch lineage.
    pub parent_revision_id: Option<RevisionId>,
    /// The moment sequence captured by this revision.
    pub sequence: MomentSequence,
    /// The trace sequence captured by this revision.
    pub trace_sequence: TraceSequence,
    /// The captured world image for this revision.
    pub image_id: ImageId,
    /// The wall-clock instant captured by this revision.
    pub wall: Instant,
    /// The monotonic instant captured by this revision.
    pub mono: Instant,
    /// The revision labels.
    pub labels: LabelSet,
}

impl Revision {
    /// Return the moment captured by this revision.
    pub const fn moment(&self) -> Moment {
        Moment::new(self.branch_id, self.sequence)
    }
}

/// Revision identifier for one world lineage.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct RevisionId(u64);

impl RevisionId {
    /// Create a new revision identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw revision identifier value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl World {
    /// Return the active branch revision identifier for this world.
    pub fn revision_id(&self) -> RuntimeResult<RevisionId> {
        let lineage = self.lineage.read();
        let branch = lineage
            .branches
            .get(&self.state.branch_id)
            .ok_or_else(|| RuntimeError::branch_not_found(self.state.branch_id.get()).boxed())?;

        Ok(branch.head_revision_id)
    }

    /// Return metadata for the active branch revision.
    pub fn current_revision(&self) -> RuntimeResult<Revision> {
        let lineage = self.lineage.read();
        let branch = lineage
            .branches
            .get(&self.state.branch_id)
            .ok_or_else(|| RuntimeError::branch_not_found(self.state.branch_id.get()).boxed())?;
        let revision = lineage
            .revisions
            .get(&branch.head_revision_id)
            .ok_or_else(|| {
                RuntimeError::revision_not_found(branch.head_revision_id.get()).boxed()
            })?;

        Ok(revision.clone())
    }

    /// Return metadata for one specific revision.
    pub fn revision(&self, revision_id: RevisionId) -> RuntimeResult<Revision> {
        let lineage = self.lineage.read();
        let revision = lineage
            .revisions
            .get(&revision_id)
            .ok_or_else(|| RuntimeError::revision_not_found(revision_id.get()).boxed())?;

        Ok(revision.clone())
    }

    /// Resolve one revision and its retained data.
    pub(crate) fn revision_data(
        &self,
        revision_id: RevisionId,
    ) -> RuntimeResult<(Revision, Arc<WorldImage>, Arc<TraceImage>)> {
        let revision = {
            let lineage = self.lineage.read();
            lineage.revision(revision_id)?
        };

        let lineage = self.lineage.read();
        let image =
            lineage
                .world_image(revision.image_id)
                .map_err(|error| match error.as_ref() {
                    RuntimeError::Entity {
                        reason: EntityError::NotFound(Entity::Image { .. }),
                    } => RuntimeError::revision_image_missing(
                        revision_id.get(),
                        revision.image_id.get(),
                    )
                    .boxed(),
                    _ => error,
                })?;
        let trace_image = lineage.trace_image(revision_id)?;

        Ok((revision, image, trace_image))
    }

    /// Return the nearest retained base revision for one target revision.
    pub(crate) fn nearest_image_revision(
        &self,
        revision_id: RevisionId,
    ) -> RuntimeResult<RevisionId> {
        let lineage = self.lineage.read();

        lineage.nearest_image_revision(revision_id, |image_id| lineage.contains_image(image_id))
    }

    /// Return one retained trace image by revision identifier.
    pub(crate) fn trace_image(&self, revision_id: RevisionId) -> RuntimeResult<Arc<TraceImage>> {
        let lineage = self.lineage.read();

        lineage.trace_image(revision_id)
    }

    /// Return identifiers for all known revisions in stable order.
    pub fn revisions(&self) -> Vec<RevisionId> {
        self.lineage.read().revisions.keys().copied().collect()
    }
}
