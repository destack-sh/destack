// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::{FileId, Span};

/// One source replacement crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct Replacement {
    span: Span,
    new_text: String,
}

#[wasm_bindgen]
impl Replacement {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(span: Span, new_text: String) -> Self {
        Self { span, new_text }
    }

    /// Source span to replace.
    #[wasm_bindgen(getter, js_name = "span")]
    pub fn span(&self) -> Span {
        self.span.clone()
    }

    /// Replacement text.
    #[wasm_bindgen(getter, js_name = "newText")]
    pub fn new_text(&self) -> String {
        self.new_text.clone()
    }
}

impl Replacement {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::Replacement) -> Self {
        Self {
            span: Span::from_bridge(value.span),
            new_text: value.new_text,
        }
    }
}

/// Edits for a single file.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct FilePatch {
    file: FileId,
    replacements: Vec<Replacement>,
}

#[wasm_bindgen]
impl FilePatch {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(file: FileId, replacements: Vec<Replacement>) -> Self {
        Self { file, replacements }
    }

    /// Edited file.
    #[wasm_bindgen(getter, js_name = "file")]
    pub fn file(&self) -> FileId {
        self.file.clone()
    }

    /// Source replacements.
    #[wasm_bindgen(getter, js_name = "replacements")]
    pub fn replacements(&self) -> Vec<Replacement> {
        self.replacements.clone()
    }
}

impl FilePatch {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::FilePatch) -> Self {
        Self {
            file: FileId::from_bridge(value.file),
            replacements: value
                .replacements
                .into_iter()
                .map(|item| Replacement::from_bridge(item))
                .collect(),
        }
    }
}

/// Edits across multiple files.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct BatchEdit {
    files: Vec<FilePatch>,
}

#[wasm_bindgen]
impl BatchEdit {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(files: Vec<FilePatch>) -> Self {
        Self { files }
    }

    /// Per-file edits.
    #[wasm_bindgen(getter, js_name = "files")]
    pub fn files(&self) -> Vec<FilePatch> {
        self.files.clone()
    }
}

impl BatchEdit {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::BatchEdit) -> Self {
        Self {
            files: value
                .files
                .into_iter()
                .map(|item| FilePatch::from_bridge(item))
                .collect(),
        }
    }
}
