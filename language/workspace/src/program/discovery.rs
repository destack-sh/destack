use std::path::PathBuf;

use destack_source::{ModuleId, PackageId, matches as glob_matches};

use crate::{ModuleRegistry, Target, TargetDiscovery, TargetId};

/// Describe a failure while discovering target modules.
#[derive(Debug, Clone)]
pub enum TargetDiscoveryIssue {
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

/// Discover modules for one target using the selected discovery strategy.
pub fn discover_target_modules(
    modules: &ModuleRegistry,
    package_id: PackageId,
    package_path: &Option<PathBuf>,
    target: &Target,
    target_id: &TargetId,
) -> Result<Vec<ModuleId>, TargetDiscoveryIssue> {
    // dispatch discovery based on target mode
    match target.discovery {
        TargetDiscovery::Entry => {
            discover_entry_modules(modules, package_id, package_path, target, target_id)
        }
        TargetDiscovery::Include => {
            discover_include_modules(modules, package_id, package_path, target)
        }
    }
}

/// Discover modules from target entry points.
pub fn discover_entry_modules(
    modules: &ModuleRegistry,
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

    let mut discovered_modules = Vec::new();

    // collect target entry modules in package order
    for entry in &target.entry {
        // build entry path relative to the package when needed
        let entry_path = if entry.is_absolute() {
            entry.clone()
        } else {
            package_dir.join(entry)
        };

        let Some(module_id) = modules.get_id_by_path(&entry_path) else {
            return Err(TargetDiscoveryIssue::MissingEntry {
                package: package_id,
                target: target_id.clone(),
                path: entry_path,
            });
        };

        // retain only package local modules
        let module_ref = modules.get(module_id);
        let module = module_ref.read();
        if module.package_id == package_id {
            discovered_modules.push(module_id);
        }
    }

    Ok(discovered_modules)
}

/// Discover modules from target entry points with direct path fallback.
///
/// This variant supports in-memory and synthetic package setups where
/// a package path may not exist.
pub fn discover_entry_modules_relaxed(
    modules: &ModuleRegistry,
    package_id: PackageId,
    package_path: &Option<PathBuf>,
    target: &Target,
    target_id: &TargetId,
) -> Result<Vec<ModuleId>, TargetDiscoveryIssue> {
    let mut discovered_modules = Vec::new();

    // collect target entry modules in package order
    for entry in &target.entry {
        let mut resolved_module_id = None;

        // prefer package relative path resolution when available
        if let Some(package_dir) = package_path
            && !entry.is_absolute()
        {
            let package_entry_path = package_dir.join(entry);
            resolved_module_id = resolve_entry_module_id(modules, package_id, &package_entry_path);
        }

        // fall back to direct entry path resolution
        if resolved_module_id.is_none() {
            resolved_module_id = resolve_entry_module_id(modules, package_id, entry);
        }

        // reject unresolved entries with a clear issue
        let Some(module_id) = resolved_module_id else {
            if package_path.is_none() && !entry.is_absolute() {
                return Err(TargetDiscoveryIssue::MissingPackagePath {
                    package: package_id,
                    target: target_id.clone(),
                });
            }

            return Err(TargetDiscoveryIssue::MissingEntry {
                package: package_id,
                target: target_id.clone(),
                path: entry.clone(),
            });
        };

        discovered_modules.push(module_id);
    }

    Ok(discovered_modules)
}

/// Resolve one entry path to a package-local module id.
fn resolve_entry_module_id(
    modules: &ModuleRegistry,
    package_id: PackageId,
    entry_path: &std::path::Path,
) -> Option<ModuleId> {
    let module_id = modules.get_id_by_path(entry_path)?;
    let module_ref = modules.get(module_id);
    let module = module_ref.read();
    if module.package_id != package_id {
        return None;
    }

    Some(module_id)
}

/// Discover modules matching include and exclude patterns.
pub fn discover_include_modules(
    modules: &ModuleRegistry,
    package_id: PackageId,
    package_path: &Option<PathBuf>,
    target: &Target,
) -> Result<Vec<ModuleId>, TargetDiscoveryIssue> {
    let mut discovered_modules = Vec::new();

    // scan modules that belong to the package
    for module_ref in modules.iter() {
        let module = module_ref.read();
        if module.package_id != package_id {
            continue;
        }

        // compute module path relative to the package
        let relative_path = match (&module.path, package_path) {
            (Some(module_path), Some(package_path)) => module_path
                .strip_prefix(package_path)
                .ok()
                .map(|path| path.to_path_buf()),
            _ => None,
        };

        // include pathless modules only when include patterns are empty
        let Some(relative_path) = relative_path else {
            if target.include.is_empty() {
                discovered_modules.push(module.id);
            }

            continue;
        };

        // skip modules excluded by pattern
        let path_text = relative_path.to_string_lossy();
        let path_bytes = path_text.as_bytes();
        let is_excluded = target
            .exclude
            .iter()
            .any(|pattern| glob_matches(pattern.as_bytes(), 0, path_bytes, 0));
        if is_excluded {
            continue;
        }

        // keep modules included by pattern, or all when include is empty
        let is_included = target.include.is_empty()
            || target
                .include
                .iter()
                .any(|pattern| glob_matches(pattern.as_bytes(), 0, path_bytes, 0));
        if is_included {
            discovered_modules.push(module.id);
        }
    }

    Ok(discovered_modules)
}
