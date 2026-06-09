// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::ModuleId;

/// One module loaded through a live session.
#[derive(Debug)]
#[napi(object)]
pub struct Module {
    /// Stable source module id.
    pub id: ModuleId,
}

impl Module {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::Module) -> Self {
        Self {
            id: ModuleId::from_bridge(value.id),
        }
    }
}
