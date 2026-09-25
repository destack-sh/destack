use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use tspp_source::{FileId, PackageId};

use crate::ManifestFile;
use crate::config::MANIFEST_FILE_NAME;
use crate::repository::{Repository, RepositoryError, Revision};

impl Repository {
    /// Return one inherited manifest by workspace path.
    pub fn inherited_manifest_for_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<ManifestFile>, RepositoryError> {
        let mut active_paths = BTreeSet::new();

        self.inherit_manifest(revision, path, &mut active_paths)
    }

    /// Return the effective manifest for one manifest file id.
    pub fn manifest_for_file(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<Arc<ManifestFile>>, RepositoryError> {
        let Some(file) = self.file(revision, file_id)? else {
            return Ok(None);
        };
        let Some(path) = file.path.as_deref() else {
            return Ok(None);
        };
        let config = self.inherited_manifest_for_path(revision, path)?;

        Ok(config.map(Arc::new))
    }

    /// Return one local manifest by file id, none for an unmarked `package.json`.
    pub(crate) fn local_manifest_for_file(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<Arc<ManifestFile>>, RepositoryError> {
        // read file
        let Some(blob) = self.file_blob(revision, file_id)? else {
            return Ok(None);
        };

        // cached parse result
        let cache_key = (file_id, blob.id);
        if let Some(config) = self.file_cache.manifest.get(&cache_key) {
            return config
                .value()
                .clone()
                .map_err(|message| RepositoryError::InvalidConfig {
                    file: file_id,
                    message,
                });
        }

        // parse from source
        let Some(file) = self.file(revision, file_id)? else {
            return Ok(None);
        };
        let config = ManifestFile::parse(&file)
            .map(|config| config.map(Arc::new))
            .map_err(|error| error.to_string());
        let manifest = config
            .clone()
            .map_err(|message| RepositoryError::InvalidConfig {
                file: file_id,
                message,
            });

        // populate cache
        self.file_cache.manifest.insert(cache_key, config);

        manifest
    }

    /// Return the effective root workspace config for one revision.
    pub fn manifest_for_workspace(
        &self,
        revision: Revision,
    ) -> Result<Option<Arc<ManifestFile>>, RepositoryError> {
        let path = self.root.join(MANIFEST_FILE_NAME);
        let config = self.inherited_manifest_for_path(revision, &path)?;

        Ok(config.map(Arc::new))
    }

    /// Return one inherited manifest by workspace path.
    fn inherit_manifest(
        &self,
        revision: Revision,
        path: &Path,
        active_paths: &mut BTreeSet<PathBuf>,
    ) -> Result<Option<ManifestFile>, RepositoryError> {
        let Some(path) = normalize_path(path) else {
            return Ok(None);
        };
        let file_id = self.file_id(&path);

        // parse child config
        let Some(config) = self.local_manifest_for_file(revision, file_id)? else {
            return Ok(None);
        };
        if !active_paths.insert(path.clone()) {
            return Err(RepositoryError::ConfigCycle { path });
        }
        let mut config = config.as_ref().clone();

        // apply parents from nearest to furthest
        let parents = config.extends().map(str::to_string).collect::<Vec<_>>();
        for extends in parents {
            let parent_path = config_parent_path(&config, &extends)?;
            let Some(parent) = self.inherit_manifest(revision, &parent_path, active_paths)? else {
                return Err(self.missing_parent_error(revision, &config, &parent_path)?);
            };

            config
                .extend_from(&parent)
                .map_err(|error| RepositoryError::InvalidConfig {
                    file: config.file_id,
                    message: error.to_string(),
                })?;
        }

        active_paths.remove(&path);

        Ok(Some(config))
    }

    /// Return the error for one `extends` target that yields no manifest.
    fn missing_parent_error(
        &self,
        revision: Revision,
        config: &ManifestFile,
        parent_path: &Path,
    ) -> Result<RepositoryError, RepositoryError> {
        let path = parent_path.display().to_string();
        let is_present = self
            .file_blob(revision, self.file_id(parent_path))?
            .is_some();

        // report an existing parent without the TS++ marker
        let error = if is_present {
            RepositoryError::InvalidConfig {
                file: config.file_id,
                message: format!("extended {path} is not a TS++ manifest"),
            }
        }
        // report a missing parent
        else {
            RepositoryError::MissingFile { path }
        };

        Ok(error)
    }

    /// Return the package root paths for one revision.
    pub fn package_roots(&self, revision: Revision) -> Result<Vec<PathBuf>, RepositoryError> {
        let mut package_roots = self
            .package_index(revision)?
            .roots()
            .iter()
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>();

        package_roots.sort();
        package_roots.dedup();

        if package_roots.is_empty() {
            package_roots.push(self.root.clone());
        }

        Ok(package_roots)
    }

    /// Return the effective `package.json` config for one package id.
    pub fn manifest_for_package_id(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Option<Arc<ManifestFile>>, RepositoryError> {
        let package =
            self.package(revision, package_id)?
                .ok_or(RepositoryError::MissingPackage {
                    package: package_id,
                })?;

        Ok(package.configuration.clone())
    }
}

/// Resolve one config inheritance specifier.
fn config_parent_path(config: &ManifestFile, specifier: &str) -> Result<PathBuf, RepositoryError> {
    let specifier_path = Path::new(specifier);
    if specifier_path.components().next().is_none() {
        return Err(RepositoryError::InvalidConfigExtends {
            file: config.file_id,
            specifier: specifier.to_string(),
        });
    }

    let path = if specifier_path.is_absolute() {
        specifier_path.to_path_buf()
    } else {
        config.directory.join(specifier_path)
    };

    normalize_path(&path).ok_or_else(|| RepositoryError::InvalidConfigExtends {
        file: config.file_id,
        specifier: specifier.to_string(),
    })
}

/// Normalize one lexical workspace path.
fn normalize_path(path: &Path) -> Option<PathBuf> {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            Component::Normal(component) => normalized.push(component),
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }
    }

    Some(normalized)
}
