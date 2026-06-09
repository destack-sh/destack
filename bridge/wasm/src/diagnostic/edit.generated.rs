// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::{FileId, Span};

/// One source edit crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct Edit {
    span: Span,
    new_text: String,
}

#[wasm_bindgen]
impl Edit {
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

impl Edit {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::Edit) -> Self {
        Self {
            span: Span::from_bridge(value.span),
            new_text: value.new_text,
        }
    }
}

/// Edits for a single file.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct FileEdit {
    file: FileId,
    edits: Vec<Edit>,
}

#[wasm_bindgen]
impl FileEdit {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(file: FileId, edits: Vec<Edit>) -> Self {
        Self { file, edits }
    }

    /// Edited file.
    #[wasm_bindgen(getter, js_name = "file")]
    pub fn file(&self) -> FileId {
        self.file.clone()
    }

    /// Source edits.
    #[wasm_bindgen(getter, js_name = "edits")]
    pub fn edits(&self) -> Vec<Edit> {
        self.edits.clone()
    }
}

impl FileEdit {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::FileEdit) -> Self {
        Self {
            file: FileId::from_bridge(value.file),
            edits: value
                .edits
                .into_iter()
                .map(|item| Edit::from_bridge(item))
                .collect(),
        }
    }
}

/// Edits across multiple files.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct BatchEdit {
    files: Vec<FileEdit>,
}

#[wasm_bindgen]
impl BatchEdit {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(files: Vec<FileEdit>) -> Self {
        Self { files }
    }

    /// Per-file edits.
    #[wasm_bindgen(getter, js_name = "files")]
    pub fn files(&self) -> Vec<FileEdit> {
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
                .map(|item| FileEdit::from_bridge(item))
                .collect(),
        }
    }
}
