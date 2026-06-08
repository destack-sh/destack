use std::fmt::{self, Display, Formatter};

use destack_repository as repository;

use crate::bridge;

/// External revision value crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Revision {
    /// Displayed repository revision id.
    pub id: String,
}

impl Revision {
    /// Convert one repository revision into one bridge revision.
    pub fn from_repository(revision: repository::Revision) -> Self {
        Self {
            id: revision.to_string(),
        }
    }

    /// Convert this bridge revision into one repository revision.
    pub fn into_repository(self) -> Result<repository::Revision, RevisionParseError> {
        parse_repository_revision(&self.id)
    }
}

impl From<repository::Revision> for Revision {
    /// Convert one repository revision into one bridge revision.
    fn from(revision: repository::Revision) -> Self {
        Self::from_repository(revision)
    }
}

impl TryFrom<Revision> for repository::Revision {
    type Error = RevisionParseError;

    /// Convert one bridge revision into one repository revision.
    fn try_from(revision: Revision) -> Result<Self, Self::Error> {
        revision.into_repository()
    }
}

/// Error returned when a bridge revision id is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionParseError {
    /// The invalid revision id.
    pub revision: String,
}

impl Display for RevisionParseError {
    /// Format this revision parse error.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid revision id: {}", self.revision)
    }
}

impl std::error::Error for RevisionParseError {}

/// Parse one displayed repository revision.
pub fn parse_repository_revision(value: &str) -> Result<repository::Revision, RevisionParseError> {
    let Some(hex) = value.strip_prefix('r') else {
        return Err(RevisionParseError {
            revision: value.to_string(),
        });
    };
    if hex.len() != 64 {
        return Err(RevisionParseError {
            revision: value.to_string(),
        });
    }

    let mut bytes = [0; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let start = 2 * index;
        let end = start + 2;
        let parsed =
            u8::from_str_radix(&hex[start..end], 16).map_err(|_error| RevisionParseError {
                revision: value.to_string(),
            })?;
        *byte = parsed;
    }

    Ok(repository::Revision::new(bytes))
}
