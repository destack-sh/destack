// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

/// One module loaded through a live session.
#[derive(Debug)]
#[napi(object)]
pub struct Module {
    /// Displayed compiler module id.
    pub module_id: String,
}

impl Module {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::Module) -> Self {
        Self { module_id: value.module_id }
    }
}
