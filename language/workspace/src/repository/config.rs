use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{FileId, PackageId};

use crate::repository::{Repository, RepositoryError, Revision};
use crate::{DestackConfig, Package};

impl Repository {
    /// Return one parsed destack config by file id.
    pub fn destack_config_for_file(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<Arc<DestackConfig>>, RepositoryError> {
        // read file
        let Some(content_id) = self.file_content_id(revision, file_id)? else {
            return Ok(None);
        };

        // already cached
        if let Some(config) = self.file_cache.destack_configs.get(&content_id) {
            return Ok(config.value().as_ref().ok().cloned());
        }

        // parse from source
        let Some(file) = self.file(revision, file_id)? else {
            return Ok(None);
        };
        let config = DestackConfig::parse(&file)
            .map(Arc::new)
            .map_err(|error| error.to_string());
        let destack_config = config.as_ref().ok().cloned();

        // populate cache
        self.file_cache.destack_configs.insert(content_id, config);

        Ok(destack_config)
    }

    /// Return the parsed root workspace config for one revision.
    pub fn destack_config_for_workspace(
        &self,
        revision: Revision,
    ) -> Result<Option<Arc<DestackConfig>>, RepositoryError> {
        let file_id = self.file_id(&self.root.join("destack.json"));
        self.destack_config_for_file(revision, file_id)
    }

    /// Return one parsed `destack.json` config by workspace path.
    pub fn destack_config_for_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<Arc<DestackConfig>>, RepositoryError> {
        let file_id = self.file_id(path);

        self.destack_config_for_file(revision, file_id)
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

    /// Return the parsed `destack.json` config for one package.
    pub fn destack_config_for_package(
        &self,
        revision: Revision,
        package: &Package,
    ) -> Result<Option<Arc<DestackConfig>>, RepositoryError> {
        if let Some(destack_file_id) = package.destack_file_id {
            return self.destack_config_for_file(revision, destack_file_id);
        }

        let Some(package_path) = package.path.as_ref() else {
            return Ok(None);
        };

        let file_id = self.file_id(&package_path.join("destack.json"));
        self.destack_config_for_file(revision, file_id)
    }

    /// Return the parsed `destack.json` config for one package id.
    pub fn destack_config_for_package_id(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Option<Arc<DestackConfig>>, RepositoryError> {
        let Some(package) = self.package(revision, package_id)? else {
            return Ok(None);
        };

        self.destack_config_for_package(revision, package.as_ref())
    }
}
