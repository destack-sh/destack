mod ast;
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

use destack_artifact::ArtifactImageError;
use destack_source::{File, FileId, ModuleId, PackageId};
use destack_workspace::{Module, Package, Ref};
use std::sync::Arc;

pub(crate) use hasher::CacheHasher;

/// Resolve the current repository revision for cache reads.
fn repository_revision(
    compiler: &Compiler,
) -> Result<destack_workspace::Revision, ArtifactImageError> {
    if let Some(revision) = compiler.current_execution_revision() {
        return Ok(revision);
    }

    let reference = Ref::for_workspace_root(compiler.repository.workspace_root());

    compiler.repository.current(&reference).map_err(|error| {
        ArtifactImageError::Io(std::io::Error::other(format!(
            "failed to load current repository revision: {error}"
        )))
    })
}

/// Load one module snapshot for the current compiler revision.
pub(super) fn repository_module(
    compiler: &Compiler,
    module_id: ModuleId,
) -> Result<Arc<Module>, ArtifactImageError> {
    let revision = repository_revision(compiler)?;

    compiler
        .repository
        .module(revision, module_id)
        .map_err(|error| {
            ArtifactImageError::Io(std::io::Error::other(format!(
                "failed to load module snapshot: {error}"
            )))
        })?
        .ok_or_else(|| ArtifactImageError::Io(std::io::Error::other("missing module snapshot")))
}

/// Load one file snapshot for the current compiler revision.
pub(super) fn repository_file(
    compiler: &Compiler,
    file_id: FileId,
) -> Result<Arc<File>, ArtifactImageError> {
    let revision = repository_revision(compiler)?;

    compiler
        .repository
        .file(revision, file_id)
        .map_err(|error| {
            ArtifactImageError::Io(std::io::Error::other(format!(
                "failed to load file snapshot: {error}"
            )))
        })?
        .ok_or_else(|| ArtifactImageError::Io(std::io::Error::other("missing file snapshot")))
}

/// Load one package snapshot for the current compiler revision.
pub(super) fn repository_package(
    compiler: &Compiler,
    package_id: PackageId,
) -> Result<Arc<Package>, ArtifactImageError> {
    let revision = repository_revision(compiler)?;

    compiler
        .repository
        .package(revision, package_id)
        .map_err(|error| {
            ArtifactImageError::Io(std::io::Error::other(format!(
                "failed to load package snapshot: {error}"
            )))
        })?
        .ok_or_else(|| ArtifactImageError::Io(std::io::Error::other("missing package snapshot")))
}
