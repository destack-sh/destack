use super::parse_u128;
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
    pub fn from_source(id: destack_source::TargetId) -> Self {
        Self {
            package: PackageId::from_source(id.package_id),
            key: format!("{:032x}", id.target_key.raw()),
        }
    }

    /// Convert this bridge target id into one source target id.
    pub fn into_source(self) -> Result<destack_source::TargetId, SourceIdParseError> {
        let package = self.package.into_source()?;
        let key = parse_u128("target", &self.key)?;

        Ok(destack_source::TargetId {
            package_id: package,
            target_key: destack_source::TargetKey::new(key),
        })
    }
}

impl From<destack_source::TargetId> for TargetId {
    /// Convert one source target id into one bridge target id.
    fn from(id: destack_source::TargetId) -> Self {
        Self::from_source(id)
    }
}

impl TryFrom<TargetId> for destack_source::TargetId {
    type Error = SourceIdParseError;

    /// Convert one bridge target id into one source target id.
    fn try_from(id: TargetId) -> Result<Self, Self::Error> {
        id.into_source()
    }
}
