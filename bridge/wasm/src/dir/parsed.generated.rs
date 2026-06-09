// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::{ArtifactVersion, ModuleId};

/// Typed projection of one parsed DIR artifact.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct DirParsed {
    version: ArtifactVersion,
    module: ModuleId,
}

#[wasm_bindgen]
impl DirParsed {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(version: ArtifactVersion, module: ModuleId) -> Self {
        Self { version, module }
    }

    /// Exact parsed artifact version.
    #[wasm_bindgen(getter, js_name = "version")]
    pub fn version(&self) -> ArtifactVersion {
        self.version.clone()
    }

    /// Parsed module id.
    #[wasm_bindgen(getter, js_name = "module")]
    pub fn module(&self) -> ModuleId {
        self.module.clone()
    }
}

impl DirParsed {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::DirParsed) -> Self {
        Self {
            version: ArtifactVersion::from_bridge(value.version),
            module: ModuleId::from_bridge(value.module),
        }
    }
}
