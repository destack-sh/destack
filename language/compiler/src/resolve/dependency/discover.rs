use std::path::PathBuf;

use destack_source::{ModuleId, PackageId};
use destack_workspace::{
    EntryResolutionMode, EntrySource, Target, TargetDiscoveryIssue, TargetDiscoveryOptions,
    TargetId, discover_entry_modules, discover_include_modules,
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
        // resolve manifest entry targets for auto entry source mode
        let manifest_entry_targets = self.manifest_entry_targets(package_id);
        let options = TargetDiscoveryOptions {
            entry_source: EntrySource::Auto,
            entry_resolution: EntryResolutionMode::Strict,
            manifest_entry_targets: &manifest_entry_targets,
        };
        discover_entry_modules(
            &self.program.modules,
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
        package_id: PackageId,
        package_path: &Option<PathBuf>,
        target: &Target,
    ) -> Result<Vec<ModuleId>, TargetDiscoveryIssue> {
        discover_include_modules(&self.program.modules, package_id, package_path, target)
    }

    /// Return package manifest entry targets for one package.
    fn manifest_entry_targets(&self, package_id: PackageId) -> Vec<String> {
        let package = self.program.packages.get(package_id);
        let package = package.read();
        package
            .manifest
            .as_ref()
            .map(|manifest| manifest.content.entry_targets())
            .unwrap_or_default()
    }
}
