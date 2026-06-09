// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::FileContent;

/// One stable sidecar label crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct ArtifactSidecarLabel {
    key: String,
    value: String,
}

#[wasm_bindgen]
impl ArtifactSidecarLabel {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(key: String, value: String) -> Self {
        Self { key, value }
    }

    /// Label key.
    #[wasm_bindgen(getter, js_name = "key")]
    pub fn key(&self) -> String {
        self.key.clone()
    }

    /// Label value.
    #[wasm_bindgen(getter, js_name = "value")]
    pub fn value(&self) -> String {
        self.value.clone()
    }
}

impl ArtifactSidecarLabel {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::ArtifactSidecarLabel) -> Self {
        Self {
            key: value.key,
            value: value.value,
        }
    }
}

/// One named artifact sidecar crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct ArtifactSidecar {
    name: String,
    labels: Vec<ArtifactSidecarLabel>,
    content: FileContent,
}

#[wasm_bindgen]
impl ArtifactSidecar {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(name: String, labels: Vec<ArtifactSidecarLabel>, content: FileContent) -> Self {
        Self {
            name,
            labels,
            content,
        }
    }

    /// Sidecar name.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Stable labels describing this sidecar.
    #[wasm_bindgen(getter, js_name = "labels")]
    pub fn labels(&self) -> Vec<ArtifactSidecarLabel> {
        self.labels.clone()
    }

    /// Sidecar content.
    #[wasm_bindgen(getter, js_name = "content")]
    pub fn content(&self) -> FileContent {
        self.content.clone()
    }
}

impl ArtifactSidecar {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::ArtifactSidecar) -> Self {
        Self {
            name: value.name,
            labels: value
                .labels
                .into_iter()
                .map(|item| ArtifactSidecarLabel::from_bridge(item))
                .collect(),
            content: FileContent::from_bridge(value.content),
        }
    }
}
