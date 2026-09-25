use std::io;
use std::path::{Path, PathBuf};

use tspp_core::Blob;
use tspp_query::QueryError;
use tspp_repository::{RepositoryError, Revision};
use tspp_session::SessionError;
use tspp_source::PackageId;

/// Failure from a workspace operation.
#[derive(Debug)]
pub enum Error {
    /// This workspace is closed.
    WorkspaceClosed,
    /// A path is outside the workspace root.
    PathNotInRoot {
        /// The path that failed root routing.
        path: PathBuf,
    },
    /// A requested file is missing from the current revision.
    FileMissing {
        /// The missing file path.
        path: PathBuf,
    },
    /// A filesystem path cannot be loaded as a module.
    ModuleNotLoadable {
        /// The path that could not be loaded.
        path: PathBuf,
        /// The failure detail.
        detail: String,
    },
    /// The requested branch does not exist.
    MissingBranch {
        /// Missing branch name.
        name: String,
    },
    /// The requested branch already exists.
    BranchExists {
        /// Existing branch name.
        name: String,
    },
    /// The requested text change is invalid.
    InvalidTextChange {
        /// The changed file path.
        path: PathBuf,
        /// The validation failure detail.
        detail: String,
    },
    /// The requested edit is not valid for this workspace operation.
    InvalidEdit {
        /// The validation failure detail.
        detail: String,
    },
    /// A semantic workspace watch fell behind its root.
    WatchLagged {
        /// Root that advanced beyond the pending watch commit.
        root: PathBuf,
        /// Watched branch, absent for physical state.
        branch: Option<String>,
    },
    /// A semantic workspace watch lost its closed root.
    WatchClosed {
        /// Root closed while it was watched.
        root: PathBuf,
        /// Watched branch, absent for physical state.
        branch: Option<String>,
    },
    /// A semantic workspace watch lost its removed branch.
    WatchRemoved {
        /// Root containing the removed branch.
        root: PathBuf,
        /// Removed branch name.
        branch: String,
    },
    /// The host stopped watching a workspace root.
    WatchFailed {
        /// Root whose host watch failed.
        root: PathBuf,
        /// The host watch failure detail.
        detail: String,
    },
    /// The expected revision does not match the current revision.
    StaleRevision {
        /// The caller expected revision.
        expected: Revision,
        /// The current revision.
        current: Revision,
    },
    /// A physical file no longer matches the workspace revision.
    FileChanged {
        /// Physical file that changed.
        path: Box<Path>,
        /// Blob expected by the workspace revision.
        expected: Option<Blob>,
        /// Blob observed on the filesystem.
        actual: Option<Blob>,
    },
    /// A package has no selected query target.
    TargetNotSelected {
        /// The package requiring a target selection.
        package_id: PackageId,
    },
    /// Repository work failed inside the workspace.
    Repository(RepositoryError),
    /// Session work failed inside the workspace.
    Session(Box<SessionError>),
    /// Semantic query execution failed.
    Query(Box<QueryError>),
    /// Filesystem work failed inside the workspace.
    Io {
        /// The path that failed.
        path: PathBuf,
        /// The filesystem failure.
        source: io::Error,
    },
    /// Restoring source files after a failed operation also failed.
    RollbackFailed {
        /// The original operation failure.
        operation: Box<Error>,
        /// Every restoration failure.
        failures: Vec<Error>,
    },
    /// Internal workspace failure.
    Internal {
        /// The failure detail.
        detail: String,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::WorkspaceClosed => write!(formatter, "workspace is closed"),
            Error::PathNotInRoot { path } => {
                write!(
                    formatter,
                    "path is outside workspace root: {}",
                    path.display()
                )
            }
            Error::FileMissing { path } => {
                write!(formatter, "file is missing: {}", path.display())
            }
            Error::ModuleNotLoadable { path, detail } => {
                write!(
                    formatter,
                    "module is not loadable for {}: {detail}",
                    path.display()
                )
            }
            Error::MissingBranch { name } => write!(formatter, "missing workspace branch '{name}'"),
            Error::BranchExists { name } => {
                write!(formatter, "workspace branch already exists: '{name}'")
            }
            Error::InvalidTextChange { path, detail } => {
                write!(
                    formatter,
                    "invalid text change for {}: {detail}",
                    path.display()
                )
            }
            Error::InvalidEdit { detail } => {
                write!(formatter, "invalid edit: {detail}")
            }
            Error::WatchLagged { root, branch } => {
                write!(
                    formatter,
                    "workspace watch lagged behind {}",
                    root.display()
                )?;
                if let Some(branch) = branch {
                    write!(formatter, " for branch '{branch}'")?;
                }

                Ok(())
            }
            Error::WatchClosed { root, branch } => {
                write!(
                    formatter,
                    "workspace closed while {} was watched",
                    root.display()
                )?;
                if let Some(branch) = branch {
                    write!(formatter, " on branch '{branch}'")?;
                }

                Ok(())
            }
            Error::WatchRemoved { root, branch } => {
                write!(
                    formatter,
                    "workspace branch '{branch}' was removed while watched: {}",
                    root.display()
                )
            }
            Error::WatchFailed { root, detail } => {
                write!(
                    formatter,
                    "workspace watch failed for {}: {detail}",
                    root.display()
                )
            }
            Error::StaleRevision { expected, current } => {
                write!(
                    formatter,
                    "stale revision: expected {expected}, current {current}"
                )
            }
            Error::FileChanged {
                path,
                expected,
                actual,
            } => {
                write!(
                    formatter,
                    "physical file changed at {}: expected {expected:?}, actual {actual:?}",
                    path.display()
                )
            }
            Error::TargetNotSelected { package_id } => {
                write!(
                    formatter,
                    "no query target is selected for package {package_id:?}"
                )
            }
            Error::Repository(error) => {
                write!(formatter, "repository error: {error}")
            }
            Error::Session(error) => {
                write!(formatter, "session error: {error}")
            }
            Error::Query(error) => {
                write!(formatter, "query error: {error}")
            }
            Error::Io { path, source } => {
                write!(
                    formatter,
                    "filesystem error at {}: {source}",
                    path.display()
                )
            }
            Error::RollbackFailed {
                operation,
                failures,
            } => {
                write!(formatter, "{operation}; rollback failed: ")?;

                // append every restoration failure without a trailing separator
                for (index, failure) in failures.iter().enumerate() {
                    if index > 0 {
                        write!(formatter, "; ")?;
                    }
                    write!(formatter, "{failure}")?;
                }

                Ok(())
            }
            Error::Internal { detail } => {
                write!(formatter, "workspace internal error: {detail}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Repository(error) => Some(error),
            Error::Session(error) => Some(error),
            Error::Query(error) => Some(error),
            Error::Io { source, .. } => Some(source),
            Error::RollbackFailed { operation, .. } => Some(operation),
            _ => None,
        }
    }
}

impl From<RepositoryError> for Error {
    fn from(error: RepositoryError) -> Self {
        Error::Repository(error)
    }
}

impl From<SessionError> for Error {
    fn from(error: SessionError) -> Self {
        Error::Session(Box::new(error))
    }
}

impl From<QueryError> for Error {
    fn from(error: QueryError) -> Self {
        Error::Query(Box::new(error))
    }
}
