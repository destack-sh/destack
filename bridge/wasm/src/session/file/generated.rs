// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

/// One editable file visible to a live session.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct SessionFile {
    path: String,
}

#[wasm_bindgen]
impl SessionFile {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(path: String) -> Self {
        Self { path }
    }

    /// Repository logical path.
    #[wasm_bindgen(getter, js_name = "path")]
    pub fn path(&self) -> String {
        self.path.clone()
    }
}

impl SessionFile {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::SessionFile) -> Self {
        Self { path: value.path }
    }
}
