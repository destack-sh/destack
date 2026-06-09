// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::{ArtifactDependency, ArtifactSidecar, ArtifactVersion, Diagnostic};

/// One interned string carried by an artifact record.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct ArtifactString {
    id: String,
    text: String,
}

#[wasm_bindgen]
impl ArtifactString {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(id: String, text: String) -> Self {
        Self { id, text }
    }

    /// Canonical lowercase hex string id.
    #[wasm_bindgen(getter, js_name = "id")]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    /// Interned string text.
    #[wasm_bindgen(getter, js_name = "text")]
    pub fn text(&self) -> String {
        self.text.clone()
    }
}

impl ArtifactString {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::ArtifactString) -> Self {
        Self {
            id: value.id,
            text: value.text,
        }
    }
}

/// Self-contained raw artifact body crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct ArtifactRecord {
    version: ArtifactVersion,
    image: Vec<u8>,
    strings: Vec<ArtifactString>,
    dependencies: Vec<ArtifactDependency>,
    diagnostics: Vec<Diagnostic>,
    sidecars: Vec<ArtifactSidecar>,
}

#[wasm_bindgen]
impl ArtifactRecord {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(
        version: ArtifactVersion,
        image: Vec<u8>,
        strings: Vec<ArtifactString>,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: Vec<Diagnostic>,
        sidecars: Vec<ArtifactSidecar>,
    ) -> Self {
        Self {
            version,
            image,
            strings,
            dependencies,
            diagnostics,
            sidecars,
        }
    }

    /// The exact artifact version.
    #[wasm_bindgen(getter, js_name = "version")]
    pub fn version(&self) -> ArtifactVersion {
        self.version.clone()
    }

    /// Serialized artifact image bytes.
    #[wasm_bindgen(getter, js_name = "image")]
    pub fn image(&self) -> Vec<u8> {
        self.image.clone()
    }

    /// String pool needed to interpret interned ids in the payload.
    #[wasm_bindgen(getter, js_name = "strings")]
    pub fn strings(&self) -> Vec<ArtifactString> {
        self.strings.clone()
    }

    /// Exact artifact dependencies.
    #[wasm_bindgen(getter, js_name = "dependencies")]
    pub fn dependencies(&self) -> Vec<ArtifactDependency> {
        self.dependencies.clone()
    }

    /// Diagnostics recorded for this artifact version.
    #[wasm_bindgen(getter, js_name = "diagnostics")]
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.diagnostics.clone()
    }

    /// Artifact sidecars recorded for this artifact version.
    #[wasm_bindgen(getter, js_name = "sidecars")]
    pub fn sidecars(&self) -> Vec<ArtifactSidecar> {
        self.sidecars.clone()
    }
}

impl ArtifactRecord {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::ArtifactRecord) -> Self {
        Self {
            version: ArtifactVersion::from_bridge(value.version),
            image: value.image,
            strings: value
                .strings
                .into_iter()
                .map(|item| ArtifactString::from_bridge(item))
                .collect(),
            dependencies: value
                .dependencies
                .into_iter()
                .map(|item| ArtifactDependency::from_bridge(item))
                .collect(),
            diagnostics: value
                .diagnostics
                .into_iter()
                .map(|item| Diagnostic::from_bridge(item))
                .collect(),
            sidecars: value
                .sidecars
                .into_iter()
                .map(|item| ArtifactSidecar::from_bridge(item))
                .collect(),
        }
    }
}
