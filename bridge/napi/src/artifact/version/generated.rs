// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::ArtifactKey;

/// External artifact version crossing bridge boundaries.
#[derive(Debug)]
#[napi(object)]
pub struct ArtifactVersion {
    /// Semantic artifact slot.
    pub key: ArtifactKey,
    /// Exact semantic fingerprint.
    pub fingerprint: String,
}

impl ArtifactVersion {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::ArtifactVersion) -> Self {
        Self {
            key: ArtifactKey::from_bridge(value.key),
            fingerprint: value.fingerprint,
        }
    }
}
