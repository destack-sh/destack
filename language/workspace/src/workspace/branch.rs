use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tspp_repository::Revision;
use tspp_serde::Reflect;

use crate::{Error, Watch, Workspace};

/// One named workspace revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Branch {
    /// Branch name.
    pub name: String,
    /// Current branch revision.
    pub revision: Revision,
}

/// Files selected by one branch operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum FileSelection {
    /// Every changed file.
    All,
    /// Selected paths within the workspace.
    Paths(Vec<PathBuf>),
}

impl Workspace {
    /// List every branch in name order.
    pub fn branches(&self) -> Result<Vec<Branch>, Error> {
        let state = self.lock()?;
        let mut branches = state
            .branches
            .iter()
            .map(|(name, revision)| Branch {
                name: name.clone(),
                revision: revision.revision(),
            })
            .collect::<Vec<_>>();
        branches.sort_unstable_by(|left, right| left.name.cmp(&right.name));

        Ok(branches)
    }

    /// Create one branch at an existing revision.
    pub fn create_branch(&self, name: String, revision: Revision) -> Result<Branch, Error> {
        let mut state = self.lock()?;
        if state.branches.contains_key(&name) {
            return Err(Error::BranchExists { name });
        }

        let pin = self.repository.pin(revision)?;
        let branch = Branch {
            name: name.clone(),
            revision,
        };
        state.branches.insert(name, pin);

        Ok(branch)
    }

    /// Return one branch's current revision.
    pub fn branch_revision(&self, name: &str) -> Result<Revision, Error> {
        let state = self.lock()?;
        let branch = state.branch(name)?;

        Ok(branch.revision())
    }

    /// Remove one branch.
    pub fn remove_branch(&self, name: &str) -> Result<(), Error> {
        let mut state = self.lock()?;
        state
            .branches
            .remove(name)
            .ok_or_else(|| Error::MissingBranch {
                name: name.to_string(),
            })?;
        self.watch.lock().remove_branch(name);

        Ok(())
    }

    /// Watch one workspace branch.
    pub fn watch_branch(&self, name: &str) -> Result<Watch, Error> {
        let state = self.lock()?;
        let revision = state.branch(name)?.revision();

        Watch::new(
            self.root.clone(),
            Some(name.to_string()),
            revision,
            &self.repository,
            self.watch.clone(),
        )
    }
}
