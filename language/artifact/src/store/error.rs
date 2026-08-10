use std::{error, fmt, io};

use destack_core::StringId;
use destack_program::ProgramLoadError;
use destack_serde as serde;

use ::serde::ser;

/// An artifact record or publication error.
#[derive(Debug)]
pub enum ArtifactError {
    /// Artifact state is malformed or internally inconsistent.
    Invalid(&'static str),
    /// The record failed to encode or decode.
    Codec(Box<serde::Error>),
    /// The program payload failed to load or store.
    Program(Box<ProgramLoadError>),
    /// The record references an interned string missing from the pool.
    MissingString {
        /// The missing string id.
        string: StringId,
    },
    /// Artifact persistence failed.
    Store {
        /// The storage failure.
        message: String,
    },
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
            ArtifactError::Store { message } => {
                write!(formatter, "artifact store error: {message}")
            }
        }
    }
}

impl error::Error for ArtifactError {}

impl ArtifactError {
    /// Build one artifact persistence failure.
    pub fn store(message: impl Into<String>) -> Self {
        Self::Store {
            message: message.into(),
        }
    }
}

impl ser::Error for ArtifactError {
    fn custom<T: fmt::Display>(_message: T) -> Self {
        Self::Invalid("artifact string id collection failed")
    }
}

impl From<ProgramLoadError> for ArtifactError {
    fn from(error: ProgramLoadError) -> Self {
        Self::Program(Box::new(error))
    }
}

impl From<io::Error> for ArtifactError {
    fn from(error: io::Error) -> Self {
        Self::store(error.to_string())
    }
}
