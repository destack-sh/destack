// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::{Diagnostic, DirChecked};

/// One checker output.
#[derive(Debug)]
#[napi(object, js_name = "CheckOutput")]
pub struct CheckOutput {
    /// Checked DIR artifact projection.
    pub checked: DirChecked,
    /// Diagnostics emitted by checking.
    pub diagnostics: Vec<Diagnostic>,
}

impl CheckOutput {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::CheckOutput) -> Self {
        Self {
            checked: DirChecked::from_bridge(value.checked),
            diagnostics: value
                .diagnostics
                .into_iter()
                .map(Diagnostic::from_bridge)
                .collect(),
        }
    }
}
