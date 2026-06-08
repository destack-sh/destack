use destack_source as source;

use crate::source::parse_u128;
use crate::{PackageId, SourceIdParseError, bridge};

/// External module id crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleId {
    /// Owning package.
    pub package: PackageId,
    /// Canonical lowercase hex module key within the package.
    pub key: String,
}

impl ModuleId {
    /// Convert one source module id into one bridge module id.
    pub fn from_source(id: source::ModuleId) -> Self {
        Self {
            package: PackageId::from_source(id.package_id),
            key: format!("{:032x}", id.module_key.raw()),
        }
    }

    /// Convert this bridge module id into one source module id.
    pub fn into_source(self) -> Result<source::ModuleId, SourceIdParseError> {
        let package = self.package.into_source()?;
        let key = parse_u128("module", &self.key)?;

        Ok(source::ModuleId::new(package, key))
    }
}

impl From<source::ModuleId> for ModuleId {
    /// Convert one source module id into one bridge module id.
    fn from(id: source::ModuleId) -> Self {
        Self::from_source(id)
    }
}

impl TryFrom<ModuleId> for source::ModuleId {
    type Error = SourceIdParseError;

    /// Convert one bridge module id into one source module id.
    fn try_from(id: ModuleId) -> Result<Self, Self::Error> {
        id.into_source()
    }
}
