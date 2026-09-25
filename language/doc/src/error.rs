use std::error::Error;
use std::fmt::{self, Display, Formatter};

use tspp_repository::{ProviderError, RepositoryError};

/// A documentation generation failure.
#[derive(Debug)]
pub enum DocError {
    /// Artifact access failed.
    Artifact(Box<ProviderError>),
    /// Repository state could not be read.
    Repository(RepositoryError),
    /// Compiler state violates its required shape.
    Invalid(String),
}

impl DocError {
    /// Build a missing compiler state error.
    pub(crate) fn missing(message: impl Into<String>) -> Self {
        Self::invalid(format!("missing {}", message.into()))
    }

    /// Build an invalid compiler state error.
    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }

    /// Build a cyclic compiler state error.
    pub(crate) fn cycle(message: impl Into<String>) -> Self {
        Self::invalid(format!("cyclic {}", message.into()))
    }
}

impl Display for DocError {
    /// Format the documentation failure.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Artifact(error) => write!(formatter, "artifact error: {error}"),
            Self::Repository(error) => write!(formatter, "repository error: {error}"),
            Self::Invalid(message) => write!(formatter, "invalid documentation state: {message}"),
        }
    }
}

impl Error for DocError {
    /// Return the underlying repository failure when one exists.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Artifact(error) => Some(error.as_ref()),
            Self::Repository(error) => Some(error),
            Self::Invalid(_) => None,
        }
    }
}

impl From<ProviderError> for DocError {
    /// Convert an artifact provider failure into a documentation failure.
    fn from(error: ProviderError) -> Self {
        Self::Artifact(Box::new(error))
    }
}

impl From<RepositoryError> for DocError {
    /// Convert a repository failure into a documentation failure.
    fn from(error: RepositoryError) -> Self {
        Self::Repository(error)
    }
}

/// A documentation generation result.
pub type DocResult<T> = Result<T, DocError>;
