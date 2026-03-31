use std::fmt;
use std::path::PathBuf;

use destack_source::{ModuleId, PackageId};

use crate::repository::ContentId;
use crate::revision::{Ref, Revision};

/// One error raised by repository operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositoryError {
    /// The requested ref does not exist.
    MissingRef { reference: Ref },
    /// The requested revision does not exist.
    MissingRevision { revision: Revision },
    /// The requested content payload does not exist.
    MissingContent { content: ContentId },
    /// The requested module does not exist in the given revision.
    MissingModule { module: ModuleId },
    /// The requested package does not exist in the given revision.
    MissingPackage { package: PackageId },
    /// The requested rename source does not exist in the base revision.
    MissingRenameSource { path: String },
    /// The requested edit path is not writable through generic repository edits.
    InvalidEditPath { path: String, message: String },
    /// Import from the attached file system failed.
    ImportFileSystem {
        operation: &'static str,
        path: PathBuf,
        message: String,
    },
    /// Repository image persistence failed.
    RepositoryImage {
        operation: &'static str,
        message: String,
    },
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRef { reference } => {
                write!(formatter, "missing repository ref '{reference}'")
            }
            Self::MissingRevision { revision } => {
                write!(formatter, "missing repository revision '{revision}'")
            }
            Self::MissingContent { content } => {
                write!(formatter, "missing repository content '{content}'")
            }
            Self::MissingModule { module } => {
                write!(formatter, "missing repository module '{module}'")
            }
            Self::MissingPackage { package } => {
                write!(formatter, "missing repository package '{package}'")
            }
            Self::MissingRenameSource { path } => {
                write!(formatter, "cannot rename missing file '{path}'")
            }
            Self::InvalidEditPath { path, message } => {
                write!(
                    formatter,
                    "invalid repository edit path '{path}': {message}"
                )
            }
            Self::ImportFileSystem {
                operation,
                path,
                message,
            } => {
                write!(
                    formatter,
                    "repository fs import failed during {operation} for '{}': {message}",
                    path.display()
                )
            }
            Self::RepositoryImage { operation, message } => {
                write!(formatter, "repository image {operation} failed: {message}")
            }
        }
    }
}

impl std::error::Error for RepositoryError {}
