// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

/// One editable file visible to a live session.
#[derive(Debug)]
#[napi(object)]
pub struct SessionFile {
    /// Repository logical path.
    pub path: String,
}

impl SessionFile {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::SessionFile) -> Self {
        Self { path: value.path }
    }
}
