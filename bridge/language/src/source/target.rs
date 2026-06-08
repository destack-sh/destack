use destack_source as source;

use crate::source::parse_u128;
use crate::{PackageId, SourceIdParseError, bridge};

/// External target id crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TargetId {
    /// Owning package.
    pub package: PackageId,
    /// Canonical lowercase hex target key within the package.
    pub key: String,
}

impl TargetId {
    /// Convert one source target id into one bridge target id.
    pub fn from_source(id: source::TargetId) -> Self {
        Self {
            package: PackageId::from_source(id.package_id),
            key: format!("{:032x}", id.target_key.raw()),
        }
    }

    /// Convert this bridge target id into one source target id.
    pub fn into_source(self) -> Result<source::TargetId, SourceIdParseError> {
        let package = self.package.into_source()?;
        let key = parse_u128("target", &self.key)?;

        Ok(source::TargetId {
            package_id: package,
            target_key: source::TargetKey::new(key),
        })
    }
}

impl From<source::TargetId> for TargetId {
    /// Convert one source target id into one bridge target id.
    fn from(id: source::TargetId) -> Self {
        Self::from_source(id)
    }
}

impl TryFrom<TargetId> for source::TargetId {
    type Error = SourceIdParseError;

    /// Convert one bridge target id into one source target id.
    fn try_from(id: TargetId) -> Result<Self, Self::Error> {
        id.into_source()
    }
}
