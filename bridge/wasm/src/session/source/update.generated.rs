// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::{Change, Revision};

/// One text range in byte offsets.
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

/// One text replacement.
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

/// One edit accepted by a session.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct Edit {
    content: EditContent,
}

/// Concrete payload enum content.
#[derive(Debug, Clone)]
enum EditContent {
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
        /// Repository logical path.
        from: String,
        /// Destination repository logical path.
        to: String,
    },
}

#[wasm_bindgen]
impl Edit {
    /// Create one payload variant.
    #[wasm_bindgen(js_name = "setText")]
    pub fn set_text(path: String, text: String) -> Self {
        Self {
            content: EditContent::SetText { path, text },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "editText")]
    pub fn edit_text(path: String, edits: Vec<TextEdit>) -> Self {
        Self {
            content: EditContent::EditText { path, edits },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "setBytes")]
    pub fn set_bytes(path: String, bytes: Vec<u8>) -> Self {
        Self {
            content: EditContent::SetBytes { path, bytes },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "remove")]
    pub fn remove(path: String) -> Self {
        Self {
            content: EditContent::Remove { path },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "move")]
    pub fn move_file(from: String, to: String) -> Self {
        Self {
            content: EditContent::Move { from, to },
        }
    }
}

impl Edit {
    /// Convert this WASM payload enum into one bridge enum.
    pub(crate) fn into_bridge(self) -> bridge::Edit {
        match self.content {
            EditContent::SetText { path, text } => bridge::Edit::SetText { path, text },
            EditContent::EditText { path, edits } => bridge::Edit::EditText {
                path,
                edits: edits.into_iter().map(|item| item.into_bridge()).collect(),
            },
            EditContent::SetBytes { path, bytes } => bridge::Edit::SetBytes { path, bytes },
            EditContent::Remove { path } => bridge::Edit::Remove { path },
            EditContent::Move { from, to } => bridge::Edit::Move { from, to },
        }
    }
}

/// One committed edit batch.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct Commit {
    before: Revision,
    after: Revision,
    changes: Vec<Change>,
}

#[wasm_bindgen]
impl Commit {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(before: Revision, after: Revision, changes: Vec<Change>) -> Self {
        Self {
            before,
            after,
            changes,
        }
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
    #[wasm_bindgen(getter, js_name = "changes")]
    pub fn changes(&self) -> Vec<Change> {
        self.changes.clone()
    }
}

impl Commit {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::Commit) -> Self {
        Self {
            before: Revision::from_bridge(value.before),
            after: Revision::from_bridge(value.after),
            changes: value
                .changes
                .into_iter()
                .map(|item| Change::from_bridge(item))
                .collect(),
        }
    }
}
