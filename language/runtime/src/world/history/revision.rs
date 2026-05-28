use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::diagnostic::{Entity, EntityError, RuntimeError, RuntimeResult};
use crate::runtime::time::Instant;
use crate::world::trace::{TraceImage, TraceSequence};
use crate::world::{World, WorldImage};

use super::{BranchId, ImageId};

/// Revision metadata for one world history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Revision {
    /// The branch that owns this revision.
    pub branch_id: BranchId,
    /// The parent revision in this branch history.
    pub parent_revision_id: Option<RevisionId>,
    /// The trace sequence captured by this revision.
    pub sequence: TraceSequence,
    /// The captured world image for this revision.
    pub image_id: ImageId,
    /// The wall-clock instant captured by this revision.
    pub wall: Instant,
    /// The monotonic instant captured by this revision.
    pub mono: Instant,
    /// The revision labels.
    pub labels: BTreeMap<String, String>,
}

/// Revision identifier for one world history.
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

impl World {
    /// Return the active branch revision identifier for this world.
    pub fn revision_id(&self) -> RevisionId {
        let history = self.history.read();
        let branch = history
            .branches
            .get(&self.state.branch_id)
            .expect("world history must contain the active branch");

        branch.head_revision_id
    }

    /// Return metadata for the active branch revision.
    pub fn current_revision(&self) -> Revision {
        let history = self.history.read();
        let revision = history
            .revisions
            .get(&self.revision_id())
            .expect("world history must contain the active revision");

        revision.clone()
    }

    /// Return metadata for one specific revision.
    pub fn revision(&self, revision_id: RevisionId) -> RuntimeResult<Revision> {
        let history = self.history.read();
        let revision = history
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
            let history = self.history.read();
            history.revision(revision_id)?
        };

        let history = self.history.read();
        let image = history
            .image(revision.image_id)
            .map_err(|error| match error.as_ref() {
                RuntimeError::Entity {
                    reason: EntityError::NotFound(Entity::Image { .. }),
                } => {
                    RuntimeError::revision_image_missing(revision_id.get(), revision.image_id.get())
                        .boxed()
                }
                _ => error,
            })?;
        let trace_image = history.trace_image(revision_id)?;

        Ok((revision, image, trace_image))
    }

    /// Return the nearest retained base revision for one target revision.
    pub(crate) fn nearest_image_revision(
        &self,
        revision_id: RevisionId,
    ) -> RuntimeResult<RevisionId> {
        let history = self.history.read();

        history.nearest_image_revision(revision_id, |image_id| history.contains_image(image_id))
    }

    /// Return one retained trace image by revision identifier.
    pub(crate) fn trace_image(&self, revision_id: RevisionId) -> RuntimeResult<Arc<TraceImage>> {
        let history = self.history.read();

        history.trace_image(revision_id)
    }

    /// Return identifiers for all known revisions in stable order.
    pub fn revisions(&self) -> Vec<RevisionId> {
        self.history.read().revisions.keys().copied().collect()
    }
}
