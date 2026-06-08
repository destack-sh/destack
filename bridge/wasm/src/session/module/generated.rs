// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

/// One module loaded through a live session.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct Module {
    module_id: String,
}

#[wasm_bindgen]
impl Module {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(module_id: String) -> Self {
        Self { module_id }
    }

    /// Displayed compiler module id.
    #[wasm_bindgen(getter, js_name = "moduleId")]
    pub fn module_id(&self) -> String {
        self.module_id.clone()
    }
}

impl Module {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::Module) -> Self {
        Self { module_id: value.module_id }
    }
}
