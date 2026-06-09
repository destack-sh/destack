// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::ArtifactKey;

/// External artifact version crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct ArtifactVersion {
    key: ArtifactKey,
    fingerprint: String,
}

#[wasm_bindgen]
impl ArtifactVersion {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(key: ArtifactKey, fingerprint: String) -> Self {
        Self { key, fingerprint }
    }

    /// Semantic artifact slot.
    #[wasm_bindgen(getter, js_name = "key")]
    pub fn key(&self) -> ArtifactKey {
        self.key.clone()
    }

    /// Exact semantic fingerprint.
    #[wasm_bindgen(getter, js_name = "fingerprint")]
    pub fn fingerprint(&self) -> String {
        self.fingerprint.clone()
    }
}

impl ArtifactVersion {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::ArtifactVersion) -> Self {
        Self {
            key: ArtifactKey::from_bridge(value.key),
            fingerprint: value.fingerprint,
        }
    }
}
