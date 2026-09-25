use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tspp_repository::Revision;
use tspp_serde::Reflect;

/// Request to list branches in one workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ListBranchesRequest {
    /// Owning workspace root.
    pub root: PathBuf,
}

/// Request to create one workspace branch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CreateBranchRequest {
    /// Owning workspace root.
    pub root: PathBuf,
    /// New branch name.
    pub name: String,
    /// Existing revision selected by the new branch.
    pub revision: Revision,
}

/// Request to read one workspace branch revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct BranchRevisionRequest {
    /// Owning workspace root.
    pub root: PathBuf,
    /// Branch to read.
    pub name: String,
}

/// Request to remove one workspace branch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct RemoveBranchRequest {
    /// Owning workspace root.
    pub root: PathBuf,
    /// Branch to remove.
    pub name: String,
}
