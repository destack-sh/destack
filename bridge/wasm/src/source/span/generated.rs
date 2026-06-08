// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::FileId;

/// Source byte span crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct Span {
    file: FileId,
    start: u32,
    end: u32,
}

#[wasm_bindgen]
impl Span {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(file: FileId, start: u32, end: u32) -> Self {
        Self { file, start, end }
    }

    /// File containing this span.
    #[wasm_bindgen(getter, js_name = "file")]
    pub fn file(&self) -> FileId {
        self.file.clone()
    }

    /// Inclusive start byte offset.
    #[wasm_bindgen(getter, js_name = "start")]
    pub fn start(&self) -> u32 {
        self.start
    }

    /// Exclusive end byte offset.
    #[wasm_bindgen(getter, js_name = "end")]
    pub fn end(&self) -> u32 {
        self.end
    }
}

impl Span {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::Span) -> Self {
        Self {
            file: FileId::from_bridge(value.file),
            start: value.start,
            end: value.end,
        }
    }
}

/// Source span with a display label.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct LabeledSpan {
    span: Span,
    label: String,
}

#[wasm_bindgen]
impl LabeledSpan {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(span: Span, label: String) -> Self {
        Self { span, label }
    }

    /// Source span.
    #[wasm_bindgen(getter, js_name = "span")]
    pub fn span(&self) -> Span {
        self.span.clone()
    }

    /// Display label.
    #[wasm_bindgen(getter, js_name = "label")]
    pub fn label(&self) -> String {
        self.label.clone()
    }
}
