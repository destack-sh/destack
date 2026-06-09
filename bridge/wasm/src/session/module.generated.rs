// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::ModuleId;

/// One module loaded through a live session.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct Module {
    id: ModuleId,
}

#[wasm_bindgen]
impl Module {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(id: ModuleId) -> Self {
        Self { id }
    }

    /// Stable source module id.
    #[wasm_bindgen(getter, js_name = "id")]
    pub fn id(&self) -> ModuleId {
        self.id.clone()
    }
}

impl Module {
    /// Convert this WASM value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::Module {
        bridge::Module {
            id: self.id.into_bridge(),
        }
    }
}

impl Module {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::Module) -> Self {
        Self {
            id: ModuleId::from_bridge(value.id),
        }
    }
}
