use super::parse_u128;
use crate::{PackageId, SourceIdParseError, bridge};

/// External product id crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProductId {
    /// Owning package.
    pub package: PackageId,
    /// Canonical lowercase hex product key within the package.
    pub key: String,
}

impl ProductId {
    /// Convert one source product id into one bridge product id.
    pub fn from_source(id: destack_source::ProductId) -> Self {
        Self {
            package: PackageId::from_source(id.package_id),
            key: format!("{:032x}", id.product_key.raw()),
        }
    }

    /// Convert this bridge product id into one source product id.
    pub fn into_source(self) -> Result<destack_source::ProductId, SourceIdParseError> {
        let package = self.package.into_source()?;
        let key = parse_u128("product", &self.key)?;

        Ok(destack_source::ProductId {
            package_id: package,
            product_key: destack_source::ProductKey::new(key),
        })
    }
}

impl From<destack_source::ProductId> for ProductId {
    /// Convert one source product id into one bridge product id.
    fn from(id: destack_source::ProductId) -> Self {
        Self::from_source(id)
    }
}

impl TryFrom<ProductId> for destack_source::ProductId {
    type Error = SourceIdParseError;

    /// Convert one bridge product id into one source product id.
    fn try_from(id: ProductId) -> Result<Self, Self::Error> {
        id.into_source()
    }
}
