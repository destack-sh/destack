use std::path::{Path, PathBuf};

use destack_source::{ModuleId, PackageId, TargetId, matches as glob_matches};

use crate::repository::{Repository, Revision};
use crate::{Target, TargetDiscovery};

/// Select how entry paths are resolved against package and repository paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryResolutionMode {
    /// Resolve entries relative to package path only.
    Strict,
    /// Resolve entries relative to package path, then as repository paths.
    RepositoryRelative,
}

/// Options for entry discovery behavior.
#[derive(Debug, Clone)]
pub struct TargetDiscoveryOptions {
    /// Resolution policy for selected entry paths.
    pub entry_resolution: EntryResolutionMode,
}

impl Default for TargetDiscoveryOptions {
    /// Return the default target discovery options.
    fn default() -> Self {
        Self {
            entry_resolution: EntryResolutionMode::Strict,
        }
    }
}

/// Describe a failure while discovering target modules.
#[derive(Debug, Clone)]
pub enum TargetDiscoveryError {
    /// Repository state lookup failed during discovery.
    Repository {
        /// Package id for the discovery.
        package: PackageId,
        /// Target id for the discovery.
        target: TargetId,
        /// The repository error message.
        message: String,
    },
    /// Missing package path for entry based discovery.
    MissingPackagePath {
        /// Package id for the discovery.
        package: PackageId,
        /// Target id for the discovery.
        target: TargetId,
    },
    /// Missing target for target based discovery.
    MissingTarget {
        /// Package id for the discovery.
        package: PackageId,
        /// Target id for the discovery.
        target: TargetId,
    },
    /// Missing module path for target discovery.
    MissingModulePath {
        /// Package id for the discovery.
        package: PackageId,
        /// Target id for the discovery.
        target: TargetId,
        /// Entry path that could not be resolved.
        path: PathBuf,
    },
}

impl Repository {
    /// Match one target glob against one path.
    fn matches_target_glob(pattern: &str, path: &Path) -> bool {
        // borrowed byte views
        let pattern = pattern.as_bytes();
        let path = path.to_string_lossy();
        let path = path.as_bytes();

        glob_matches(pattern, 0, path, 0)
    }

    /// Select effective entry paths for one target.
    fn select_target_entry_paths(
        &self,
        package_id: PackageId,
        target_id: TargetId,
        target: &Target,
        _options: &TargetDiscoveryOptions,
    ) -> Result<Vec<PathBuf>, TargetDiscoveryError> {
        let _ = package_id;
        let _ = target_id;

        Ok(target.entry.clone())
    }

    /// Resolve selected target paths to package-local module ids.
    fn resolve_target_paths(
        &self,
        revision: Revision,
        package_id: PackageId,
        target_id: TargetId,
        package_path: &Option<PathBuf>,
        target_paths: &[PathBuf],
        resolution_mode: EntryResolutionMode,
    ) -> Result<Vec<ModuleId>, TargetDiscoveryError> {
        let mut module_ids = Vec::new();

        // selected target paths
        for path in target_paths {
            let module_id = match resolution_mode {
                EntryResolutionMode::Strict => self.resolve_package_target_path(
                    revision,
                    package_id,
                    target_id,
                    package_path,
                    path,
                )?,
                EntryResolutionMode::RepositoryRelative => self.resolve_repository_target_path(
                    revision,
                    package_id,
                    target_id,
                    package_path,
                    path,
                )?,
            };

            module_ids.push(module_id);
        }

        Ok(module_ids)
    }

    /// Resolve one target path relative to package path only.
    fn resolve_package_target_path(
        &self,
        revision: Revision,
        package_id: PackageId,
        target_id: TargetId,
        package_path: &Option<PathBuf>,
        target_path: &Path,
    ) -> Result<ModuleId, TargetDiscoveryError> {
        // package directory
        let package_directory =
            package_path
                .as_ref()
                .ok_or(TargetDiscoveryError::MissingPackagePath {
                    package: package_id,
                    target: target_id,
                })?;

        // package relative path
        let resolved_path = package_directory.join(target_path);

        // repository module
        self.package_module_id_for_path(revision, package_id, &resolved_path)
            .ok_or(TargetDiscoveryError::MissingModulePath {
                package: package_id,
                target: target_id,
                path: resolved_path,
            })
    }

    /// Resolve one target path with package-relative and repository-relative checks.
    fn resolve_repository_target_path(
        &self,
        revision: Revision,
        package_id: PackageId,
        target_id: TargetId,
        package_path: &Option<PathBuf>,
        target_path: &Path,
    ) -> Result<ModuleId, TargetDiscoveryError> {
        let mut candidate_paths = Vec::new();

        // package relative candidate
        if !target_path.is_absolute() {
            let package_directory =
                package_path
                    .as_ref()
                    .ok_or(TargetDiscoveryError::MissingPackagePath {
                        package: package_id,
                        target: target_id,
                    })?;
            candidate_paths.push(package_directory.join(target_path));
        }

        // repository relative or absolute candidate
        candidate_paths.push(target_path.to_path_buf());

        // return the first path with one repository module
        for candidate_path in &candidate_paths {
            if let Some(module_id) =
                self.package_module_id_for_path(revision, package_id, candidate_path)
            {
                return Ok(module_id);
            }
        }

        let missing_path = candidate_paths
            .first()
            .cloned()
            .unwrap_or_else(|| target_path.to_path_buf());

        Err(TargetDiscoveryError::MissingModulePath {
            package: package_id,
            target: target_id,
            path: missing_path,
        })
    }

    /// Resolve one repository path to a package-local module id.
    fn package_module_id_for_path(
        &self,
        revision: Revision,
        package_id: PackageId,
        path: &Path,
    ) -> Option<ModuleId> {
        // resolve path to module
        let module_id = self.module_id_for_path(revision, path).ok()??;
        let module = self.module(revision, module_id).ok()??;

        // keep package local modules only
        if module.package_id != package_id {
            return None;
        }

        Some(module_id)
    }

    /// Discover modules selected by target entry rules.
    fn discover_entry_modules(
        &self,
        revision: Revision,
        package_id: PackageId,
        target_id: TargetId,
        package_path: &Option<PathBuf>,
        target: &Target,
        options: &TargetDiscoveryOptions,
    ) -> Result<Vec<ModuleId>, TargetDiscoveryError> {
        // entry paths
        let entry_paths = self.select_target_entry_paths(package_id, target_id, target, options)?;

        // repository modules
        self.resolve_target_paths(
            revision,
            package_id,
            target_id,
            package_path,
            &entry_paths,
            options.entry_resolution,
        )
    }

    /// Discover modules selected as target globals.
    fn discover_global_modules(
        &self,
        revision: Revision,
        package_id: PackageId,
        target_id: TargetId,
        package_path: &Option<PathBuf>,
        target: &Target,
        options: &TargetDiscoveryOptions,
    ) -> Result<Vec<ModuleId>, TargetDiscoveryError> {
        self.resolve_target_paths(
            revision,
            package_id,
            target_id,
            package_path,
            &target.globals,
            options.entry_resolution,
        )
    }

    /// Discover modules selected by target include rules.
    fn discover_included_modules(
        &self,
        revision: Revision,
        package_id: PackageId,
        target_id: TargetId,
        package_path: &Option<PathBuf>,
        target: &Target,
    ) -> Result<Vec<ModuleId>, TargetDiscoveryError> {
        let mut selected_module_ids = Vec::new();

        // package local include scan
        let module_ids = self
            .package_module_ids(revision, package_id)
            .map_err(|error| TargetDiscoveryError::Repository {
                package: package_id,
                target: target_id,
                message: error.to_string(),
            })?;
        for module_id in module_ids {
            let Some(module) = self.module(revision, module_id).map_err(|error| {
                TargetDiscoveryError::Repository {
                    package: package_id,
                    target: target_id,
                    message: error.to_string(),
                }
            })?
            else {
                continue;
            };
            let module = module.as_ref();
            let Some(module_path) = module.path.as_ref() else {
                continue;
            };

            // path relative to package when possible
            let relative_path = package_path
                .as_ref()
                .and_then(|package_path| module_path.strip_prefix(package_path).ok())
                .unwrap_or(module_path);

            // include patterns
            let is_included = target.include.is_empty()
                || target
                    .include
                    .iter()
                    .any(|pattern| Self::matches_target_glob(pattern, relative_path));
            if !is_included {
                continue;
            }

            // exclude patterns
            let is_excluded = target
                .exclude
                .iter()
                .any(|pattern| Self::matches_target_glob(pattern, relative_path));

            if is_excluded {
                continue;
            }

            selected_module_ids.push(module.id);
        }

        Ok(selected_module_ids)
    }

    /// Discover module ids selected by one target in one pinned revision.
    pub fn target_module_ids(
        &self,
        revision: Revision,
        target_id: TargetId,
        options: &TargetDiscoveryOptions,
    ) -> Result<Vec<ModuleId>, TargetDiscoveryError> {
        // package and target
        let package_id = target_id.package_id();
        let package = self
            .package(revision, package_id)
            .map_err(|error| TargetDiscoveryError::Repository {
                package: package_id,
                target: target_id,
                message: error.to_string(),
            })?
            .ok_or(TargetDiscoveryError::MissingPackagePath {
                package: package_id,
                target: target_id,
            })?;
        let target = self
            .effective_target(revision, target_id)
            .map_err(|error| TargetDiscoveryError::Repository {
                package: package_id,
                target: target_id,
                message: error.to_string(),
            })?
            .ok_or(TargetDiscoveryError::MissingTarget {
                package: package_id,
                target: target_id,
            })?;
        let package_options = self
            .package_options(revision, package_id)
            .map_err(|error| TargetDiscoveryError::Repository {
                package: package_id,
                target: target_id,
                message: error.to_string(),
            })?;
        let compiler_globals = package_options
            .as_ref()
            .map(|package_options| package_options.compiler.globals.clone())
            .unwrap_or_default();
        let profile_globals = package_options
            .as_ref()
            .and_then(|package_options| {
                target
                    .profile
                    .as_ref()
                    .or(package_options.compiler.profile.as_ref())
                    .and_then(|name| package_options.profiles.get(name))
            })
            .map(|profile| profile.globals.clone())
            .unwrap_or_default();

        // selected modules
        let mut module_ids = match target.discovery {
            TargetDiscovery::Entry => self.discover_entry_modules(
                revision,
                package_id,
                target_id,
                &package.path,
                &target,
                options,
            ),
            TargetDiscovery::Include => self.discover_included_modules(
                revision,
                package_id,
                target_id,
                &package.path,
                &target,
            ),
        }?;

        // explicit globals are additional roots
        let global_module_ids = self.discover_global_modules(
            revision,
            package_id,
            target_id,
            &package.path,
            &target,
            options,
        )?;
        let compiler_global_module_ids = self.resolve_target_paths(
            revision,
            package_id,
            target_id,
            &package.path,
            &compiler_globals,
            options.entry_resolution,
        )?;
        let profile_global_module_ids = self.resolve_target_paths(
            revision,
            package_id,
            target_id,
            &package.path,
            &profile_globals,
            options.entry_resolution,
        )?;
        for module_id in global_module_ids
            .into_iter()
            .chain(compiler_global_module_ids)
            .chain(profile_global_module_ids)
        {
            if module_ids.contains(&module_id) {
                continue;
            }

            module_ids.push(module_id);
        }

        Ok(module_ids)
    }
}
