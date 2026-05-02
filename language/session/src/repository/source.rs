use std::path::Path;

use destack_source::FileContent;
use destack_workspace::RepositoryError;

/// A source of external truth that can be applied to or synchronized with a repository.
pub(crate) trait RepositorySource {
    /// The source-owned file descriptor.
    type File;

    /// List files visible to this source.
    fn list(&mut self) -> Result<Vec<Self::File>, RepositoryError>;

    /// Get one visible file by path.
    fn get(&mut self, path: &Path) -> Result<Option<Self::File>, RepositoryError>;

    /// Return the repository path for one source file.
    fn path<'file>(&self, file: &'file Self::File) -> &'file Path;

    /// Read one source file.
    fn read(&self, file: &Self::File) -> Result<FileContent, RepositoryError>;

    /// Return whether a full sync from this source owns one repository file path.
    fn tracks(&self, path: &Path) -> bool;

    /// Return whether one repository file path currently exists as a source file.
    fn has(&self, path: &Path) -> Result<bool, RepositoryError>;
}
