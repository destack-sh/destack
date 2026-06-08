// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::PackageId;

/// External module id crossing bridge boundaries.
#[derive(Debug)]
#[napi(object)]
pub struct ModuleId {
    /// Owning package.
    pub package: PackageId,
    /// Canonical lowercase hex module key within the package.
    pub key: String,
}

impl ModuleId {
    /// Convert this NAPI value into one bridge value.
    pub(crate) fn into_bridge(self) -> napi::Result<bridge::ModuleId> {
        Ok(bridge::ModuleId {
            package: self.package.into_bridge()?,
            key: self.key,
        })
    }
}

impl ModuleId {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::ModuleId) -> Self {
        Self {
            package: PackageId::from_bridge(value.package),
            key: value.key,
        }
    }
}
