// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

/// External revision value crossing bridge boundaries.
#[derive(Debug)]
#[napi(object)]
pub struct Revision {
    /// Displayed repository revision id.
    pub id: String,
}

impl Revision {
    /// Convert this NAPI value into one bridge value.
    pub(crate) fn into_bridge(self) -> napi::Result<bridge::Revision> {
        Ok(bridge::Revision { id: self.id })
    }
}

impl Revision {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::Revision) -> Self {
        Self { id: value.id }
    }
}
