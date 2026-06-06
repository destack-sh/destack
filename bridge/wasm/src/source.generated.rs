// generated bridge source surface, do not edit

use std::path::PathBuf;

use js_sys::Array;
use wasm_bindgen::prelude::{JsValue, wasm_bindgen};

/// One source file in an explicit source snapshot.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct SourceFile {
    path: String,
    content: SourceFileContent,
}

/// Full content for one source snapshot file.
#[derive(Debug, Clone)]
enum SourceFileContent {
    /// Text file content.
    Text(String),
    /// Binary file content.
    Bytes(Vec<u8>),
}

#[wasm_bindgen]
impl SourceFile {
    /// Create one text source file.
    #[wasm_bindgen(js_name = text)]
    pub fn text(path: String, text: String) -> Self {
        Self {
            path,
            content: SourceFileContent::Text(text),
        }
    }

    /// Create one binary source file.
    #[wasm_bindgen(js_name = bytes)]
    pub fn bytes(path: String, bytes: Vec<u8>) -> Self {
        Self {
            path,
            content: SourceFileContent::Bytes(bytes),
        }
    }

    /// Return the repository-root relative path.
    #[wasm_bindgen(getter)]
    pub fn path(&self) -> String {
        self.path.clone()
    }
}

impl SourceFile {
    /// Convert this WASM source file into one session source file.
    fn into_core(self) -> destack_bridge_core::SourceFile {
        match self.content {
            SourceFileContent::Text(text) => destack_bridge_core::SourceFile::text(self.path, text),
            SourceFileContent::Bytes(bytes) => {
                destack_bridge_core::SourceFile::bytes(self.path, bytes)
            }
        }
    }
}

/// Source truth used to open a live session.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct SourceSnapshot {
    files: Vec<SourceFile>,
}

#[wasm_bindgen]
impl SourceSnapshot {
    /// Create one source snapshot.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    /// Add one source file.
    #[wasm_bindgen(js_name = addFile)]
    pub fn add_file(&mut self, file: SourceFile) {
        self.files.push(file);
    }

    /// Return the source file count.
    #[wasm_bindgen(getter)]
    pub fn len(&self) -> usize {
        self.files.len()
    }
}

impl SourceSnapshot {
    /// Convert this WASM source snapshot into one session source snapshot.
    pub(crate) fn into_core(self) -> destack_bridge_core::SourceSnapshot {
        let files = self.files.into_iter().map(SourceFile::into_core).collect();

        destack_bridge_core::SourceSnapshot::new(files)
    }
}

/// Immutable source revision.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct Revision {
    id: String,
}

#[wasm_bindgen]
impl Revision {
    /// Create one source revision.
    #[wasm_bindgen(constructor)]
    pub fn new(id: String) -> Self {
        Self { id }
    }

    /// Return the revision id.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.id.clone()
    }
}

impl Revision {
    /// Convert one workspace revision into one WASM revision.
    pub(crate) fn from_core(revision: destack_bridge_core::Revision) -> Self {
        Self {
            id: revision.to_string(),
        }
    }

    /// Convert this WASM revision into one workspace revision.
    fn into_core(self) -> Result<destack_bridge_core::Revision, JsValue> {
        destack_bridge_core::parse_revision(&self.id).map_err(js_error)
    }
}

/// Source text range in byte offsets.
#[derive(Debug, Clone, Copy)]
#[wasm_bindgen]
pub struct TextRange {
    start: u32,
    end: u32,
}

#[wasm_bindgen]
impl TextRange {
    /// Create one source text range.
    #[wasm_bindgen(constructor)]
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    /// Return the inclusive start byte offset.
    #[wasm_bindgen(getter)]
    pub fn start(&self) -> u32 {
        self.start
    }

    /// Return the exclusive end byte offset.
    #[wasm_bindgen(getter)]
    pub fn end(&self) -> u32 {
        self.end
    }
}

impl TextRange {
    /// Convert this WASM range into one session range.
    fn into_core(self) -> destack_bridge_core::TextRange {
        destack_bridge_core::TextRange {
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
    /// Create one source text replacement.
    #[wasm_bindgen(constructor)]
    pub fn new(range: TextRange, text: String) -> Self {
        Self { range, text }
    }

    /// Return the replaced byte range.
    #[wasm_bindgen(getter)]
    pub fn range(&self) -> TextRange {
        self.range
    }

    /// Return the replacement text.
    #[wasm_bindgen(getter)]
    pub fn text(&self) -> String {
        self.text.clone()
    }
}

impl TextEdit {
    /// Convert this WASM text edit into one session text edit.
    fn into_core(self) -> destack_bridge_core::TextEdit {
        destack_bridge_core::TextEdit {
            range: self.range.into_core(),
            text: self.text,
        }
    }
}

/// Source edit accepted by a session update.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct SourceEdit {
    edit: SourceEditContent,
}

/// Concrete source edit content.
#[derive(Debug, Clone)]
enum SourceEditContent {
    /// Replace or create one text file.
    SetText { path: String, text: String },
    /// Apply text replacements to one tracked text file.
    EditText { path: String, edits: Vec<TextEdit> },
    /// Replace or create one binary file.
    SetBytes { path: String, bytes: Vec<u8> },
    /// Remove one file.
    Remove { path: String },
    /// Move one file.
    Move { from: String, to: String },
}

#[wasm_bindgen]
impl SourceEdit {
    /// Create one text replacement edit.
    #[wasm_bindgen(js_name = setText)]
    pub fn set_text(path: String, text: String) -> Self {
        Self {
            edit: SourceEditContent::SetText { path, text },
        }
    }

    /// Create one text edit edit.
    #[wasm_bindgen(js_name = editText)]
    pub fn edit_text(path: String) -> Self {
        Self {
            edit: SourceEditContent::EditText {
                path,
                edits: Vec::new(),
            },
        }
    }

    /// Add one text edit to an editText source edit.
    #[wasm_bindgen(js_name = addTextEdit)]
    pub fn add_text_edit(&mut self, edit: TextEdit) -> Result<(), JsValue> {
        let SourceEditContent::EditText { edits, .. } = &mut self.edit else {
            return Err(JsValue::from_str(
                "text edits can only be added to editText",
            ));
        };
        edits.push(edit);

        Ok(())
    }

    /// Create one binary replacement edit.
    #[wasm_bindgen(js_name = setBytes)]
    pub fn set_bytes(path: String, bytes: Vec<u8>) -> Self {
        Self {
            edit: SourceEditContent::SetBytes { path, bytes },
        }
    }

    /// Create one removal edit.
    #[wasm_bindgen(js_name = remove)]
    pub fn remove(path: String) -> Self {
        Self {
            edit: SourceEditContent::Remove { path },
        }
    }

    /// Create one move edit.
    #[wasm_bindgen(js_name = moveFile)]
    pub fn move_file(from: String, to: String) -> Self {
        Self {
            edit: SourceEditContent::Move { from, to },
        }
    }
}

impl SourceEdit {
    /// Convert this WASM source edit into one session source edit.
    fn into_core(self) -> Result<destack_bridge_core::SourceEdit, JsValue> {
        match self.edit {
            SourceEditContent::SetText { path, text } => {
                Ok(destack_bridge_core::SourceEdit::SetText {
                    path: PathBuf::from(path),
                    text,
                })
            }
            SourceEditContent::EditText { path, edits } => {
                let edits = edits.into_iter().map(TextEdit::into_core).collect();

                Ok(destack_bridge_core::SourceEdit::EditText {
                    path: PathBuf::from(path),
                    edits,
                })
            }
            SourceEditContent::SetBytes { path, bytes } => {
                Ok(destack_bridge_core::SourceEdit::SetBytes {
                    path: PathBuf::from(path),
                    bytes,
                })
            }
            SourceEditContent::Remove { path } => Ok(destack_bridge_core::SourceEdit::Remove {
                path: PathBuf::from(path),
            }),
            SourceEditContent::Move { from, to } => Ok(destack_bridge_core::SourceEdit::Move {
                from: PathBuf::from(from),
                to: PathBuf::from(to),
            }),
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
    /// Create one source update.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            base: None,
            edits: Vec::new(),
        }
    }

    /// Set the expected base revision.
    #[wasm_bindgen(js_name = setBase)]
    pub fn set_base(&mut self, base: Revision) {
        self.base = Some(base);
    }

    /// Add one source edit.
    #[wasm_bindgen(js_name = addEdit)]
    pub fn add_edit(&mut self, edit: SourceEdit) {
        self.edits.push(edit);
    }
}

impl SourceUpdate {
    /// Convert this WASM source update into one session source update.
    pub(crate) fn into_core(self) -> Result<destack_bridge_core::SourceUpdate, JsValue> {
        let base = self.base.map(Revision::into_core).transpose()?;
        let edits = self
            .edits
            .into_iter()
            .map(SourceEdit::into_core)
            .collect::<Result<Vec<_>, JsValue>>()?;

        Ok(destack_bridge_core::SourceUpdate { base, edits })
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
    module_id: Option<String>,
}

#[wasm_bindgen]
impl FileUpdate {
    /// Return the repository logical path.
    #[wasm_bindgen(getter)]
    pub fn path(&self) -> String {
        self.path.clone()
    }

    /// Return the client-facing URI.
    #[wasm_bindgen(getter)]
    pub fn uri(&self) -> String {
        self.uri.clone()
    }

    /// Return the coarse file update kind.
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> String {
        self.kind.clone()
    }

    /// Return whether the file was removed.
    #[wasm_bindgen(getter, js_name = isRemoved)]
    pub fn is_removed(&self) -> bool {
        self.is_removed
    }

    /// Return the updated module id when known.
    #[wasm_bindgen(getter, js_name = moduleId)]
    pub fn module_id(&self) -> Option<String> {
        self.module_id.clone()
    }
}

impl FileUpdate {
    /// Convert one session file update into one WASM file update.
    pub(crate) fn from_core(
        session: &destack_bridge_core::LanguageSession,
        update: destack_bridge_core::FileUpdate,
    ) -> Self {
        let uri = update.uri().to_string();
        let path = session.file_update_path(&update);
        let kind = file_update_kind_label(update.kind()).to_string();
        let is_removed = update.is_removed();
        let module_id = update.module_id().map(|module_id| format!("{module_id:?}"));

        Self {
            path,
            uri,
            kind,
            is_removed,
            module_id,
        }
    }
}

/// Source update result.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct SourceUpdateResult {
    before: Revision,
    after: Revision,
    files: Array,
}

#[wasm_bindgen]
impl SourceUpdateResult {
    /// Return the previous revision.
    #[wasm_bindgen(getter)]
    pub fn before(&self) -> Revision {
        self.before.clone()
    }

    /// Return the updated revision.
    #[wasm_bindgen(getter)]
    pub fn after(&self) -> Revision {
        self.after.clone()
    }

    /// Return changed files.
    #[wasm_bindgen(getter)]
    pub fn files(&self) -> Array {
        self.files.clone()
    }
}

impl SourceUpdateResult {
    /// Convert one session source update result into one WASM source update result.
    pub(crate) fn from_core(
        session: &destack_bridge_core::LanguageSession,
        result: destack_bridge_core::SourceUpdateResult,
    ) -> Self {
        let files = Array::new();
        for file in result.files {
            files.push(&JsValue::from(FileUpdate::from_core(session, file)));
        }

        Self {
            before: Revision::from_core(result.before),
            after: Revision::from_core(result.after),
            files,
        }
    }
}

/// Convert one bridge error into a JS error.
pub(crate) fn js_error(error: impl ToString) -> JsValue {
    JsValue::from_str(&error.to_string())
}

/// Return the WASM label for one file update kind.
fn file_update_kind_label(kind: destack_bridge_core::FileUpdateKind) -> &'static str {
    match kind {
        destack_bridge_core::FileUpdateKind::Source => "source",
        destack_bridge_core::FileUpdateKind::Config => "config",
    }
}
