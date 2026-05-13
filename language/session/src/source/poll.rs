use std::path::Path;

use destack_source::{FileContent, FileContentId, FileId};
use destack_workspace::{Edit, Repository, RepositoryChange, Revision};

use super::SourceError;
use crate::SessionError;

/// External source truth expressed in repository paths.
pub(crate) trait RepositorySource {
    /// The source-owned file descriptor.
    type File;

    /// List files visible to this source.
    fn list(&mut self) -> Result<Vec<Self::File>, SourceError>;

    /// Get one visible file by logical repository path.
    fn get(&mut self, path: &Path) -> Result<Option<Self::File>, SourceError>;

    /// Return the logical repository path for one source file.
    fn path<'file>(&self, file: &'file Self::File) -> &'file Path;

    /// Read one source file.
    fn read(&mut self, file: &Self::File) -> Result<FileContent, SourceError>;

    /// Return whether a full poll from this source owns one logical repository path.
    fn owns(&mut self, path: &Path) -> Result<bool, SourceError>;

    /// Return whether one logical repository path currently exists as a source file.
    fn exists(&mut self, path: &Path) -> Result<bool, SourceError>;

    /// Poll repository edits from this source against one revision.
    fn poll(
        &mut self,
        repository: &Repository,
        revision: Revision,
        filter: RepositorySourceFilter<Self::File>,
    ) -> Result<RepositoryChange, SessionError>
    where
        Self: Sized,
    {
        let mut change = RepositoryChange::new();

        // select source files
        let is_full_poll = matches!(filter, RepositorySourceFilter::All);
        let files = match filter {
            RepositorySourceFilter::All => self.list()?,
            RepositorySourceFilter::Files(files) => files,
        };

        // collect changed source files
        for file in files {
            let path = self.path(&file);
            let logical_path = path.to_string_lossy().to_string();
            let file_id = FileId::from_logical_path(path);
            let content = self.read(&file)?;

            if is_changed_file(repository, revision, file_id, &content)? {
                change.push(Edit::SetFile {
                    logical_path,
                    content,
                });
            }
        }

        // full polls also remove files no longer present in this source
        if is_full_poll {
            self.poll_removed_files(repository, revision, &mut change)?;
        }

        Ok(change)
    }

    /// Add removals for revision files no longer present in this source.
    fn poll_removed_files(
        &mut self,
        repository: &Repository,
        revision: Revision,
        change: &mut RepositoryChange,
    ) -> Result<(), SessionError>
    where
        Self: Sized,
    {
        // compare revision files with current source presence
        for file_id in repository.file_ids(revision)? {
            let Some(logical_path) = repository.file_logical_path(revision, file_id)? else {
                continue;
            };
            let path = Path::new(&logical_path);

            if self.owns(path)? && !self.exists(path)? {
                change.push(Edit::remove_file(logical_path));
            }
        }

        Ok(())
    }
}

/// File filter for a repository source poll.
pub(crate) enum RepositorySourceFilter<F> {
    /// Poll all visible files and removals.
    All,
    /// Poll selected files without removals.
    Files(Vec<F>),
}

impl<F> RepositorySourceFilter<F> {
    /// Build a selected file filter.
    pub(crate) fn files(files: impl IntoIterator<Item = F>) -> Self {
        Self::Files(files.into_iter().collect())
    }
}

/// Return whether one source file differs from the current revision.
fn is_changed_file(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
    content: &FileContent,
) -> Result<bool, SessionError> {
    let current = repository.file_content_id(revision, file_id)?;
    let incoming = FileContentId::for_content(content);

    Ok(current != Some(incoming))
}
