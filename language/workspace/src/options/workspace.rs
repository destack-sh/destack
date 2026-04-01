use crate::config::{DestackJson, WorkspaceMembershipDeclaration};

use super::PackageOptions;

/// Effective normalized workspace options for one revision scoped workspace view.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceOptions {
    /// The effective package scoped defaults inherited by packages in the workspace.
    pub package: PackageOptions,
    /// The effective workspace membership declarations.
    pub membership: WorkspaceMembershipDeclaration,
}

impl From<&DestackJson> for WorkspaceOptions {
    fn from(json: &DestackJson) -> Self {
        Self {
            package: PackageOptions::from(json),
            membership: json
                .workspace
                .as_ref()
                .map(WorkspaceMembershipDeclaration::from)
                .unwrap_or_default(),
        }
    }
}
