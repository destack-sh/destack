// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::{ArtifactVersion, ComponentId, ModuleId, ProfileId};

/// Typed projection of one checked DIR module artifact.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct DirChecked {
    version: ArtifactVersion,
    module: ModuleId,
    profile: ProfileId,
    component: ComponentId,
    entry: ModuleId,
}

#[wasm_bindgen]
impl DirChecked {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(
        version: ArtifactVersion,
        module: ModuleId,
        profile: ProfileId,
        component: ComponentId,
        entry: ModuleId,
    ) -> Self {
        Self {
            version,
            module,
            profile,
            component,
            entry,
        }
    }

    /// Exact checked facade artifact version.
    #[wasm_bindgen(getter, js_name = "version")]
    pub fn version(&self) -> ArtifactVersion {
        self.version.clone()
    }

    /// Checked module id.
    #[wasm_bindgen(getter, js_name = "module")]
    pub fn module(&self) -> ModuleId {
        self.module.clone()
    }

    /// Checked semantic profile.
    #[wasm_bindgen(getter, js_name = "profile")]
    pub fn profile(&self) -> ProfileId {
        self.profile.clone()
    }

    /// Component that owns the checked module output.
    #[wasm_bindgen(getter, js_name = "component")]
    pub fn component(&self) -> ComponentId {
        self.component.clone()
    }

    /// Component entry module.
    #[wasm_bindgen(getter, js_name = "entry")]
    pub fn entry(&self) -> ModuleId {
        self.entry.clone()
    }
}

impl DirChecked {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::DirChecked) -> Self {
        Self {
            version: ArtifactVersion::from_bridge(value.version),
            module: ModuleId::from_bridge(value.module),
            profile: ProfileId::from_bridge(value.profile),
            component: ComponentId::from_bridge(value.component),
            entry: ModuleId::from_bridge(value.entry),
        }
    }
}
