use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::time::Instant;
use crate::world::World;
use crate::world::trace::{TraceImage, TraceSequence};

use super::{BranchId, ImageId, TraceImageId, WorldImage};

/// Revision handle for one world lineage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Revision(u128);

impl Revision {
    /// Create a new revision handle.
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    /// Return the raw revision handle value.
    pub const fn get(self) -> u128 {
        self.0
    }
}

/// Revision state for one world lineage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionState {
    /// The branch that owns this revision.
    pub branch_id: BranchId,
    /// The parent revision in this branch lineage.
    pub parent_revision: Option<Revision>,
    /// The trace sequence captured by this revision.
    pub sequence: TraceSequence,
    /// The captured world image for this revision.
    pub image_id: ImageId,
    /// The captured trace image for this revision.
    pub trace_image_id: TraceImageId,
    /// The wall-clock instant captured by this revision.
    pub wall: Instant,
    /// The monotonic instant captured by this revision.
    pub mono: Instant,
    /// The revision labels.
    pub labels: BTreeMap<String, String>,
}

impl World {
    /// Return the active branch revision handle for this world.
    pub fn revision(&self) -> Revision {
        let lineage = self.lineage.read();
        let branch = lineage
            .branches
            .get(&self.state.branch_id)
            .expect("world lineage must contain the active branch");

        branch.head_revision
    }

    /// Return metadata for the active branch revision.
    pub fn current_revision_state(&self) -> RevisionState {
        let lineage = self.lineage.read();
        let revision = lineage
            .revisions
            .get(&self.revision())
            .expect("world lineage must contain the active revision");

        revision.clone()
    }

    /// Return metadata for one specific revision.
    pub fn revision_state(&self, revision: Revision) -> RuntimeResult<RevisionState> {
        let lineage = self.lineage.read();
        let revision = lineage.revisions.get(&revision).ok_or_else(|| {
            RuntimeError::RevisionNotFound {
                revision_id: revision.get(),
            }
            .boxed()
        })?;

        Ok(revision.clone())
    }

    /// Resolve one revision and its retained data.
    pub(crate) fn revision_data(
        &self,
        revision: Revision,
    ) -> RuntimeResult<(RevisionState, Arc<WorldImage>, Arc<TraceImage>)> {
        let revision_state = {
            let lineage = self.lineage.read();
            lineage.revision_state(revision)?
        };

        let lineage = self.lineage.read();
        let image = lineage
            .image(revision_state.image_id)
            .map_err(|error| match *error {
                RuntimeError::ImageNotFound { .. } => RuntimeError::RevisionImageMissing {
                    revision_id: revision.get(),
                    image_id: revision_state.image_id.get(),
                }
                .boxed(),
                _ => error,
            })?;
        let trace_image = lineage
            .trace_image(revision_state.trace_image_id)
            .map_err(|error| match *error {
                RuntimeError::ImageNotFound { .. } => RuntimeError::RevisionTraceImageMissing {
                    revision_id: revision.get(),
                }
                .boxed(),
                _ => error,
            })?;

        Ok((revision_state, image, trace_image))
    }

    /// Return the nearest retained base revision for one target revision.
    pub(crate) fn nearest_image_revision(&self, revision: Revision) -> RuntimeResult<Revision> {
        let lineage = self.lineage.read();

        lineage.nearest_image_revision(revision, |image_id| lineage.contains_image(image_id))
    }

    /// Return one retained trace image by revision identifier.
    pub(crate) fn trace_image(&self, revision: Revision) -> RuntimeResult<Arc<TraceImage>> {
        let lineage = self.lineage.read();
        let revision_state = lineage.revision_state(revision)?;

        lineage
            .trace_image(revision_state.trace_image_id)
            .map_err(|error| match *error {
                RuntimeError::ImageNotFound { .. } => RuntimeError::RevisionTraceImageMissing {
                    revision_id: revision.get(),
                }
                .boxed(),
                _ => error,
            })
    }

    /// Return handles for all known revisions in stable order.
    pub fn revisions(&self) -> Vec<Revision> {
        self.lineage.read().revisions.keys().copied().collect()
    }
}
