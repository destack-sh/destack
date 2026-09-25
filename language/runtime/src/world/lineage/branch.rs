use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::World;
use crate::world::topology::LabelSet;

use super::{Moment, RevisionId};

/// Branch metadata for one world lineage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Branch {
    /// The branch identifier.
    pub id: BranchId,
    /// The current head revision for this branch.
    pub head_revision_id: RevisionId,
    /// The branch origin in the world lineage.
    pub origin: BranchOrigin,
    /// The branch name.
    pub name: String,
    /// The branch labels.
    pub labels: LabelSet,
}

/// Branch origin in one world lineage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum BranchOrigin {
    /// The root branch for one new world lineage.
    Root,
    /// One child branch forked from one parent moment.
    Fork {
        /// The moment where this branch forked.
        parent: Moment,
    },
}

/// Branch identifier for one world lineage.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct BranchId(u64);

impl World {
    /// Return the active branch identifier for this world.
    pub fn branch_id(&self) -> BranchId {
        self.state.branch_id
    }

    /// Set one label on one specific branch.
    pub fn label_branch(
        &self,
        branch_id: BranchId,
        key: impl Into<Box<str>>,
        value: impl Into<Box<str>>,
    ) -> RuntimeResult<()> {
        let mut lineage = self.lineage.write();
        let branch = lineage
            .branches
            .get_mut(&branch_id)
            .ok_or_else(|| RuntimeError::branch_not_found(branch_id.get()).boxed())?;
        branch.labels.insert(key, value);

        Ok(())
    }
}

impl BranchId {
    /// Create a new branch identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw branch identifier value.
    pub const fn get(self) -> u64 {
        self.0
    }
}
