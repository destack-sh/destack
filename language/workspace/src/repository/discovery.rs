use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{ModuleId, PackageId, TargetId, matches as glob_matches};

use crate::repository::{Repository, RepositoryError, Revision};
use crate::{DestackFile, Package, Target, TargetRoot};

impl Repository {
    /// Discover module ids selected by one target in one pinned revision.
    pub fn modules_for_target(
        &self,
        revision: Revision,
        target_id: TargetId,
    ) -> Result<Vec<ModuleId>, RepositoryError> {
        let package = self.target_package(revision, target_id)?;
        let target = self.required_target(revision, target_id)?;
        let config = self.target_config(revision, target_id)?;

        // selected roots
        let mut module_ids = match target.root() {
            TargetRoot::Entry => {
                self.resolve_target_paths(revision, target_id, &package.path, &target.entry)
            }
            TargetRoot::Include => {
                self.included_module_ids(revision, target_id, &package.path, &target)
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
        let global_module_ids =
            self.resolve_target_paths(revision, target_id, &package.path, &global_paths)?;
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
        target_id: TargetId,
    ) -> Result<Arc<Package>, RepositoryError> {
        let package_id = target_id.package_id();

        self.package(revision, package_id).and_then(|package| {
            package.ok_or(RepositoryError::MissingPackage {
                package: package_id,
            })
        })
    }

    /// Return one target or built-in target.
    fn required_target(
        &self,
        revision: Revision,
        target_id: TargetId,
    ) -> Result<Target, RepositoryError> {
        self.target_or_builtin(revision, target_id)
            .and_then(|target| target.ok_or(RepositoryError::MissingTarget { target: target_id }))
    }

    /// Return the package config that contributes target globals.
    fn target_config(
        &self,
        revision: Revision,
        target_id: TargetId,
    ) -> Result<Option<Arc<DestackFile>>, RepositoryError> {
        self.destack_for_package_id(revision, target_id.package_id())
    }

    /// Resolve target paths relative to the package path.
    fn resolve_target_paths(
        &self,
        revision: Revision,
        target_id: TargetId,
        package_path: &Option<PathBuf>,
        target_paths: &[PathBuf],
    ) -> Result<Vec<ModuleId>, RepositoryError> {
        let mut module_ids = Vec::with_capacity(target_paths.len());
        let package_id = target_id.package_id();
        if target_paths.is_empty() {
            return Ok(module_ids);
        }

        // package directory
        let package_directory =
            package_path
                .as_ref()
                .ok_or(RepositoryError::MissingPackagePath {
                    package: package_id,
                })?;

        // target paths
        for target_path in target_paths {
            let resolved_path = package_directory.join(target_path);
            let Some(module_id) =
                self.package_module_id_for_path(revision, package_id, &resolved_path)?
            else {
                return Err(RepositoryError::MissingModulePath {
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
        path: &Path,
    ) -> Result<Option<ModuleId>, RepositoryError> {
        let Some(module_id) = self.module_id_for_path(revision, path)? else {
            return Ok(None);
        };
        let Some(module) = self.module(revision, module_id)? else {
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
        target_id: TargetId,
        package_path: &Option<PathBuf>,
        target: &Target,
    ) -> Result<Vec<ModuleId>, RepositoryError> {
        let mut module_ids = Vec::new();
        let package_id = target_id.package_id();
        let candidate_ids = self.package_module_ids(revision, package_id)?;

        // package modules
        for module_id in candidate_ids {
            let Some(module) = self.module(revision, module_id)? else {
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
