// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::PackageId;

/// External product id crossing bridge boundaries.
#[derive(Debug)]
#[napi(object, js_name = "ProductId")]
pub struct ProductId {
    /// Owning package.
    pub package: PackageId,
    /// Canonical lowercase hex product key within the package.
    pub key: String,
}

impl ProductId {
    /// Convert this NAPI value into one bridge value.
    pub(crate) fn into_bridge(self) -> napi::Result<bridge::ProductId> {
        Ok(bridge::ProductId {
            package: self.package.into_bridge()?,
            key: self.key,
        })
    }
}

impl ProductId {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::ProductId) -> Self {
        Self {
            package: PackageId::from_bridge(value.package),
            key: value.key,
        }
    }
}
