// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

/// External profile id crossing bridge boundaries.
#[derive(Debug)]
#[napi(object)]
pub struct ProfileId {
    /// Canonical lowercase hex profile id.
    pub id: String,
}

impl ProfileId {
    /// Convert this NAPI value into one bridge value.
    pub(crate) fn into_bridge(self) -> napi::Result<bridge::ProfileId> {
        Ok(bridge::ProfileId { id: self.id })
    }
}

impl ProfileId {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::ProfileId) -> Self {
        Self { id: value.id }
    }
}
