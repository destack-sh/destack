use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use destack_source::{FileId, PackageId};

use crate::repository::{Repository, RepositoryError, Revision};
use crate::{DestackDeclaration, Package};

impl Repository {
    /// Return one inherited destack config by workspace path.
    pub fn inherited_destack_config_for_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<DestackDeclaration>, RepositoryError> {
        let mut active_paths = BTreeSet::new();

        self.inherit_destack_config(revision, path, &mut active_paths)
    }

    /// Return one local destack declaration by file id.
    fn local_destack_declaration_for_file(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<Arc<DestackDeclaration>>, RepositoryError> {
        // read file
        let Some(content_id) = self.file_content_id(revision, file_id)? else {
            return Ok(None);
        };

        // cached parse result
        if let Some(config) = self
            .file_cache
            .destack_config_by_content_id
            .get(&content_id)
        {
            return config
                .value()
                .as_ref()
                .map(|config| Some(Arc::clone(config)))
                .map_err(|message| RepositoryError::InvalidConfig {
                    file: file_id,
                    message: message.clone(),
                });
        }

        // parse from source
        let Some(file) = self.file(revision, file_id)? else {
            return Ok(None);
        };
        let config = DestackDeclaration::parse(&file)
            .map(Arc::new)
            .map_err(|error| error.to_string());
        let destack_config = config
            .as_ref()
            .map(|config| Some(Arc::clone(config)))
            .map_err(|message| RepositoryError::InvalidConfig {
                file: file_id,
                message: message.clone(),
            });

        // populate cache
        self.file_cache
            .destack_config_by_content_id
            .insert(content_id, config);

        destack_config
    }

    /// Return the effective root workspace config for one revision.
    pub fn destack_config_for_workspace(
        &self,
        revision: Revision,
    ) -> Result<Option<Arc<DestackDeclaration>>, RepositoryError> {
        let path = self.root.join("destack.json");
        let config = self.inherited_destack_config_for_path(revision, &path)?;

        Ok(config.map(Arc::new))
    }

    /// Return one inherited destack config by workspace path.
    fn inherit_destack_config(
        &self,
        revision: Revision,
        path: &Path,
        active_paths: &mut BTreeSet<PathBuf>,
    ) -> Result<Option<DestackDeclaration>, RepositoryError> {
        let Some(path) = normalize_path(path) else {
            return Ok(None);
        };
        let file_id = self.file_id(&path);

        // parse child config
        let Some(config) = self.local_destack_declaration_for_file(revision, file_id)? else {
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
            let Some(parent) = self.inherit_destack_config(revision, &parent_path, active_paths)?
            else {
                return Err(RepositoryError::MissingFile {
                    path: parent_path.display().to_string(),
                });
            };

            config
                .extend_from(&parent)
                .map_err(|error| RepositoryError::InvalidConfig {
                    file: config.file_id,
                    message: error.to_string(),
                })?;
        }

        // validate inherited invariants
        config
            .validate()
            .map_err(|error| RepositoryError::InvalidConfig {
                file: config.file_id,
                message: error.to_string(),
            })?;

        active_paths.remove(&path);

        Ok(Some(config))
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

    /// Return the effective `destack.json` config for one package.
    pub fn destack_config_for_package(
        &self,
        revision: Revision,
        package: &Package,
    ) -> Result<Option<Arc<DestackDeclaration>>, RepositoryError> {
        let Some(package_path) = package.path.as_ref() else {
            return Ok(None);
        };

        let path = package_path.join("destack.json");
        let config = self.inherited_destack_config_for_path(revision, &path)?;

        Ok(config.map(Arc::new))
    }

    /// Return the effective `destack.json` config for one package id.
    pub fn destack_config_for_package_id(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Option<Arc<DestackDeclaration>>, RepositoryError> {
        let Some(package) = self.package(revision, package_id)? else {
            return Ok(None);
        };

        self.destack_config_for_package(revision, package.as_ref())
    }
}

/// Resolve one config inheritance specifier.
fn config_parent_path(
    config: &DestackDeclaration,
    specifier: &str,
) -> Result<PathBuf, RepositoryError> {
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
