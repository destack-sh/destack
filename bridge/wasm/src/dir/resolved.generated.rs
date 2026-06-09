// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::{ArtifactVersion, ModuleId, ProfileId};

/// Typed projection of one resolved DIR artifact.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct DirResolved {
    version: ArtifactVersion,
    module: ModuleId,
    profile: ProfileId,
}

#[wasm_bindgen]
impl DirResolved {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(version: ArtifactVersion, module: ModuleId, profile: ProfileId) -> Self {
        Self {
            version,
            module,
            profile,
        }
    }

    /// Exact resolved artifact version.
    #[wasm_bindgen(getter, js_name = "version")]
    pub fn version(&self) -> ArtifactVersion {
        self.version.clone()
    }

    /// Resolved module id.
    #[wasm_bindgen(getter, js_name = "module")]
    pub fn module(&self) -> ModuleId {
        self.module.clone()
    }

    /// Resolved semantic profile.
    #[wasm_bindgen(getter, js_name = "profile")]
    pub fn profile(&self) -> ProfileId {
        self.profile.clone()
    }
}

impl DirResolved {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::DirResolved) -> Self {
        Self {
            version: ArtifactVersion::from_bridge(value.version),
            module: ModuleId::from_bridge(value.module),
            profile: ProfileId::from_bridge(value.profile),
        }
    }
}
