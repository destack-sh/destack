use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::world::World;

use super::Revision;

/// Branch identifier for one world lineage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BranchId(u128);

impl BranchId {
    /// Create a new branch identifier.
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    /// Return the raw branch identifier value.
    pub const fn get(self) -> u128 {
        self.0
    }
}

/// Branch metadata for one world lineage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Branch {
    /// The branch identifier.
    pub id: BranchId,
    /// The current head revision for this branch.
    pub head_revision: Revision,
    /// The branch origin in the world lineage.
    pub origin: BranchOrigin,
    /// The branch name.
    pub name: String,
    /// The branch labels.
    pub labels: BTreeMap<String, String>,
}

/// Branch origin in one world lineage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchOrigin {
    /// The root branch for one new world lineage.
    Root,
    /// One child branch forked from one parent revision.
    Fork {
        /// The revision where this branch forked.
        parent_revision: Revision,
    },
}

impl World {
    /// Return the active branch identifier for this world.
    pub fn branch_id(&self) -> BranchId {
        self.state.branch_id
    }

    /// Return the active branch metadata for this world.
    pub fn branch(&self) -> Branch {
        let lineage = self.lineage.read();
        let branch = lineage
            .branches
            .get(&self.state.branch_id)
            .expect("world lineage must contain the active branch");

        branch.clone()
    }

    /// Return metadata for one specific branch.
    pub fn branch_info(&self, branch_id: BranchId) -> RuntimeResult<Branch> {
        let lineage = self.lineage.read();
        let branch = lineage.branches.get(&branch_id).ok_or_else(|| {
            RuntimeError::BranchNotFound {
                branch_id: branch_id.get(),
            }
            .boxed()
        })?;

        Ok(branch.clone())
    }

    /// Return identifiers for all known branches in stable order.
    pub fn branch_ids(&self) -> Vec<BranchId> {
        self.lineage.read().branches.keys().copied().collect()
    }

    /// Replace labels for one specific branch.
    pub fn set_branch_labels(
        &self,
        branch_id: BranchId,
        labels: BTreeMap<String, String>,
    ) -> RuntimeResult<()> {
        let mut lineage = self.lineage.write();
        let branch = lineage.branches.get_mut(&branch_id).ok_or_else(|| {
            RuntimeError::BranchNotFound {
                branch_id: branch_id.get(),
            }
            .boxed()
        })?;
        branch.labels = labels;

        Ok(())
    }
}
