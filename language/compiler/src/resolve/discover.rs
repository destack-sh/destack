use std::path::PathBuf;

use destack_source::{ModuleId, PackageId, matches as glob_matches};
use destack_workspace::{Target, TargetId};

use crate::Compiler;

/// Describe a failure while discovering target modules.
#[derive(Debug, Clone)]
pub(crate) enum TargetDiscoveryIssue {
    /// Missing package path for entry based discovery.
    MissingPackagePath {
        /// Package id for the discovery.
        package: PackageId,
        /// Target id for the discovery.
        target: TargetId,
    },
    /// Missing entry module for entry based discovery.
    MissingEntry {
        /// Package id for the discovery.
        package: PackageId,
        /// Target id for the discovery.
        target: TargetId,
        /// Entry path that could not be resolved.
        path: PathBuf,
    },
}

impl Compiler {
    /// Discover modules from entry points.
    pub(crate) fn discover_entry_modules(
        &self,
        package_id: PackageId,
        package_path: &Option<PathBuf>,
        target: &Target,
        target_id: &TargetId,
    ) -> Result<Vec<ModuleId>, TargetDiscoveryIssue> {
        // require a package path for entry discovery
        let package_dir =
            package_path
                .as_ref()
                .ok_or_else(|| TargetDiscoveryIssue::MissingPackagePath {
                    package: package_id,
                    target: target_id.clone(),
                })?;

        // collect entry modules in the package
        let mut modules = Vec::new();
        for entry in &target.entry {
            // build the entry path relative to the package
            let entry_path = if entry.is_absolute() {
                entry.clone()
            } else {
                package_dir.join(entry)
            };

            // look up module by path
            let module_id = self.program.modules.get_id_by_path(&entry_path);
            if let Some(id) = module_id {
                // retain modules that belong to this package
                let module = self.program.modules.get(id);
                if module.read().package_id == package_id {
                    modules.push(id);
                }
            } else {
                return Err(TargetDiscoveryIssue::MissingEntry {
                    package: package_id,
                    target: target_id.clone(),
                    path: entry_path,
                });
            }
        }
        Ok(modules)
    }

    /// Discover modules matching include and exclude patterns.
    pub(crate) fn discover_include_modules(
        &self,
        package_id: PackageId,
        package_path: &Option<PathBuf>,
        target: &Target,
    ) -> Result<Vec<ModuleId>, TargetDiscoveryIssue> {
        // gather include and exclude patterns
        let include = &target.include;
        let exclude = &target.exclude;
        let mut modules = Vec::new();

        // scan modules that belong to the package
        for module in self.program.modules.iter() {
            let module = module.read();

            // skip modules not in this package
            if module.package_id != package_id {
                continue;
            }

            // compute the module path relative to the package
            let relative_path = match (&module.path, package_path) {
                (Some(module_path), Some(package_path)) => module_path
                    .strip_prefix(package_path)
                    .ok()
                    .map(|path| path.to_path_buf()),
                _ => None,
            };

            // include the module when there is no relative path and include is empty
            let Some(relative_path) = relative_path else {
                if include.is_empty() {
                    modules.push(module.id);
                }
                continue;
            };

            // evaluate exclude patterns
            let path_str = relative_path.to_string_lossy();
            let path_bytes = path_str.as_bytes();

            let is_excluded = exclude
                .iter()
                .any(|pattern| glob_matches(pattern.as_bytes(), 0, path_bytes, 0));
            if is_excluded {
                continue;
            }

            // evaluate include patterns
            let is_included = include.is_empty()
                || include
                    .iter()
                    .any(|pattern| glob_matches(pattern.as_bytes(), 0, path_bytes, 0));
            if is_included {
                modules.push(module.id);
            }
        }

        Ok(modules)
    }
}
