// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

/// External package id crossing bridge boundaries.
#[derive(Debug)]
#[napi(object)]
pub struct PackageId {
    /// Canonical lowercase hex package id.
    pub id: String,
}

impl PackageId {
    /// Convert this NAPI value into one bridge value.
    pub(crate) fn into_bridge(self) -> napi::Result<bridge::PackageId> {
        Ok(bridge::PackageId { id: self.id })
    }
}

impl PackageId {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::PackageId) -> Self {
        Self { id: value.id }
    }
}
