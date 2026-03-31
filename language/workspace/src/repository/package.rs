use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::PackageId;

use crate::{Destack, Package, PackageManifest};

use crate::repository::{Repository, RepositoryError};
use crate::revision::Revision;

impl Repository {
    /// Return the parsed root workspace config for one revision.
    pub fn workspace_config(&self, revision: Revision) -> Result<Option<Destack>, RepositoryError> {
        let config_path = self.root.join("destack.json");
        let Some(config_file) = self.file_at_path(revision, &config_path)? else {
            return Ok(None);
        };
        let Ok(config) = Destack::parse(&config_file) else {
            return Ok(None);
        };

        Ok(Some(config))
    }

    /// Return the discovered package paths for one revision.
    pub fn workspace_package_paths(
        &self,
        revision: Revision,
    ) -> Result<Vec<PathBuf>, RepositoryError> {
        let mut package_paths = self
            .workspace_package_ids(revision)?
            .into_iter()
            .filter_map(|package_id| {
                self.package(revision, package_id)
                    .ok()
                    .flatten()
                    .and_then(|package| package.path.clone())
            })
            .collect::<Vec<_>>();

        package_paths.sort();
        package_paths.dedup();

        if package_paths.is_empty() {
            package_paths.push(self.root.clone());
        }

        Ok(package_paths)
    }

    /// Return one package snapshot for one revision and package id.
    pub fn package(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Option<Arc<Package>>, RepositoryError> {
        let revision_id = revision;
        let revision = self.revision(revision_id)?;
        let packages = self.derive_packages(revision.source.as_ref());
        let mut package = if let Some(package) = packages.get(&package_id) {
            Package {
                id: package.id,
                kind: package.kind,
                uri: package.uri.clone(),
                path: package.path.clone(),
                name: None,
                version: None,
                manifest: None,
                config: None,
                tsconfig_file_id: package.tsconfig_file_id,
                targets: Default::default(),
            }
        } else {
            return Ok(None);
        };

        // revision backed files
        package.manifest = self.package_manifest_at(revision_id, &package)?;
        package.config = self.package_config_at(revision_id, &package)?;

        // revision backed manifest and config
        if let Some(config) = package.config.as_ref() {
            if let Some(manifest) = package.manifest.as_mut() {
                manifest.refresh_from_destack(Some(config));
                package.name = Some(manifest.name.clone());
                package.version = Some(manifest.version.clone());
            } else {
                let realpath = package.path.clone().unwrap_or_else(|| config.path.clone());
                let manifest = PackageManifest::from_destack(config, realpath);
                package.name = Some(manifest.name.clone());
                package.version = Some(manifest.version.clone());
                package.manifest = Some(manifest);
            }

            if !config.options.targets.is_empty() {
                package.targets = config
                    .options
                    .targets
                    .iter()
                    .map(|(name, options)| {
                        let target_id = self.intern_target_id(package.id, name);
                        let target = options.to_target(name);

                        (target_id, target)
                    })
                    .collect();
            }
        } else if let Some(manifest) = package.manifest.as_ref() {
            package.name = Some(manifest.name.clone());
            package.version = Some(manifest.version.clone());
        }

        Ok(Some(Arc::new(package)))
    }

    /// Return the package ids visible in one revision.
    pub fn workspace_package_ids(
        &self,
        revision: Revision,
    ) -> Result<Vec<PackageId>, RepositoryError> {
        let mut package_ids = self
            .workspace_module_ids(revision)?
            .into_iter()
            .map(|module_id| module_id.package_id)
            .collect::<Vec<_>>();
        package_ids.sort_unstable();
        package_ids.dedup();

        Ok(package_ids)
    }

    /// Return the parsed package manifest for one package.
    fn package_manifest_at(
        &self,
        revision: Revision,
        package: &Package,
    ) -> Result<Option<PackageManifest>, RepositoryError> {
        let manifest_file = if let Some(manifest) = package.manifest.as_ref() {
            self.file(revision, manifest.file_id)?
        } else if let Some(package_path) = package.path.as_ref() {
            let manifest_path = package_path.join("package.json");
            self.file_at_path(revision, &manifest_path)?
        } else {
            None
        };
        let Some(manifest_file) = manifest_file else {
            return Ok(None);
        };

        let realpath = package.path.clone().unwrap_or_else(|| {
            manifest_file
                .path
                .as_ref()
                .and_then(|path| path.parent())
                .unwrap_or(&self.root)
                .to_path_buf()
        });
        let Ok(manifest) = PackageManifest::parse(&manifest_file, realpath) else {
            return Ok(None);
        };

        Ok(Some(manifest))
    }

    /// Return the parsed package config for one package.
    fn package_config_at(
        &self,
        revision: Revision,
        package: &Package,
    ) -> Result<Option<Destack>, RepositoryError> {
        let config_file = if let Some(config) = package.config.as_ref() {
            self.file(revision, config.file_id)?
        } else if let Some(package_path) = package.path.as_ref() {
            let config_path = package_path.join("destack.json");
            self.file_at_path(revision, &config_path)?
        } else {
            None
        };
        let Some(config_file) = config_file else {
            return Ok(None);
        };
        let Ok(config) = Destack::parse(&config_file) else {
            return Ok(None);
        };

        Ok(Some(config))
    }

    /// Return the package module type for one package.
    pub(crate) fn package_module_type_at(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Option<String>, RepositoryError> {
        let Some(package) = self.package(revision, package_id)? else {
            return Ok(None);
        };

        Ok(package
            .manifest
            .as_ref()
            .and_then(|manifest| manifest.content.module_type.clone()))
    }

    /// Return one file by workspace path.
    fn file_at_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<Arc<destack_source::File>>, RepositoryError> {
        let file_id = self.file_id_for_workspace_path(path);
        self.file(revision, file_id)
    }
}
