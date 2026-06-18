// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::Module;

/// One document accepted by formatter operations.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct Document {
    content: DocumentContent,
}

/// Concrete payload enum content.
#[derive(Debug, Clone)]
enum DocumentContent {
    /// Repository module at one immutable revision.
    Module {
        /// Loaded source module.
        module: Module,
    },
    /// Ad hoc source text.
    Text {
        /// Display path used for parser language detection.
        path: String,
        /// Source text.
        text: String,
    },
}

#[wasm_bindgen]
impl Document {
    /// Create one payload variant.
    #[wasm_bindgen(js_name = "module")]
    pub fn module(module: Module) -> Self {
        Self {
            content: DocumentContent::Module { module },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "text")]
    pub fn text(path: String, text: String) -> Self {
        Self {
            content: DocumentContent::Text { path, text },
        }
    }
}

impl Document {
    /// Convert this WASM payload enum into one bridge enum.
    pub(crate) fn into_bridge(self) -> bridge::Document {
        match self.content {
            DocumentContent::Module { module } => bridge::Document::Module {
                module: module.into_bridge(),
            },
            DocumentContent::Text { path, text } => bridge::Document::Text { path, text },
        }
    }
}

/// One formatter request.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct FormatRequest {
    document: Document,
}

#[wasm_bindgen]
impl FormatRequest {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(document: Document) -> Self {
        Self { document }
    }

    /// Document to format.
    #[wasm_bindgen(getter, js_name = "document")]
    pub fn document(&self) -> Document {
        self.document.clone()
    }
}

impl FormatRequest {
    /// Convert this WASM value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::FormatRequest {
        bridge::FormatRequest {
            document: self.document.into_bridge(),
        }
    }
}

/// One formatter output.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct FormatOutput {
    text: String,
}

#[wasm_bindgen]
impl FormatOutput {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(text: String) -> Self {
        Self { text }
    }

    /// Formatted source text.
    #[wasm_bindgen(getter, js_name = "text")]
    pub fn text(&self) -> String {
        self.text.clone()
    }
}

impl FormatOutput {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::FormatOutput) -> Self {
        Self { text: value.text }
    }
}
