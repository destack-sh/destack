use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::World;
use crate::world::trace::TraceSequence;

use super::{BranchId, RevisionId};

/// Precise history coordinate over one branch and one trace sequence.
///
/// This is not a clock timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Moment {
    /// The branch that owns this history coordinate.
    pub branch_id: BranchId,
    /// The trace sequence reached at this coordinate.
    pub sequence: TraceSequence,
}

impl Moment {
    /// Create one history coordinate from one branch and sequence.
    pub const fn new(branch_id: BranchId, sequence: TraceSequence) -> Self {
        Self {
            branch_id,
            sequence,
        }
    }
}

impl World {
    /// Return the current live history coordinate for this world.
    pub fn moment(&self) -> Moment {
        Moment::new(self.state.branch_id, self.state.trace.log().next_sequence())
    }

    /// Return the exact history coordinate for one committed revision.
    pub fn revision_moment(&self, revision_id: RevisionId) -> RuntimeResult<Moment> {
        let history = self.history.read();
        let revision = history
            .revisions
            .get(&revision_id)
            .ok_or_else(|| RuntimeError::revision_not_found(revision_id.get()).boxed())?;

        Ok(Moment::new(revision.branch_id, revision.sequence))
    }
}
