use std::fmt::{self, Display, Formatter};

use destack_source as source;

use crate::bridge;

/// External package id crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageId {
    /// Canonical lowercase hex package id.
    pub id: String,
}

impl PackageId {
    /// Convert one source package id into one bridge package id.
    pub fn from_source(id: source::PackageId) -> Self {
        Self {
            id: format!("{:032x}", id.raw()),
        }
    }

    /// Convert this bridge package id into one source package id.
    pub fn into_source(self) -> Result<source::PackageId, SourceIdParseError> {
        let id = parse_u128("package", &self.id)?;

        Ok(source::PackageId::new(id))
    }
}

impl From<source::PackageId> for PackageId {
    /// Convert one source package id into one bridge package id.
    fn from(id: source::PackageId) -> Self {
        Self::from_source(id)
    }
}

impl TryFrom<PackageId> for source::PackageId {
    type Error = SourceIdParseError;

    /// Convert one bridge package id into one source package id.
    fn try_from(id: PackageId) -> Result<Self, Self::Error> {
        id.into_source()
    }
}

/// Error returned when a bridge source id is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceIdParseError {
    /// Source id kind.
    pub kind: &'static str,
    /// Invalid id text.
    pub value: String,
}

impl Display for SourceIdParseError {
    /// Format this source id parse error.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid {} id: {}", self.kind, self.value)
    }
}

impl std::error::Error for SourceIdParseError {}

/// Parse one canonical u128 hex bridge id.
pub(crate) fn parse_u128(kind: &'static str, value: &str) -> Result<u128, SourceIdParseError> {
    if value.len() != 32 {
        return Err(SourceIdParseError {
            kind,
            value: value.to_string(),
        });
    }

    u128::from_str_radix(value, 16).map_err(|_error| SourceIdParseError {
        kind,
        value: value.to_string(),
    })
}
