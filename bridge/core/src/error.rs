use std::fmt::{self, Display, Formatter};

use destack_workspace::RepositoryError;

/// Bridge core result.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors produced by the bridge core.
#[derive(Debug)]
pub enum Error {
    /// A session operation failed.
    Session {
        /// The failure detail.
        error: destack_session::SessionError,
    },
    /// A repository operation failed.
    Repository {
        /// The failure detail.
        error: RepositoryError,
    },
    /// A revision id is not a displayed workspace revision.
    InvalidRevision {
        /// The invalid revision id.
        revision: String,
    },
}

impl Display for Error {
    /// Format this bridge core error.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Session { error } => write!(formatter, "session error: {error}"),
            Self::Repository { error } => write!(formatter, "repository error: {error}"),
            Self::InvalidRevision { revision } => {
                write!(formatter, "invalid revision id: {revision}")
            }
        }
    }
}

impl std::error::Error for Error {
    /// Return the underlying error source.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Session { error } => Some(error),
            Self::Repository { error } => Some(error),
            Self::InvalidRevision { .. } => None,
        }
    }
}

impl From<destack_session::SessionError> for Error {
    /// Convert one session error into a bridge core error.
    fn from(error: destack_session::SessionError) -> Self {
        Self::Session { error }
    }
}

impl From<RepositoryError> for Error {
    /// Convert one repository error into a bridge core error.
    fn from(error: RepositoryError) -> Self {
        Self::Repository { error }
    }
}
