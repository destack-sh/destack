use std::path::Path;

use destack_source::FileContent;

use super::SourceError;

/// External source truth that can synchronize repository files.
pub(crate) trait Source {
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

    /// Return whether a full sync from this source owns one logical repository path.
    fn owns(&mut self, path: &Path) -> Result<bool, SourceError>;

    /// Return whether one logical repository path currently exists as a source file.
    fn exists(&mut self, path: &Path) -> Result<bool, SourceError>;
}
