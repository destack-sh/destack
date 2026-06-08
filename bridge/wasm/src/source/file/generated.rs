// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

/// External file id crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct FileId {
    id: String,
}

#[wasm_bindgen]
impl FileId {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(id: String) -> Self {
        Self { id }
    }

    /// Canonical lowercase hex file id.
    #[wasm_bindgen(getter, js_name = "id")]
    pub fn id(&self) -> String {
        self.id.clone()
    }
}

impl FileId {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::FileId) -> Self {
        Self { id: value.id }
    }
}

/// External file content id crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct FileContentId {
    id: String,
}

#[wasm_bindgen]
impl FileContentId {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(id: String) -> Self {
        Self { id }
    }

    /// Canonical lowercase hex file content id.
    #[wasm_bindgen(getter, js_name = "id")]
    pub fn id(&self) -> String {
        self.id.clone()
    }
}

impl FileContentId {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::FileContentId) -> Self {
        Self { id: value.id }
    }
}

/// Full file content crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct FileContent {
    content: FileContentContent,
}

/// Concrete payload enum content.
#[derive(Debug, Clone)]
enum FileContentContent {
    /// Text file content.
    Text {
        /// Text content.
        content: String,
    },
    /// Binary file content.
    Binary {
        /// Binary content.
        content: Vec<u8>,
    },
}

#[wasm_bindgen]
impl FileContent {
    /// Create one payload variant.
    #[wasm_bindgen(js_name = "text")]
    pub fn text(content: String) -> Self {
        Self {
            content: FileContentContent::Text { content },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "binary")]
    pub fn binary(content: Vec<u8>) -> Self {
        Self {
            content: FileContentContent::Binary { content },
        }
    }

    /// Payload variant label.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        let label = match &self.content {
            FileContentContent::Text { .. } => "text",
            FileContentContent::Binary { .. } => "binary",
        };
        label.to_string()
    }

    /// Text content.
    #[wasm_bindgen(getter, js_name = "textContent")]
    pub fn text_content(&self) -> Option<String> {
        match &self.content {
            FileContentContent::Text { content: value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    /// Binary content.
    #[wasm_bindgen(getter, js_name = "binaryContent")]
    pub fn binary_content(&self) -> Option<Vec<u8>> {
        match &self.content {
            FileContentContent::Binary { content: value, .. } => Some(value.clone()),
            _ => None,
        }
    }
}

impl FileContent {
    /// Convert one bridge payload enum into one WASM payload enum.
    pub(crate) fn from_bridge(value: bridge::FileContent) -> Self {
        match value {
            bridge::FileContent::Text { content } => Self {
                content: FileContentContent::Text { content },
            },
            bridge::FileContent::Binary { content } => Self {
                content: FileContentContent::Binary { content },
            },
        }
    }
}
