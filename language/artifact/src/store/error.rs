use std::fmt;

use destack_core::StringId;
use destack_program::ProgramLoadError;
use serde::ser;

use crate::BlobStoreError;

/// An artifact record or publication error.
#[derive(Debug)]
pub enum ArtifactError {
    /// Artifact state is malformed or internally inconsistent.
    Invalid(&'static str),
    /// The record failed to encode or decode.
    Codec(Box<destack_serde::Error>),
    /// The program payload failed to load or store.
    Program(Box<ProgramLoadError>),
    /// The record references an interned string missing from the pool.
    MissingString {
        /// The missing string id.
        string: StringId,
    },
    /// The record exceeded the configured size limit.
    Size {
        /// The configured size limit.
        limit: u64,
        /// The actual encoded byte length.
        actual: u64,
    },
    /// The artifact store failed to read or write.
    Store(Box<BlobStoreError>),
}

impl fmt::Display for ArtifactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArtifactError::Invalid(message) => {
                write!(formatter, "invalid artifact state: {message}")
            }
            ArtifactError::Codec(error) => {
                write!(formatter, "artifact record codec error: {error}")
            }
            ArtifactError::Program(error) => {
                write!(formatter, "program artifact error: {error}")
            }
            ArtifactError::MissingString { string } => {
                write!(
                    formatter,
                    "artifact record references missing string {string}"
                )
            }
            ArtifactError::Size { limit, actual } => {
                write!(
                    formatter,
                    "artifact record exceeded size limit, limit {limit}, actual {actual}"
                )
            }
            ArtifactError::Store(error) => {
                write!(formatter, "artifact store error: {error}")
            }
        }
    }
}

impl std::error::Error for ArtifactError {}

impl ser::Error for ArtifactError {
    fn custom<T: fmt::Display>(_message: T) -> Self {
        Self::Invalid("artifact string id collection failed")
    }
}

impl From<std::io::Error> for ArtifactError {
    fn from(error: std::io::Error) -> Self {
        Self::Store(Box::new(BlobStoreError::from(error)))
    }
}

impl From<ProgramLoadError> for ArtifactError {
    fn from(error: ProgramLoadError) -> Self {
        Self::Program(Box::new(error))
    }
}

impl From<BlobStoreError> for ArtifactError {
    fn from(error: BlobStoreError) -> Self {
        Self::Store(Box::new(error))
    }
}
