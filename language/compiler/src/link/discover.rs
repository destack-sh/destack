use crate::{Compiler, LinkError, LinkResult};

use destack_source::{ModuleId, PackageId, matches as glob_matches};
use destack_workspace::Target;

impl Compiler {
    /// Discover modules from entry points.
    pub(super) fn discover_entry_modules(
        &self,
        package_id: PackageId,
        package_path: &Option<std::path::PathBuf>,
        target: &Target,
    ) -> LinkResult<Vec<ModuleId>> {
        let package_dir = package_path
            .as_ref()
            .ok_or_else(|| LinkError::InvalidTarget {
                node: self.program.root_node_id,
                target: target.name.clone(),
                message: "entry-based discovery requires package path".to_string(),
            })?;

        let mut modules = Vec::new();
        for entry in &target.entry {
            let entry_path = if entry.is_absolute() {
                entry.clone()
            } else {
                package_dir.join(entry)
            };

            // look up module by path
            let module_id = self.program.modules.get_id_by_path(&entry_path);
            if let Some(id) = module_id {
                let module = self.program.modules.get(id);
                if module.read().package_id == package_id {
                    modules.push(id);
                }
            } else {
                return Err(LinkError::Internal {
                    node: self.program.root_node_id,
                    message: format!("entry point not found: {}", entry_path.display()),
                });
            }
        }
        Ok(modules)
    }

    /// Discover modules matching include/exclude patterns.
    pub(super) fn discover_include_modules(
        &self,
        package_id: PackageId,
        package_path: &Option<std::path::PathBuf>,
        target: &Target,
    ) -> LinkResult<Vec<ModuleId>> {
        let include = &target.include;
        let exclude = &target.exclude;
        let mut modules = Vec::new();

        // iterate all modules in the package
        for module in self.program.modules.iter() {
            let module = module.read();

            // skip modules not in this package
            if module.package_id != package_id {
                continue;
            }

            // get relative path for pattern matching
            let relative_path = match (&module.path, package_path) {
                (Some(module_path), Some(pkg_path)) => module_path
                    .strip_prefix(pkg_path)
                    .ok()
                    .map(|p| p.to_path_buf()),
                _ => None,
            };

            // modules without paths or package paths: include by default if no patterns
            let Some(relative_path) = relative_path else {
                if include.is_empty() {
                    modules.push(module.id);
                }
                continue;
            };

            let path_str = relative_path.to_string_lossy();
            let path_bytes = path_str.as_bytes();

            // check exclude patterns first
            let is_excluded = exclude
                .iter()
                .any(|pattern| glob_matches(pattern.as_bytes(), 0, path_bytes, 0));
            if is_excluded {
                continue;
            }

            // check include patterns (empty include = include all)
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
