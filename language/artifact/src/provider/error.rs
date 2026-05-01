use std::error::Error;
use std::fmt::{self, Display, Formatter};

use crate::{ArtifactFailure, ArtifactKey, ArtifactVersion};

/// Error returned when one required artifact is not ready.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequireError {
    /// The required artifact must be built first.
    Blocked {
        /// The required artifact key.
        key: ArtifactKey,
    },
    /// The required artifact failed upstream.
    Failed {
        /// The required artifact key.
        key: ArtifactKey,
    },
    /// The required artifact record is ready but has the wrong payload shape.
    Corrupt {
        /// The corrupt artifact version.
        version: ArtifactVersion,
    },
}

impl RequireError {
    /// Build one blocked requirement error.
    pub fn blocked(key: ArtifactKey) -> Self {
        Self::Blocked { key }
    }
}

impl Display for RequireError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Blocked { key } => write!(formatter, "artifact requirement is blocked: {key:?}"),
            Self::Failed { key } => write!(formatter, "artifact requirement failed: {key:?}"),
            Self::Corrupt { version } => {
                write!(formatter, "artifact requirement is corrupt: {version:?}")
            }
        }
    }
}

impl Error for RequireError {}

/// Error returned by one provider attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvideError {
    /// The provider needs the listed artifact keys first.
    Blocked {
        /// The required artifact keys.
        keys: Vec<ArtifactKey>,
    },
    /// One required artifact failed upstream.
    RequirementFailed {
        /// The failed artifact key.
        key: ArtifactKey,
    },
    /// One required artifact was ready with the wrong payload shape.
    Corrupt {
        /// The corrupt artifact version.
        version: ArtifactVersion,
    },
    /// The provider failed this artifact without aborting the session run.
    Failed {
        /// The artifact failure.
        failure: ArtifactFailure,
    },
    /// The provider failed due to infrastructure or an invariant violation.
    Internal {
        /// The failure message.
        message: String,
    },
}

impl ProvideError {
    /// Build one internal provider error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Build one artifact provider failure.
    pub fn failed(failure: ArtifactFailure) -> Self {
        Self::Failed { failure }
    }
}

impl From<RequireError> for ProvideError {
    /// Convert one requirement error into provider control flow.
    fn from(error: RequireError) -> Self {
        match error {
            RequireError::Blocked { key } => Self::Blocked { keys: vec![key] },
            RequireError::Failed { key } => Self::RequirementFailed { key },
            RequireError::Corrupt { version } => Self::Corrupt { version },
        }
    }
}

impl Display for ProvideError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Blocked { keys } => write!(formatter, "artifact provider is blocked: {keys:?}"),
            Self::RequirementFailed { key } => {
                write!(formatter, "artifact provider requirement failed: {key:?}")
            }
            Self::Corrupt { version } => {
                write!(
                    formatter,
                    "artifact provider requirement is corrupt: {version:?}"
                )
            }
            Self::Failed { failure } => write!(formatter, "artifact provider failed: {failure:?}"),
            Self::Internal { message } => write!(formatter, "{message}"),
        }
    }
}

impl Error for ProvideError {}

/// Result returned by artifact providers.
pub type ProviderResult<T = crate::ArtifactPayload> = Result<T, ProvideError>;
