use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use super::hash::stable_source_id;
use crate::PackageId;

const PRODUCT_DOMAIN: &[u8] = b"tspp.source.product.v1";

/// Stable key for one product within a package.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
#[serde(transparent)]
pub struct ProductKey(pub u64);

impl std::fmt::Display for ProductKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:016x}", self.0)
    }
}

impl ProductKey {
    /// Wrap a raw stable product key.
    pub const fn new(key: u64) -> Self {
        Self(key)
    }

    /// Return the raw stable key value.
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Unique identifier for one product within a package.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct ProductId {
    /// The owning package id.
    pub package_id: PackageId,
    /// The stable key for this product within its package.
    pub product_key: ProductKey,
}

impl std::fmt::Display for ProductId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "p{package_id:032x}:{product_key}",
            package_id = self.package_id.raw(),
            product_key = self.product_key
        )
    }
}

impl ProductId {
    /// Create a new ProductId.
    pub fn new(package_id: PackageId, name: impl AsRef<str>) -> Self {
        Self {
            package_id,
            product_key: ProductKey::new(stable_source_id(
                PRODUCT_DOMAIN,
                &[name.as_ref().as_bytes()],
            )),
        }
    }

    /// Return the owning package id.
    pub fn package_id(&self) -> PackageId {
        self.package_id
    }
}
