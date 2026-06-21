use std::fmt;

use destack_core::StringId;
use serde::ser;

use crate::{ArtifactVersion, BlobStoreError};

/// Errors that can occur while reading or writing artifact records.
#[derive(Debug)]
pub enum ArtifactStoreError {
    /// The record bytes are malformed or internally inconsistent.
    Corrupt(&'static str),
    /// The record version did not match the expected exact version.
    Version {
        /// The requested artifact version.
        expected: Box<ArtifactVersion>,
        /// The artifact version carried by the record.
        found: Box<ArtifactVersion>,
    },
    /// The record failed to encode or decode.
    Codec(Box<destack_serde::Error>),
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
    /// The store already has different bytes for the same exact version.
    Conflict {
        /// The conflicting exact artifact version.
        version: Box<ArtifactVersion>,
    },
    /// The artifact store failed to read or write.
    Store(Box<BlobStoreError>),
}

impl fmt::Display for ArtifactStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArtifactStoreError::Corrupt(message) => {
                write!(f, "corrupt artifact record: {message}")
            }
            ArtifactStoreError::Version { expected, found } => {
                write!(
                    f,
                    "unexpected artifact record version, expected {expected:?}, found {found:?}"
                )
            }
            ArtifactStoreError::Codec(error) => {
                write!(f, "artifact record codec error: {error}")
            }
            ArtifactStoreError::MissingString { string } => {
                write!(f, "artifact record references missing string {string}")
            }
            ArtifactStoreError::Size { limit, actual } => {
                write!(
                    f,
                    "artifact record exceeded size limit, limit {limit}, actual {actual}"
                )
            }
            ArtifactStoreError::Conflict { version } => {
                write!(f, "conflicting artifact record bytes for {version:?}")
            }
            ArtifactStoreError::Store(error) => {
                write!(f, "artifact store error: {error}")
            }
        }
    }
}

impl std::error::Error for ArtifactStoreError {}

impl ser::Error for ArtifactStoreError {
    fn custom<T: fmt::Display>(_message: T) -> Self {
        Self::Corrupt("artifact string id collection failed")
    }
}

impl From<std::io::Error> for ArtifactStoreError {
    fn from(error: std::io::Error) -> Self {
        Self::Store(Box::new(BlobStoreError::from(error)))
    }
}
