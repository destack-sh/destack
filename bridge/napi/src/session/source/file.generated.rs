// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::ModuleId;

/// One file change observed by a session.
#[derive(Debug)]
#[napi(object)]
pub struct Change {
    /// Repository logical path.
    pub path: String,
    /// External file URI.
    pub uri: String,
    /// Whether the file was removed.
    pub is_removed: bool,
    /// Updated module id when known.
    pub module_id: Option<ModuleId>,
}

impl Change {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::Change) -> Self {
        Self {
            path: value.path,
            uri: value.uri,
            is_removed: value.is_removed,
            module_id: value.module_id.map(|item| ModuleId::from_bridge(item)),
        }
    }
}
