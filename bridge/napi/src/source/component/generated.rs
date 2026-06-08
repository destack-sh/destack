// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

/// External component id crossing bridge boundaries.
#[derive(Debug)]
#[napi(object)]
pub struct ComponentId {
    /// Canonical lowercase hex component id.
    pub id: String,
}

impl ComponentId {
    /// Convert this NAPI value into one bridge value.
    pub(crate) fn into_bridge(self) -> napi::Result<bridge::ComponentId> {
        Ok(bridge::ComponentId { id: self.id })
    }
}

impl ComponentId {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::ComponentId) -> Self {
        Self { id: value.id }
    }
}
