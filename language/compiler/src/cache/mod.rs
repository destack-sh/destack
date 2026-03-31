mod ast;
mod data;
mod dir;
mod environment;
mod graph;
mod hasher;
mod image;
mod mir;
mod output;
mod store;
#[cfg(test)]
mod tests;

use crate::compile::Compiler;

use destack_artifact::{ArtifactImageError, hash_bytes};
use destack_source::{File, FileContent, FileId, ModuleId, PackageId};
use destack_workspace::{Module, Package, Revision};
use std::sync::Arc;

pub(crate) use hasher::CacheHasher;
impl Compiler {
    /// Load one module snapshot for cache work.
    pub(crate) fn cache_module_snapshot(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Result<Arc<Module>, ArtifactImageError> {
        self.repository
            .module(revision, module_id)
            .map_err(|error| {
                ArtifactImageError::Io(std::io::Error::other(format!(
                    "failed to load module snapshot: {error}"
                )))
            })?
            .ok_or_else(|| ArtifactImageError::Io(std::io::Error::other("missing module snapshot")))
    }

    /// Load one file snapshot for cache work.
    pub(crate) fn cache_file_snapshot(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Arc<File>, ArtifactImageError> {
        self.repository
            .file(revision, file_id)
            .map_err(|error| {
                ArtifactImageError::Io(std::io::Error::other(format!(
                    "failed to load file snapshot: {error}"
                )))
            })?
            .ok_or_else(|| ArtifactImageError::Io(std::io::Error::other("missing file snapshot")))
    }

    /// Load one package snapshot for cache work.
    pub(crate) fn cache_package_snapshot(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Arc<Package>, ArtifactImageError> {
        self.repository
            .package(revision, package_id)
            .map_err(|error| {
                ArtifactImageError::Io(std::io::Error::other(format!(
                    "failed to load package snapshot: {error}"
                )))
            })?
            .ok_or_else(|| {
                ArtifactImageError::Io(std::io::Error::other("missing package snapshot"))
            })
    }

    /// Hash the current source content for one module.
    pub(crate) fn module_source_hash(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Option<u64> {
        let module = self.cache_module_snapshot(revision, module_id).ok()?;
        let file = self.cache_file_snapshot(revision, module.file_id).ok()?;

        match &file.content {
            FileContent::Text { content } => Some(hash_bytes(content.as_bytes())),
            FileContent::Json { content, .. } => Some(hash_bytes(content.as_bytes())),
            FileContent::Binary { content } => Some(hash_bytes(content)),
            FileContent::Missing => None,

            // unloaded files are not in the revision cache yet, so read them directly
            FileContent::Unloaded => {
                let path = module.path.as_ref()?;
                let bytes = self.repository.file_system().read(path).ok()?;

                Some(hash_bytes(&bytes))
            }
        }
    }
}
