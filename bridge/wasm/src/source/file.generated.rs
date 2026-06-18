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

/// External content id crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct ContentId {
    id: String,
}

#[wasm_bindgen]
impl ContentId {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(id: String) -> Self {
        Self { id }
    }

    /// Canonical lowercase hex content id.
    #[wasm_bindgen(getter, js_name = "id")]
    pub fn id(&self) -> String {
        self.id.clone()
    }
}

impl ContentId {
    /// Convert this WASM value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::ContentId {
        bridge::ContentId { id: self.id }
    }
}

impl ContentId {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::ContentId) -> Self {
        Self { id: value.id }
    }
}

/// Full content crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct Content {
    content: ContentContent,
}

/// Concrete payload enum content.
#[derive(Debug, Clone)]
enum ContentContent {
    /// Text content.
    Text {
        /// Text content.
        content: String,
    },
    /// Binary content.
    Binary {
        /// Binary content.
        content: Vec<u8>,
    },
}

#[wasm_bindgen]
impl Content {
    /// Create one payload variant.
    #[wasm_bindgen(js_name = "text")]
    pub fn text(content: String) -> Self {
        Self {
            content: ContentContent::Text { content },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "binary")]
    pub fn binary(content: Vec<u8>) -> Self {
        Self {
            content: ContentContent::Binary { content },
        }
    }

    /// Payload variant label.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        let label = match &self.content {
            ContentContent::Text { .. } => "text",
            ContentContent::Binary { .. } => "binary",
        };
        label.to_string()
    }

    /// Text content.
    #[wasm_bindgen(js_name = "getTextContent")]
    pub fn get_text_content(&self) -> Option<String> {
        match &self.content {
            ContentContent::Text { content: value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    /// Binary content.
    #[wasm_bindgen(js_name = "getBinaryContent")]
    pub fn get_binary_content(&self) -> Option<Vec<u8>> {
        match &self.content {
            ContentContent::Binary { content: value, .. } => Some(value.clone()),
            _ => None,
        }
    }
}

impl Content {
    /// Convert one bridge payload enum into one WASM payload enum.
    pub(crate) fn from_bridge(value: bridge::Content) -> Self {
        match value {
            bridge::Content::Text { content } => Self {
                content: ContentContent::Text { content },
            },
            bridge::Content::Binary { content } => Self {
                content: ContentContent::Binary { content },
            },
        }
    }
}
