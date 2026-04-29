use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use destack_source::{FileContentId, ModuleId, PackageId, ProfileId, TargetId};

use crate::repository::{Ref, Revision};

/// One error raised by repository operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositoryError {
    /// The requested ref does not exist.
    MissingRef { reference: Ref },
    /// The requested revision does not exist.
    MissingRevision { revision: Revision },
    /// The requested content payload does not exist.
    MissingContent { content: FileContentId },
    /// The requested module does not exist in the given revision.
    MissingModule { module: ModuleId },
    /// The requested package does not exist in the given revision.
    MissingPackage { package: PackageId },
    /// The requested target does not exist in the given revision.
    MissingTarget { target: TargetId },
    /// The requested profile does not exist in the repository.
    MissingProfile { profile: ProfileId },
    /// The requested file does not exist in the base revision.
    MissingFile { path: String },
    /// The requested file already exists in the base revision.
    FileAlreadyExists { path: String },
    /// The requested edit path is not writable through generic repository edits.
    InvalidEditPath { path: String, message: String },
    /// One attached file system operation failed.
    FileSystem {
        operation: &'static str,
        path: PathBuf,
        message: String,
    },
    /// Workspace root discovery or parsing failed.
    WorkspaceRootDiscovery { path: PathBuf, message: String },
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
            Self::MissingTarget { target } => {
                write!(formatter, "missing repository target '{target}'")
            }
            Self::MissingProfile { profile } => {
                write!(formatter, "missing repository profile '{profile}'")
            }
            Self::MissingFile { path } => {
                write!(formatter, "missing file '{path}'")
            }
            Self::FileAlreadyExists { path } => {
                write!(formatter, "file already exists '{path}'")
            }
            Self::InvalidEditPath { path, message } => {
                write!(
                    formatter,
                    "invalid repository edit path '{path}': {message}"
                )
            }
            Self::FileSystem {
                operation,
                path,
                message,
            } => {
                write!(
                    formatter,
                    "repository file system operation failed during {operation} for '{}': {message}",
                    path.display()
                )
            }
            Self::WorkspaceRootDiscovery { path, message } => {
                write!(
                    formatter,
                    "workspace root discovery failed for '{}': {message}",
                    path.display()
                )
            }
        }
    }
}

impl Error for RepositoryError {}
