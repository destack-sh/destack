// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::ModuleId;

/// Observed file change projected from a file update.
#[derive(Debug)]
#[napi(object)]
pub struct FileChange {
    /// Repository logical path.
    pub path: String,
    /// External file URI.
    pub uri: String,
    /// Coarse file change kind.
    pub kind: String,
    /// Whether the file was removed.
    pub is_removed: bool,
    /// Updated module id when known.
    pub module_id: Option<ModuleId>,
}

impl FileChange {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::FileChange) -> Self {
        Self {
            path: value.path,
            uri: value.uri,
            kind: file_change_kind_label(value.kind),
            is_removed: value.is_removed,
            module_id: value.module_id.map(|item| ModuleId::from_bridge(item)),
        }
    }
}

/// Return one target enum label.
fn file_change_kind_label(value: bridge::FileChangeKind) -> String {
    let label = match value {
        bridge::FileChangeKind::Source => "source",
        bridge::FileChangeKind::Config => "config",
    };
    label.to_string()
}
