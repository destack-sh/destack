use std::error::Error;
use std::fmt::{self, Display, Formatter};

use tspp_artifact::{ArtifactFailure, ArtifactKey, ArtifactPayload, ArtifactVersion};

/// Error returned by one provider attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
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

impl ProviderError {
    /// Build one blocked provider error from one required artifact.
    pub fn blocked(key: ArtifactKey) -> Self {
        Self::Blocked { keys: vec![key] }
    }

    /// Build one blocked provider error from many required artifacts.
    pub fn blocked_many(keys: Vec<ArtifactKey>) -> Self {
        Self::Blocked { keys }
    }

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

impl Display for ProviderError {
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

impl Error for ProviderError {}

/// Result returned by artifact providers.
pub type ProviderResult<T = ArtifactPayload> = Result<T, Box<ProviderError>>;
