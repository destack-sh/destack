use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::World;

use super::{BranchId, RevisionId};

/// Precise lineage coordinate over one branch and one moment sequence.
///
/// This is not a clock timestamp.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct Moment {
    /// The branch that owns this lineage coordinate.
    pub branch_id: BranchId,
    /// The moment sequence reached at this coordinate.
    pub sequence: MomentSequence,
}

/// Sequence number for moments within one branch.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct MomentSequence(u64);

/// Sequence number for one worker execution stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WorkerSequence(u64);

impl Moment {
    /// Create one lineage coordinate from one branch and sequence.
    pub const fn new(branch_id: BranchId, sequence: MomentSequence) -> Self {
        Self {
            branch_id,
            sequence,
        }
    }
}

impl MomentSequence {
    /// Create one moment sequence.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw moment sequence.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Return the next moment sequence.
    pub fn next(self) -> RuntimeResult<Self> {
        let value = self.0.checked_add(1).ok_or_else(|| {
            RuntimeError::Internal {
                message: "moment sequence space exhausted".to_string(),
            }
            .boxed()
        })?;

        Ok(Self(value))
    }
}

impl WorkerSequence {
    /// Create one worker sequence.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw worker sequence.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Return the next worker sequence.
    pub fn next(self) -> RuntimeResult<Self> {
        let value = self.0.checked_add(1).ok_or_else(|| {
            RuntimeError::Internal {
                message: "worker sequence space exhausted".to_string(),
            }
            .boxed()
        })?;

        Ok(Self(value))
    }
}

impl World {
    /// Return the current live lineage coordinate for this world.
    pub fn moment(&self) -> Moment {
        self.state.moment()
    }

    /// Return the exact lineage coordinate for one committed revision.
    pub fn revision_moment(&self, revision_id: RevisionId) -> RuntimeResult<Moment> {
        let lineage = self.lineage.read();
        let revision = lineage
            .revisions
            .get(&revision_id)
            .ok_or_else(|| RuntimeError::revision_not_found(revision_id.get()).boxed())?;

        Ok(revision.moment())
    }
}
