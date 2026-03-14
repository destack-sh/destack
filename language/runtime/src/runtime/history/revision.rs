use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::time::WorldInstant;
use crate::runtime::trace::TraceSequence;
use crate::runtime::world::World;

use super::lineage::RevisionBacking;
use super::{BranchId, ImageId};

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
        let lineage = self.lineage.borrow();
        let branch = lineage
            .branches
            .get(&self.branch_id)
            .expect("world lineage must contain the active branch");

        branch.head_revision_id
    }

    /// Return the active branch revision metadata for this world.
    pub fn revision(&self) -> Revision {
        let lineage = self.lineage.borrow();
        let revision = lineage
            .revisions
            .get(&self.revision_id())
            .expect("world lineage must contain the active revision");

        revision.clone()
    }

    /// Return metadata for one specific revision.
    pub fn revision_info(&self, revision_id: RevisionId) -> RuntimeResult<Revision> {
        let lineage = self.lineage.borrow();
        let revision = lineage.revisions.get(&revision_id).ok_or_else(|| {
            RuntimeError::RevisionNotFound {
                revision_id: revision_id.get(),
            }
            .boxed()
        })?;

        Ok(revision.clone())
    }

    /// Resolve one revision and all of its materialized backing.
    pub(crate) fn revision_backing(
        &self,
        revision_id: RevisionId,
    ) -> RuntimeResult<RevisionBacking> {
        let lineage = self.lineage.borrow();

        lineage.resolve_revision_backing(revision_id)
    }

    /// Return identifiers for all known revisions in stable order.
    pub fn revision_ids(&self) -> Vec<RevisionId> {
        self.lineage.borrow().revisions.keys().copied().collect()
    }
}
