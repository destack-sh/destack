// generated bridge source surface, do not edit

use std::path::PathBuf;

use napi_derive::napi;

/// One source file in an explicit source snapshot.
#[derive(Debug)]
#[napi(object)]
pub struct SourceFile {
    /// Repository-root relative path.
    pub path: String,
    /// Full text content.
    pub text: Option<String>,
    /// Full binary content.
    pub bytes: Option<Vec<u8>>,
}

/// Source truth used to open a live session.
#[derive(Debug)]
#[napi(object)]
pub struct SourceSnapshot {
    /// Files visible to the session source root.
    pub files: Vec<SourceFile>,
}

/// Immutable source revision.
#[derive(Debug)]
#[napi(object)]
pub struct Revision {
    /// Revision id.
    pub id: String,
}

/// Source text range in byte offsets.
#[derive(Debug)]
#[napi(object)]
pub struct TextRange {
    /// Inclusive start byte offset.
    pub start: u32,
    /// Exclusive end byte offset.
    pub end: u32,
}

/// Source text replacement.
#[derive(Debug)]
#[napi(object)]
pub struct TextEdit {
    /// Replaced byte range.
    pub range: TextRange,
    /// Replacement text.
    pub text: String,
}

/// Replace or create one text file.
#[derive(Debug)]
#[napi(object)]
pub struct SetTextEdit {
    /// Repository logical path.
    pub path: String,
    /// Full text content.
    pub text: String,
}

/// Apply text edits to one tracked text file.
#[derive(Debug)]
#[napi(object)]
pub struct EditTextEdit {
    /// Repository logical path.
    pub path: String,
    /// Ordered text edits.
    pub edits: Vec<TextEdit>,
}

/// Replace or create one binary file.
#[derive(Debug)]
#[napi(object)]
pub struct SetBytesEdit {
    /// Repository logical path.
    pub path: String,
    /// Full binary content.
    pub bytes: Vec<u8>,
}

/// Remove one file.
#[derive(Debug)]
#[napi(object)]
pub struct RemoveEdit {
    /// Repository logical path.
    pub path: String,
}

/// Move one file.
#[derive(Debug)]
#[napi(object)]
pub struct MoveEdit {
    /// Source repository logical path.
    pub from: String,
    /// Destination repository logical path.
    pub to: String,
}

/// Source edit accepted by a session update.
#[derive(Debug)]
#[napi(object)]
pub struct SourceEdit {
    /// Source edit kind.
    pub kind: String,
    /// Text replacement payload.
    pub set_text: Option<SetTextEdit>,
    /// Text edit payload.
    pub edit_text: Option<EditTextEdit>,
    /// Binary replacement payload.
    pub set_bytes: Option<SetBytesEdit>,
    /// Removal payload.
    pub remove: Option<RemoveEdit>,
    /// Move payload.
    pub move_file: Option<MoveEdit>,
}

/// Source update applied through one session ref.
#[derive(Debug)]
#[napi(object)]
pub struct SourceUpdate {
    /// Expected base revision.
    pub base: Option<Revision>,
    /// Source edits in this atomic update.
    pub edits: Vec<SourceEdit>,
}

/// File update projected from a source update.
#[derive(Debug)]
#[napi(object)]
pub struct FileUpdate {
    /// Repository logical path.
    pub path: String,
    /// Client-facing URI.
    pub uri: String,
    /// Coarse file update kind.
    pub kind: String,
    /// Whether the file was removed.
    pub is_removed: bool,
    /// Updated module id when known.
    pub module_id: Option<String>,
}

/// Source update result.
#[derive(Debug)]
#[napi(object)]
pub struct SourceUpdateResult {
    /// Previous revision.
    pub before: Revision,
    /// Updated revision.
    pub after: Revision,
    /// Changed files.
    pub files: Vec<FileUpdate>,
}

impl SourceSnapshot {
    /// Convert this NAPI source snapshot into one session source snapshot.
    pub(crate) fn into_core(self) -> napi::Result<destack_bridge_core::SourceSnapshot> {
        let files = self
            .files
            .into_iter()
            .map(SourceFile::into_core)
            .collect::<napi::Result<Vec<_>>>()?;

        Ok(destack_bridge_core::SourceSnapshot::new(files))
    }
}

impl SourceFile {
    /// Convert this NAPI source file into one session source file.
    fn into_core(self) -> napi::Result<destack_bridge_core::SourceFile> {
        match (self.text, self.bytes) {
            (Some(text), None) => Ok(destack_bridge_core::SourceFile::text(self.path, text)),
            (None, Some(bytes)) => Ok(destack_bridge_core::SourceFile::bytes(self.path, bytes)),
            _ => Err(napi::Error::from_reason(format!(
                "source file must provide exactly one content payload: {}",
                self.path
            ))),
        }
    }
}

impl Revision {
    /// Convert one workspace revision into one NAPI revision.
    pub(crate) fn from_core(revision: destack_bridge_core::Revision) -> Self {
        Self {
            id: revision.to_string(),
        }
    }

    /// Convert this NAPI revision into one workspace revision.
    fn into_core(self) -> napi::Result<destack_bridge_core::Revision> {
        destack_bridge_core::parse_revision(&self.id)
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }
}

impl TextRange {
    /// Convert this NAPI text range into one session text range.
    fn into_core(self) -> destack_bridge_core::TextRange {
        destack_bridge_core::TextRange {
            start: self.start,
            end: self.end,
        }
    }
}

impl TextEdit {
    /// Convert this NAPI text edit into one session text edit.
    fn into_core(self) -> destack_bridge_core::TextEdit {
        destack_bridge_core::TextEdit {
            range: self.range.into_core(),
            text: self.text,
        }
    }
}

impl SourceEdit {
    /// Convert this NAPI source edit into one session source edit.
    fn into_core(self) -> napi::Result<destack_bridge_core::SourceEdit> {
        match self.kind.as_str() {
            "setText" => self
                .set_text
                .map(|edit| destack_bridge_core::SourceEdit::SetText {
                    path: PathBuf::from(edit.path),
                    text: edit.text,
                })
                .ok_or_else(|| missing_payload("setText")),
            "editText" => self
                .edit_text
                .map(|edit| destack_bridge_core::SourceEdit::EditText {
                    path: PathBuf::from(edit.path),
                    edits: edit.edits.into_iter().map(TextEdit::into_core).collect(),
                })
                .ok_or_else(|| missing_payload("editText")),
            "setBytes" => self
                .set_bytes
                .map(|edit| destack_bridge_core::SourceEdit::SetBytes {
                    path: PathBuf::from(edit.path),
                    bytes: edit.bytes,
                })
                .ok_or_else(|| missing_payload("setBytes")),
            "remove" => self
                .remove
                .map(|edit| destack_bridge_core::SourceEdit::Remove {
                    path: PathBuf::from(edit.path),
                })
                .ok_or_else(|| missing_payload("remove")),
            "move" => self
                .move_file
                .map(|edit| destack_bridge_core::SourceEdit::Move {
                    from: PathBuf::from(edit.from),
                    to: PathBuf::from(edit.to),
                })
                .ok_or_else(|| missing_payload("move")),
            _ => Err(napi::Error::from_reason(format!(
                "unknown source edit kind: {}",
                self.kind
            ))),
        }
    }
}

impl SourceUpdate {
    /// Convert this NAPI source update into one session source update.
    pub(crate) fn into_core(self) -> napi::Result<destack_bridge_core::SourceUpdate> {
        let base = self.base.map(Revision::into_core).transpose()?;
        let edits = self
            .edits
            .into_iter()
            .map(SourceEdit::into_core)
            .collect::<napi::Result<Vec<_>>>()?;

        Ok(destack_bridge_core::SourceUpdate { base, edits })
    }
}

impl FileUpdate {
    /// Convert one session file update into one NAPI file update.
    pub(crate) fn from_core(
        session: &destack_bridge_core::LanguageSession,
        update: destack_bridge_core::FileUpdate,
    ) -> Self {
        let uri = update.uri().to_string();
        let path = session.file_update_path(&update);
        let kind = file_update_kind_label(update.kind()).to_string();
        let module_id = update.module_id().map(|module_id| format!("{module_id:?}"));
        let is_removed = update.is_removed();

        Self {
            path,
            uri,
            kind,
            is_removed,
            module_id,
        }
    }
}

impl SourceUpdateResult {
    /// Convert one session source update result into one NAPI source update result.
    pub(crate) fn from_core(
        session: &destack_bridge_core::LanguageSession,
        result: destack_bridge_core::SourceUpdateResult,
    ) -> Self {
        Self {
            before: Revision::from_core(result.before),
            after: Revision::from_core(result.after),
            files: result
                .files
                .into_iter()
                .map(|update| FileUpdate::from_core(session, update))
                .collect(),
        }
    }
}

/// Return one missing edit payload error.
fn missing_payload(kind: &str) -> napi::Error {
    napi::Error::from_reason(format!("{kind} edit is missing payload"))
}

/// Return the NAPI label for one file update kind.
fn file_update_kind_label(kind: destack_bridge_core::FileUpdateKind) -> &'static str {
    match kind {
        destack_bridge_core::FileUpdateKind::Source => "source",
        destack_bridge_core::FileUpdateKind::Config => "config",
    }
}
