// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::{FileChange, Revision};

/// One text range in byte offsets.
#[derive(Debug)]
#[napi(object)]
pub struct TextRange {
    /// Inclusive start byte offset.
    pub start: u32,
    /// Exclusive end byte offset.
    pub end: u32,
}

impl TextRange {
    /// Convert this NAPI value into one bridge value.
    pub(crate) fn into_bridge(self) -> napi::Result<bridge::TextRange> {
        Ok(bridge::TextRange {
            start: self.start,
            end: self.end,
        })
    }
}

/// One text replacement.
#[derive(Debug)]
#[napi(object)]
pub struct TextEdit {
    /// Replaced byte range.
    pub range: TextRange,
    /// Replacement text.
    pub text: String,
}

impl TextEdit {
    /// Convert this NAPI value into one bridge value.
    pub(crate) fn into_bridge(self) -> napi::Result<bridge::TextEdit> {
        Ok(bridge::TextEdit {
            range: self.range.into_bridge()?,
            text: self.text,
        })
    }
}

/// One file edit accepted by a session update.
#[derive(Debug)]
#[napi(object)]
pub struct FileEdit {
    /// Payload variant label.
    pub kind: String,
    /// Repository logical path.
    pub path: Option<String>,
    /// Full text content.
    pub text: Option<String>,
    /// Text replacements.
    pub edits: Option<Vec<TextEdit>>,
    /// Full binary content.
    pub bytes: Option<Vec<u8>>,
    /// Repository logical path.
    pub from: Option<String>,
    /// Destination repository logical path.
    pub to: Option<String>,
}

impl FileEdit {
    /// Convert this NAPI payload enum into one bridge enum.
    pub(crate) fn into_bridge(self) -> napi::Result<bridge::FileEdit> {
        match self.kind.as_str() {
            "setText" => {
                if self.edits.is_some() {
                    return Err(unexpected_payload("edits"));
                }
                if self.bytes.is_some() {
                    return Err(unexpected_payload("bytes"));
                }
                if self.from.is_some() {
                    return Err(unexpected_payload("from"));
                }
                if self.to.is_some() {
                    return Err(unexpected_payload("to"));
                }
                let Some(value) = self.path else {
                    return Err(missing_payload("path"));
                };
                let path = value;
                let Some(value) = self.text else {
                    return Err(missing_payload("text"));
                };
                let text = value;
                Ok(bridge::FileEdit::SetText { path, text })
            }
            "editText" => {
                if self.text.is_some() {
                    return Err(unexpected_payload("text"));
                }
                if self.bytes.is_some() {
                    return Err(unexpected_payload("bytes"));
                }
                if self.from.is_some() {
                    return Err(unexpected_payload("from"));
                }
                if self.to.is_some() {
                    return Err(unexpected_payload("to"));
                }
                let Some(value) = self.path else {
                    return Err(missing_payload("path"));
                };
                let path = value;
                let Some(value) = self.edits else {
                    return Err(missing_payload("edits"));
                };
                let edits = value
                    .into_iter()
                    .map(|item| Ok::<_, napi::Error>(item.into_bridge()?))
                    .collect::<napi::Result<Vec<_>>>()?;
                Ok(bridge::FileEdit::EditText { path, edits })
            }
            "setBytes" => {
                if self.text.is_some() {
                    return Err(unexpected_payload("text"));
                }
                if self.edits.is_some() {
                    return Err(unexpected_payload("edits"));
                }
                if self.from.is_some() {
                    return Err(unexpected_payload("from"));
                }
                if self.to.is_some() {
                    return Err(unexpected_payload("to"));
                }
                let Some(value) = self.path else {
                    return Err(missing_payload("path"));
                };
                let path = value;
                let Some(value) = self.bytes else {
                    return Err(missing_payload("bytes"));
                };
                let bytes = value;
                Ok(bridge::FileEdit::SetBytes { path, bytes })
            }
            "remove" => {
                if self.text.is_some() {
                    return Err(unexpected_payload("text"));
                }
                if self.edits.is_some() {
                    return Err(unexpected_payload("edits"));
                }
                if self.bytes.is_some() {
                    return Err(unexpected_payload("bytes"));
                }
                if self.from.is_some() {
                    return Err(unexpected_payload("from"));
                }
                if self.to.is_some() {
                    return Err(unexpected_payload("to"));
                }
                let Some(value) = self.path else {
                    return Err(missing_payload("path"));
                };
                let path = value;
                Ok(bridge::FileEdit::Remove { path })
            }
            "move" => {
                if self.path.is_some() {
                    return Err(unexpected_payload("path"));
                }
                if self.text.is_some() {
                    return Err(unexpected_payload("text"));
                }
                if self.edits.is_some() {
                    return Err(unexpected_payload("edits"));
                }
                if self.bytes.is_some() {
                    return Err(unexpected_payload("bytes"));
                }
                let Some(value) = self.from else {
                    return Err(missing_payload("from"));
                };
                let from = value;
                let Some(value) = self.to else {
                    return Err(missing_payload("to"));
                };
                let to = value;
                Ok(bridge::FileEdit::Move { from, to })
            }
            _ => Err(napi::Error::from_reason(format!(
                "unknown {}: {}",
                stringify!(FileEdit),
                self.kind
            ))),
        }
    }
}

/// Return one missing payload error.
fn missing_payload(kind: &str) -> napi::Error {
    napi::Error::from_reason(format!("{kind} payload is missing"))
}

/// Return one unexpected payload error.
fn unexpected_payload(kind: &str) -> napi::Error {
    napi::Error::from_reason(format!("{kind} payload is unexpected"))
}

/// One file update applied through one session ref.
#[derive(Debug)]
#[napi(object)]
pub struct FileUpdate {
    /// Expected base revision.
    pub base: Option<Revision>,
    /// File edits in this atomic update.
    pub edits: Vec<FileEdit>,
}

impl FileUpdate {
    /// Convert this NAPI value into one bridge value.
    pub(crate) fn into_bridge(self) -> napi::Result<bridge::FileUpdate> {
        Ok(bridge::FileUpdate {
            base: self
                .base
                .map(|item| Ok::<_, napi::Error>(item.into_bridge()?))
                .transpose()?,
            edits: self
                .edits
                .into_iter()
                .map(|item| Ok::<_, napi::Error>(item.into_bridge()?))
                .collect::<napi::Result<Vec<_>>>()?,
        })
    }
}

/// File update result.
#[derive(Debug)]
#[napi(object)]
pub struct FileUpdateResult {
    /// Previous revision.
    pub before: Revision,
    /// Updated revision.
    pub after: Revision,
    /// Changed files.
    pub files: Vec<FileChange>,
}

impl FileUpdateResult {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::FileUpdateResult) -> Self {
        Self {
            before: Revision::from_bridge(value.before),
            after: Revision::from_bridge(value.after),
            files: value
                .files
                .into_iter()
                .map(|item| FileChange::from_bridge(item))
                .collect(),
        }
    }
}
