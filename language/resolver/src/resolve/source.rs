use std::io;
use std::path::Path;

use destack_artifact::ArtifactPathState;
use destack_source::{FileContent, FileContentId, FileId, FileMetadata, validate_utf8_string};

use crate::{Resolver, ResolverContext, ResolverError, ResolverResult, ResolverSource};

impl Resolver {
    /// Build the dependency file id for one resolver path.
    fn dependency_file_id(&self, path: &Path, ctx: &ResolverContext) -> FileId {
        let (repository, _) = self.repository_revision(ctx);

        if path.starts_with(repository.workspace_root()) {
            repository.file_id(path)
        } else {
            FileId::from_logical_path(path)
        }
    }

    /// Return the artifact path state for one metadata result.
    fn path_state(metadata: Option<FileMetadata>) -> ArtifactPathState {
        match metadata {
            None => ArtifactPathState::Missing,
            Some(metadata) if metadata.is_symlink => ArtifactPathState::Symlink,
            Some(metadata) if metadata.is_file => ArtifactPathState::File,
            Some(metadata) if metadata.is_directory => ArtifactPathState::Directory,
            Some(_) => ArtifactPathState::Other,
        }
    }

    /// Record one exact path metadata dependency.
    fn track_path_metadata(
        &self,
        path: &Path,
        metadata: Option<FileMetadata>,
        ctx: &mut ResolverContext,
    ) {
        let file_id = self.dependency_file_id(path, ctx);
        let state = Self::path_state(metadata);
        ctx.track_path_state(file_id, state);
    }

    /// Record one exact file content dependency.
    fn track_file_content(&self, path: &Path, content: &[u8], ctx: &mut ResolverContext) {
        let file_id = self.dependency_file_id(path, ctx);
        let content_id = FileContentId::for_binary(content);

        ctx.track_path_state(file_id, ArtifactPathState::File);
        ctx.track_file_content(file_id, content_id);
    }

    /// Record one exact file dependency from the active resolver source.
    pub(crate) fn track_file_dependency(
        &self,
        path: &Path,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<()> {
        if self.source == ResolverSource::FileSystem {
            let _metadata = self.path_metadata(path, ctx)?;

            return Ok(());
        }

        let (repository, revision) = self.repository_revision(ctx);
        let file_id = repository.file_id(path);
        let content_id = repository
            .file_content_id(revision, file_id)
            .map_err(|error| ResolverError::RepositoryError {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;

        let Some(content_id) = content_id else {
            self.track_path_metadata(path, None, ctx);

            return Ok(());
        };

        ctx.track_path_state(file_id, ArtifactPathState::File);
        ctx.track_file_content(file_id, content_id);

        Ok(())
    }

    /// Read a path as bytes.
    pub(crate) fn read_path(&self, path: &Path, ctx: &mut ResolverContext) -> io::Result<Vec<u8>> {
        let result = self.read_path_from_source(path, ctx);

        match &result {
            Ok(content) => self.track_file_content(path, content, ctx),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::InvalidInput
                ) =>
            {
                self.track_path_metadata(path, None, ctx);
            }
            Err(_) => {}
        }

        result
    }

    /// Read a path as bytes from the active resolver source without recording dependencies.
    fn read_path_from_source(&self, path: &Path, ctx: &ResolverContext) -> io::Result<Vec<u8>> {
        match self.source {
            ResolverSource::Revision => {
                let (repository, revision) = self.repository_revision(ctx);
                let file_id = repository.file_id(path);
                let file = repository
                    .file(revision, file_id)
                    .map_err(|error| io::Error::other(error.to_string()))?;

                let Some(file) = file else {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("revision file is missing: {path:?}"),
                    ));
                };

                match file.content.payload() {
                    FileContent::Text { content } => Ok(content.as_bytes().to_vec()),
                    FileContent::Binary { content } => Ok(content.clone()),
                }
            }
            ResolverSource::FileSystem => self.fs().read(path),
        }
    }

    /// Read a path as UTF-8 text.
    pub(crate) fn read_path_to_string(
        &self,
        path: &Path,
        ctx: &mut ResolverContext,
    ) -> io::Result<String> {
        let bytes = self.read_path(path, ctx)?;

        validate_utf8_string(bytes)
    }

    /// Read metadata from the active resolver source.
    fn path_metadata_from_source(
        &self,
        path: &Path,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Option<FileMetadata>> {
        match self.source {
            ResolverSource::Revision => {
                let (repository, revision) = self.repository_revision(ctx);
                repository.file_metadata(revision, path).map_err(|error| {
                    ResolverError::RepositoryError {
                        path: path.to_path_buf(),
                        message: error.to_string(),
                    }
                })
            }
            ResolverSource::FileSystem => match self.fs().metadata(path) {
                Ok(metadata) => Ok(Some(metadata)),
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::NotFound | io::ErrorKind::InvalidInput
                    ) =>
                {
                    Ok(None)
                }
                Err(error) => Err(ResolverError::IoError {
                    path: path.to_path_buf(),
                    kind: error.kind(),
                }),
            },
        }
    }

    /// Check if a path is a file.
    #[inline]
    pub(crate) fn is_file(&self, path: &Path, ctx: &mut ResolverContext) -> ResolverResult<bool> {
        let metadata = self.path_metadata(path, ctx)?;

        Ok(metadata.is_some_and(|metadata| metadata.is_file))
    }

    /// Return one path metadata snapshot from repository truth or the backing file system.
    pub(crate) fn path_metadata(
        &self,
        path: &Path,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Option<FileMetadata>> {
        if let Some(metadata) = ctx.path_metadata(path) {
            return Ok(metadata);
        }

        let metadata = self.path_metadata_from_source(path, ctx)?;
        ctx.cache_path_metadata(path, metadata);
        self.track_path_metadata(path, metadata, ctx);

        Ok(metadata)
    }
}
