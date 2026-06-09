// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::PackageId;

/// External target id crossing bridge boundaries.
#[derive(Debug)]
#[napi(object)]
pub struct TargetId {
    /// Owning package.
    pub package: PackageId,
    /// Canonical lowercase hex target key within the package.
    pub key: String,
}

impl TargetId {
    /// Convert this NAPI value into one bridge value.
    pub(crate) fn into_bridge(self) -> napi::Result<bridge::TargetId> {
        Ok(bridge::TargetId {
            package: self.package.into_bridge()?,
            key: self.key,
        })
    }
}

impl TargetId {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::TargetId) -> Self {
        Self {
            package: PackageId::from_bridge(value.package),
            key: value.key,
        }
    }
}
