// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::ModuleId;

/// One file change observed by a session.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct Change {
    path: String,
    uri: String,
    is_removed: bool,
    module_id: Option<ModuleId>,
}

#[wasm_bindgen]
impl Change {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(path: String, uri: String, is_removed: bool, module_id: Option<ModuleId>) -> Self {
        Self {
            path,
            uri,
            is_removed,
            module_id,
        }
    }

    /// Repository logical path.
    #[wasm_bindgen(getter, js_name = "path")]
    pub fn path(&self) -> String {
        self.path.clone()
    }

    /// External file URI.
    #[wasm_bindgen(getter, js_name = "uri")]
    pub fn uri(&self) -> String {
        self.uri.clone()
    }

    /// Whether the file was removed.
    #[wasm_bindgen(getter, js_name = "isRemoved")]
    pub fn is_removed(&self) -> bool {
        self.is_removed
    }

    /// Updated module id when known.
    #[wasm_bindgen(getter, js_name = "moduleId")]
    pub fn module_id(&self) -> Option<ModuleId> {
        self.module_id.clone()
    }
}

impl Change {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::Change) -> Self {
        Self {
            path: value.path,
            uri: value.uri,
            is_removed: value.is_removed,
            module_id: value.module_id.map(|item| ModuleId::from_bridge(item)),
        }
    }
}
