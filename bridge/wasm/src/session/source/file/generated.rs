// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::ModuleId;

/// One source file in an explicit source snapshot.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct SourceFile {
    path: String,
    content: SourceFileContent,
}

#[wasm_bindgen]
impl SourceFile {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(path: String, content: SourceFileContent) -> Self {
        Self { path, content }
    }

    /// Repository-root relative path.
    #[wasm_bindgen(getter, js_name = "path")]
    pub fn path(&self) -> String {
        self.path.clone()
    }

    /// Full source file content.
    #[wasm_bindgen(getter, js_name = "content")]
    pub fn content(&self) -> SourceFileContent {
        self.content.clone()
    }
}

impl SourceFile {
    /// Convert this WASM value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::SourceFile {
        bridge::SourceFile {
            path: self.path,
            content: self.content.into_bridge(),
        }
    }
}

/// Full content for one source snapshot file.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct SourceFileContent {
    content: SourceFileContentContent,
}

/// Concrete payload enum content.
#[derive(Debug, Clone)]
enum SourceFileContentContent {
    /// Text file content.
    Text(String),
    /// Binary file content.
    Bytes(Vec<u8>),
}

#[wasm_bindgen]
impl SourceFileContent {
    /// Create one payload variant.
    #[wasm_bindgen(js_name = "text")]
    pub fn text(text: String) -> Self {
        Self {
            content: SourceFileContentContent::Text(text),
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "bytes")]
    pub fn bytes(bytes: Vec<u8>) -> Self {
        Self {
            content: SourceFileContentContent::Bytes(bytes),
        }
    }
}

impl SourceFileContent {
    /// Convert this WASM payload enum into one bridge enum.
    pub(crate) fn into_bridge(self) -> bridge::SourceFileContent {
        match self.content {
            SourceFileContentContent::Text(value) => bridge::SourceFileContent::Text(value),
            SourceFileContentContent::Bytes(value) => bridge::SourceFileContent::Bytes(value),
        }
    }
}

/// File update projected from a source update.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct FileUpdate {
    path: String,
    uri: String,
    kind: String,
    is_removed: bool,
    module_id: Option<ModuleId>,
}

#[wasm_bindgen]
impl FileUpdate {
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

    /// Coarse file update kind.
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

impl FileUpdate {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::FileUpdate) -> Self {
        Self {
            path: value.path,
            uri: value.uri,
            kind: file_update_kind_label(value.kind),
            is_removed: value.is_removed,
            module_id: value.module_id.map(|item| ModuleId::from_bridge(item)),
        }
    }
}

/// Return one target enum label.
fn file_update_kind_label(value: bridge::FileUpdateKind) -> String {
    let label = match value {
        bridge::FileUpdateKind::Source => "source",
        bridge::FileUpdateKind::Config => "config",
    };
    label.to_string()
}
