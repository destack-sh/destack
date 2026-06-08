// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::{FileUpdate, Revision};

/// Source text range in byte offsets.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct TextRange {
    start: u32,
    end: u32,
}

#[wasm_bindgen]
impl TextRange {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
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

impl TextRange {
    /// Convert this WASM value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::TextRange {
        bridge::TextRange {
            start: self.start,
            end: self.end,
        }
    }
}

/// Source text replacement.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct TextEdit {
    range: TextRange,
    text: String,
}

#[wasm_bindgen]
impl TextEdit {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(range: TextRange, text: String) -> Self {
        Self { range, text }
    }

    /// Replaced byte range.
    #[wasm_bindgen(getter, js_name = "range")]
    pub fn range(&self) -> TextRange {
        self.range.clone()
    }

    /// Replacement text.
    #[wasm_bindgen(getter, js_name = "text")]
    pub fn text(&self) -> String {
        self.text.clone()
    }
}

impl TextEdit {
    /// Convert this WASM value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::TextEdit {
        bridge::TextEdit {
            range: self.range.into_bridge(),
            text: self.text,
        }
    }
}

/// One source edit accepted by a session update.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct SourceEdit {
    content: SourceEditContent,
}

/// Concrete payload enum content.
#[derive(Debug, Clone)]
enum SourceEditContent {
    /// Replace or create one text file.
    SetText {
        /// Repository logical path.
        path: String,
        /// Full text content.
        text: String,
    },
    /// Apply text replacements to one tracked text file.
    EditText {
        /// Repository logical path.
        path: String,
        /// Text replacements.
        edits: Vec<TextEdit>,
    },
    /// Replace or create one binary file.
    SetBytes {
        /// Repository logical path.
        path: String,
        /// Full binary content.
        bytes: Vec<u8>,
    },
    /// Remove one file.
    Remove {
        /// Repository logical path.
        path: String,
    },
    /// Move one file.
    Move {
        /// Source repository logical path.
        from: String,
        /// Destination repository logical path.
        to: String,
    },
}

#[wasm_bindgen]
impl SourceEdit {
    /// Create one payload variant.
    #[wasm_bindgen(js_name = "setText")]
    pub fn set_text(path: String, text: String) -> Self {
        Self {
            content: SourceEditContent::SetText {
                path,
                text,
            },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "editText")]
    pub fn edit_text(path: String, edits: Vec<TextEdit>) -> Self {
        Self {
            content: SourceEditContent::EditText {
                path,
                edits,
            },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "setBytes")]
    pub fn set_bytes(path: String, bytes: Vec<u8>) -> Self {
        Self {
            content: SourceEditContent::SetBytes {
                path,
                bytes,
            },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "remove")]
    pub fn remove(path: String) -> Self {
        Self {
            content: SourceEditContent::Remove { path },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "move")]
    pub fn move_file(from: String, to: String) -> Self {
        Self {
            content: SourceEditContent::Move {
                from,
                to,
            },
        }
    }
}

impl SourceEdit {
    /// Convert this WASM payload enum into one bridge enum.
    pub(crate) fn into_bridge(self) -> bridge::SourceEdit {
        match self.content {
            SourceEditContent::SetText { path, text } => {
                bridge::SourceEdit::SetText {
                    path,
                    text,
                }
            }
            SourceEditContent::EditText { path, edits } => {
                bridge::SourceEdit::EditText {
                    path,
                    edits: edits.into_iter().map(|item| item.into_bridge()).collect(),
                }
            }
            SourceEditContent::SetBytes { path, bytes } => {
                bridge::SourceEdit::SetBytes {
                    path,
                    bytes,
                }
            }
            SourceEditContent::Remove { path } => bridge::SourceEdit::Remove { path },
            SourceEditContent::Move { from, to } => {
                bridge::SourceEdit::Move {
                    from,
                    to,
                }
            }
        }
    }
}

/// Source update applied through one session ref.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct SourceUpdate {
    base: Option<Revision>,
    edits: Vec<SourceEdit>,
}

#[wasm_bindgen]
impl SourceUpdate {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(base: Option<Revision>, edits: Vec<SourceEdit>) -> Self {
        Self { base, edits }
    }

    /// Expected base revision.
    #[wasm_bindgen(getter, js_name = "base")]
    pub fn base(&self) -> Option<Revision> {
        self.base.clone()
    }

    /// Source edits in this atomic update.
    #[wasm_bindgen(getter, js_name = "edits")]
    pub fn edits(&self) -> Vec<SourceEdit> {
        self.edits.clone()
    }
}

impl SourceUpdate {
    /// Convert this WASM value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::SourceUpdate {
        bridge::SourceUpdate {
            base: self.base.map(|item| item.into_bridge()),
            edits: self.edits.into_iter().map(|item| item.into_bridge()).collect(),
        }
    }
}

/// Source update result.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct SourceUpdateResult {
    before: Revision,
    after: Revision,
    files: Vec<FileUpdate>,
}

#[wasm_bindgen]
impl SourceUpdateResult {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(before: Revision, after: Revision, files: Vec<FileUpdate>) -> Self {
        Self { before, after, files }
    }

    /// Previous revision.
    #[wasm_bindgen(getter, js_name = "before")]
    pub fn before(&self) -> Revision {
        self.before.clone()
    }

    /// Updated revision.
    #[wasm_bindgen(getter, js_name = "after")]
    pub fn after(&self) -> Revision {
        self.after.clone()
    }

    /// Changed files.
    #[wasm_bindgen(getter, js_name = "files")]
    pub fn files(&self) -> Vec<FileUpdate> {
        self.files.clone()
    }
}

impl SourceUpdateResult {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::SourceUpdateResult) -> Self {
        Self {
            before: Revision::from_bridge(value.before),
            after: Revision::from_bridge(value.after),
            files: value
                .files
                .into_iter()
                .map(|item| FileUpdate::from_bridge(item))
                .collect(),
        }
    }
}
