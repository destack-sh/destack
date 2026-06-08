// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::ModuleId;

/// One source file in an explicit source snapshot.
#[derive(Debug)]
#[napi(object)]
pub struct SourceFile {
    /// Repository-root relative path.
    pub path: String,
    /// Full source file content.
    pub content: SourceFileContent,
}

impl SourceFile {
    /// Convert this NAPI value into one bridge value.
    pub(crate) fn into_bridge(self) -> napi::Result<bridge::SourceFile> {
        Ok(bridge::SourceFile {
            path: self.path,
            content: self.content.into_bridge()?,
        })
    }
}

/// Full content for one source snapshot file.
#[derive(Debug)]
#[napi(object)]
pub struct SourceFileContent {
    /// Payload variant label.
    pub kind: String,
    /// Text file content.
    pub text: Option<String>,
    /// Binary file content.
    pub bytes: Option<Vec<u8>>,
}

impl SourceFileContent {
    /// Convert this NAPI payload enum into one bridge enum.
    pub(crate) fn into_bridge(self) -> napi::Result<bridge::SourceFileContent> {
        match self.kind.as_str() {
            "text" => {
                if self.bytes.is_some() {
                    return Err(unexpected_payload("bytes"));
                }
                let Some(value) = self.text else {
                    return Err(missing_payload("text"));
                };
                Ok(bridge::SourceFileContent::Text(value))
            }
            "bytes" => {
                if self.text.is_some() {
                    return Err(unexpected_payload("text"));
                }
                let Some(value) = self.bytes else {
                    return Err(missing_payload("bytes"));
                };
                Ok(bridge::SourceFileContent::Bytes(value))
            }
            _ => Err(napi::Error::from_reason(format!(
                "unknown {}: {}",
                stringify!(SourceFileContent),
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

/// File update projected from a source update.
#[derive(Debug)]
#[napi(object)]
pub struct FileUpdate {
    /// Repository logical path.
    pub path: String,
    /// External file URI.
    pub uri: String,
    /// Coarse file update kind.
    pub kind: String,
    /// Whether the file was removed.
    pub is_removed: bool,
    /// Updated module id when known.
    pub module_id: Option<ModuleId>,
}

impl FileUpdate {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::FileUpdate) -> Self {
        Self {
            path: value.path,
            uri: value.uri,
            kind: file_update_kind_label(value.kind),
            is_removed: value.is_removed,
            module_id: value.module_id.map(|item| ModuleId::from_bridge(item)),
        }
    }
}

/// Return one target enum label.
fn file_update_kind_label(value: bridge::FileUpdateKind) -> String {
    let label = match value {
        bridge::FileUpdateKind::Source => "source",
        bridge::FileUpdateKind::Config => "config",
    };
    label.to_string()
}
