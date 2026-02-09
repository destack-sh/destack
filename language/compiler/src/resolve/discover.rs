use std::path::PathBuf;

use destack_source::{ModuleId, PackageId};
use destack_workspace::{
    Target, TargetDiscoveryIssue, TargetId, discover_entry_modules, discover_include_modules,
};

use crate::Compiler;

impl Compiler {
    /// Discover modules from entry points.
    pub(crate) fn discover_entry_modules(
        &self,
        package_id: PackageId,
        package_path: &Option<PathBuf>,
        target: &Target,
        target_id: &TargetId,
    ) -> Result<Vec<ModuleId>, TargetDiscoveryIssue> {
        discover_entry_modules(
            &self.program.modules,
            package_id,
            package_path,
            target,
            target_id,
        )
    }

    /// Discover modules matching include and exclude patterns.
    pub(crate) fn discover_include_modules(
        &self,
        package_id: PackageId,
        package_path: &Option<PathBuf>,
        target: &Target,
    ) -> Result<Vec<ModuleId>, TargetDiscoveryIssue> {
        discover_include_modules(&self.program.modules, package_id, package_path, target)
    }
}
