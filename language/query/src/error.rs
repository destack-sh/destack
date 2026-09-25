use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io;

use tspp_repository::{ProviderError, RepositoryError};

/// A query failure.
#[derive(Debug)]
pub enum QueryError {
    /// Artifact access failed.
    Artifact(Box<ProviderError>),
    /// Repository state could not be read.
    Repository(RepositoryError),
    /// Query source input or output failed.
    Io(io::Error),
    /// Query state violates its required shape.
    Invalid(String),
}

impl QueryError {
    /// Build a missing-state error.
    pub(crate) fn missing(message: impl Into<String>) -> Self {
        Self::invalid(format!("missing {}", message.into()))
    }

    /// Build an invalid-state error.
    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }

    /// Build a conflicting-state error.
    pub(crate) fn conflict(message: impl Into<String>) -> Self {
        Self::invalid(format!("conflicting {}", message.into()))
    }

    /// Build a cyclic-state error.
    pub(crate) fn cycle(message: impl Into<String>) -> Self {
        Self::invalid(format!("cyclic {}", message.into()))
    }
}

impl Display for QueryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Artifact(error) => write!(formatter, "artifact error: {error}"),
            Self::Repository(error) => write!(formatter, "repository error: {error}"),
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Invalid(message) => write!(formatter, "invalid query state: {message}"),
        }
    }
}

impl Error for QueryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Artifact(error) => Some(error.as_ref()),
            Self::Repository(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::Invalid(_) => None,
        }
    }
}

impl From<ProviderError> for QueryError {
    /// Convert an artifact provider failure into a query failure.
    fn from(error: ProviderError) -> Self {
        Self::Artifact(Box::new(error))
    }
}

impl From<Box<ProviderError>> for QueryError {
    /// Convert a boxed artifact provider failure into a query failure.
    fn from(error: Box<ProviderError>) -> Self {
        Self::Artifact(error)
    }
}

impl From<RepositoryError> for QueryError {
    /// Convert a repository failure into a query failure.
    fn from(error: RepositoryError) -> Self {
        Self::Repository(error)
    }
}

impl From<io::Error> for QueryError {
    /// Convert a source input or output failure into a query failure.
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// A query result.
pub type QueryResult<T> = Result<T, QueryError>;
