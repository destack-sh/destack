use std::{error, fmt};

use tspp_serde as serde;

/// An invalid artifact result or payload representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactError {
    /// Artifact state is malformed or internally inconsistent.
    Invalid(&'static str),
    /// An artifact payload failed to encode.
    Codec(Box<serde::Error>),
}

impl fmt::Display for ArtifactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => write!(formatter, "invalid artifact state: {message}"),
            Self::Codec(error) => write!(formatter, "artifact payload codec error: {error}"),
        }
    }
}

impl error::Error for ArtifactError {}

impl From<serde::Error> for ArtifactError {
    fn from(error: serde::Error) -> Self {
        Self::Codec(Box::new(error))
    }
}
