use std::path::PathBuf;

use destack_source::{ModuleId, PackageId, TargetId};
use destack_workspace::{
    EntryResolutionMode, EntrySource, Target, TargetDiscoveryIssue, TargetDiscoveryOptions,
};

use crate::Compiler;
impl Compiler {
    /// Discover modules from entry points.
    pub(crate) fn discover_entry_modules(
        &self,
        revision: destack_workspace::Revision,
        package_id: PackageId,
        package_path: &Option<PathBuf>,
        target: &Target,
        target_id: &TargetId,
    ) -> Result<Vec<ModuleId>, TargetDiscoveryIssue> {
        // resolve manifest entry targets for auto entry source mode
        let manifest_entry_targets = self.manifest_entry_targets(revision, package_id);
        let options = TargetDiscoveryOptions {
            entry_source: EntrySource::Auto,
            entry_resolution: EntryResolutionMode::Strict,
            manifest_entry_targets: &manifest_entry_targets,
        };
        self.repository.entry_module_ids(
            revision,
            package_id,
            package_path,
            target,
            target_id,
            &options,
        )
    }

    /// Discover modules matching include and exclude patterns.
    pub(crate) fn discover_include_modules(
        &self,
        revision: destack_workspace::Revision,
        package_id: PackageId,
        package_path: &Option<PathBuf>,
        target: &Target,
        target_id: &TargetId,
    ) -> Result<Vec<ModuleId>, TargetDiscoveryIssue> {
        self.repository
            .include_module_ids(revision, package_id, package_path, target, target_id)
    }

    /// Return package manifest entry targets for one package.
    fn manifest_entry_targets(
        &self,
        revision: destack_workspace::Revision,
        package_id: PackageId,
    ) -> Vec<String> {
        let Some(package) = self.cache_package_snapshot(revision, package_id).ok() else {
            return Vec::new();
        };
        self.repository
            .package_declaration(revision, package.as_ref())
            .ok()
            .flatten()
            .map(|declaration| declaration.json.entry_targets())
            .unwrap_or_default()
    }
}
