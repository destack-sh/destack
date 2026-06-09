// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::ModuleId;

/// Observed file change projected from a file update.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct FileChange {
    path: String,
    uri: String,
    kind: String,
    is_removed: bool,
    module_id: Option<ModuleId>,
}

#[wasm_bindgen]
impl FileChange {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(
        path: String,
        uri: String,
        kind: String,
        is_removed: bool,
        module_id: Option<ModuleId>,
    ) -> Self {
        Self {
            path,
            uri,
            kind,
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

    /// Coarse file change kind.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        self.kind.clone()
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

impl FileChange {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::FileChange) -> Self {
        Self {
            path: value.path,
            uri: value.uri,
            kind: file_change_kind_label(value.kind),
            is_removed: value.is_removed,
            module_id: value.module_id.map(|item| ModuleId::from_bridge(item)),
        }
    }
}

/// Return one target enum label.
fn file_change_kind_label(value: bridge::FileChangeKind) -> String {
    let label = match value {
        bridge::FileChangeKind::Source => "source",
        bridge::FileChangeKind::Config => "config",
    };
    label.to_string()
}
