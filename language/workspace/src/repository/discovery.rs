use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{ModuleId, PackageId, TargetId, matches as glob_matches};

use crate::repository::{Repository, RepositoryError, Revision};
use crate::{DestackFile, Package, Target, TargetDiscovery};

/// Describe a failure while discovering target modules.
#[derive(Debug, Clone)]
pub enum TargetDiscoveryError {
    /// Repository state lookup failed during discovery.
    RepositoryRead {
        /// Package id for the discovery.
        package: PackageId,
        /// Target id for the discovery.
        target: TargetId,
        /// The repository failure.
        error: Box<RepositoryError>,
    },
    /// Missing package for target discovery.
    MissingPackage {
        /// Package id for the discovery.
        package: PackageId,
        /// Target id for the discovery.
        target: TargetId,
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
    /// Discover module ids selected by one target in one pinned revision.
    pub fn target_module_ids(
        &self,
        revision: Revision,
        target_id: TargetId,
    ) -> Result<Vec<ModuleId>, TargetDiscoveryError> {
        let package_id = target_id.package_id();
        let package = self.target_package(revision, package_id, target_id)?;
        let target = self.target_or_builtin_for_discovery(revision, package_id, target_id)?;
        let config = self.target_config(revision, package_id, target_id)?;

        // selected roots
        let mut module_ids = match target.discovery {
            TargetDiscovery::Entry => self.resolve_target_paths(
                revision,
                package_id,
                target_id,
                &package.path,
                &target.entry,
            ),
            TargetDiscovery::Include => {
                self.included_module_ids(revision, package_id, target_id, &package.path, &target)
            }
        }?;

        // global roots
        let mut global_paths = target.globals.clone();
        if let Some(config) = config.as_ref() {
            global_paths.extend(config.compiler.globals.clone());
            if let Some(profile) = target
                .profile
                .as_ref()
                .or(config.compiler.profile.as_ref())
                .and_then(|name| config.profiles.get(name))
            {
                global_paths.extend(profile.globals.clone());
            }
        }

        // final root set
        let global_module_ids = self.resolve_target_paths(
            revision,
            package_id,
            target_id,
            &package.path,
            &global_paths,
        )?;
        for module_id in global_module_ids {
            if !module_ids.contains(&module_id) {
                module_ids.push(module_id);
            }
        }

        Ok(module_ids)
    }

    /// Return the package that owns one target.
    fn target_package(
        &self,
        revision: Revision,
        package_id: PackageId,
        target_id: TargetId,
    ) -> Result<Arc<Package>, TargetDiscoveryError> {
        self.package(revision, package_id)
            .map_err(|error| TargetDiscoveryError::RepositoryRead {
                package: package_id,
                target: target_id,
                error: Box::new(error),
            })?
            .ok_or(TargetDiscoveryError::MissingPackage {
                package: package_id,
                target: target_id,
            })
    }

    /// Return one target or built-in.
    fn target_or_builtin_for_discovery(
        &self,
        revision: Revision,
        package_id: PackageId,
        target_id: TargetId,
    ) -> Result<Target, TargetDiscoveryError> {
        self.target_or_builtin(revision, target_id)
            .map_err(|error| TargetDiscoveryError::RepositoryRead {
                package: package_id,
                target: target_id,
                error: Box::new(error),
            })?
            .ok_or(TargetDiscoveryError::MissingTarget {
                package: package_id,
                target: target_id,
            })
    }

    /// Return the package config that contributes target globals.
    fn target_config(
        &self,
        revision: Revision,
        package_id: PackageId,
        target_id: TargetId,
    ) -> Result<Option<Arc<DestackFile>>, TargetDiscoveryError> {
        self.destack_for_package_id(revision, package_id)
            .map_err(|error| TargetDiscoveryError::RepositoryRead {
                package: package_id,
                target: target_id,
                error: Box::new(error),
            })
    }

    /// Resolve target paths relative to the package path.
    fn resolve_target_paths(
        &self,
        revision: Revision,
        package_id: PackageId,
        target_id: TargetId,
        package_path: &Option<PathBuf>,
        target_paths: &[PathBuf],
    ) -> Result<Vec<ModuleId>, TargetDiscoveryError> {
        let mut module_ids = Vec::with_capacity(target_paths.len());

        // package directory
        let package_directory =
            package_path
                .as_ref()
                .ok_or(TargetDiscoveryError::MissingPackagePath {
                    package: package_id,
                    target: target_id,
                })?;

        // target paths
        for target_path in target_paths {
            let resolved_path = package_directory.join(target_path);
            let Some(module_id) =
                self.package_module_id_for_path(revision, package_id, target_id, &resolved_path)?
            else {
                return Err(TargetDiscoveryError::MissingModulePath {
                    package: package_id,
                    target: target_id,
                    path: resolved_path,
                });
            };

            module_ids.push(module_id);
        }

        Ok(module_ids)
    }

    /// Resolve one repository path to a package-local module id.
    fn package_module_id_for_path(
        &self,
        revision: Revision,
        package_id: PackageId,
        target_id: TargetId,
        path: &Path,
    ) -> Result<Option<ModuleId>, TargetDiscoveryError> {
        let Some(module_id) = self.module_id_for_path(revision, path).map_err(|error| {
            TargetDiscoveryError::RepositoryRead {
                package: package_id,
                target: target_id,
                error: Box::new(error),
            }
        })?
        else {
            return Ok(None);
        };
        let Some(module) = self.module(revision, module_id).map_err(|error| {
            TargetDiscoveryError::RepositoryRead {
                package: package_id,
                target: target_id,
                error: Box::new(error),
            }
        })?
        else {
            return Ok(None);
        };

        // package boundary
        if module.package_id != package_id {
            return Ok(None);
        }

        Ok(Some(module_id))
    }

    /// Discover modules selected by target include rules.
    fn included_module_ids(
        &self,
        revision: Revision,
        package_id: PackageId,
        target_id: TargetId,
        package_path: &Option<PathBuf>,
        target: &Target,
    ) -> Result<Vec<ModuleId>, TargetDiscoveryError> {
        let mut module_ids = Vec::new();
        let candidate_ids = self
            .package_module_ids(revision, package_id)
            .map_err(|error| TargetDiscoveryError::RepositoryRead {
                package: package_id,
                target: target_id,
                error: Box::new(error),
            })?;

        // package modules
        for module_id in candidate_ids {
            let Some(module) = self.module(revision, module_id).map_err(|error| {
                TargetDiscoveryError::RepositoryRead {
                    package: package_id,
                    target: target_id,
                    error: Box::new(error),
                }
            })?
            else {
                continue;
            };
            let Some(module_path) = module.path.as_ref() else {
                continue;
            };

            // package relative path
            let path = package_path
                .as_ref()
                .and_then(|package_path| module_path.strip_prefix(package_path).ok())
                .unwrap_or(module_path);

            // include filter
            let is_included = target.include.is_empty()
                || target
                    .include
                    .iter()
                    .any(|pattern| Self::matches_target_glob(pattern, path));
            if !is_included {
                continue;
            }

            // exclude filter
            let is_excluded = target
                .exclude
                .iter()
                .any(|pattern| Self::matches_target_glob(pattern, path));
            if !is_excluded {
                module_ids.push(module.id);
            }
        }

        Ok(module_ids)
    }

    /// Match one target glob against one path.
    fn matches_target_glob(pattern: &str, path: &Path) -> bool {
        let pattern = pattern.as_bytes();
        let path = path.to_string_lossy();
        let path = path.as_bytes();

        glob_matches(pattern, 0, path, 0)
    }
}
