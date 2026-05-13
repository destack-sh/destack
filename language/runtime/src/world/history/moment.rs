use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::World;
use crate::world::trace::TraceSequence;

use super::{BranchId, Revision};

/// Precise execution coordinate over one branch and one trace sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Moment {
    /// The branch that owns this execution coordinate.
    pub branch_id: BranchId,
    /// The trace sequence reached at this coordinate.
    pub sequence: TraceSequence,
}

impl Moment {
    /// Create one execution coordinate from one branch and sequence.
    pub const fn new(branch_id: BranchId, sequence: TraceSequence) -> Self {
        Self {
            branch_id,
            sequence,
        }
    }
}

impl World {
    /// Return the current live execution coordinate for this world.
    pub fn moment(&self) -> Moment {
        Moment::new(self.state.branch_id, self.state.trace.log().next_sequence())
    }

    /// Return the exact execution coordinate for one committed revision.
    pub fn revision_moment(&self, revision: Revision) -> RuntimeResult<Moment> {
        let lineage = self.lineage.read();
        let revision = lineage.revisions.get(&revision).ok_or_else(|| {
            RuntimeError::RevisionNotFound {
                revision_id: revision.get(),
            }
            .boxed()
        })?;

        Ok(Moment::new(revision.branch_id, revision.sequence))
    }
}
