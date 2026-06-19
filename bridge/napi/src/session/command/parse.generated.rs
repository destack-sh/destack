// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::{Diagnostic, DirParsed};

/// One parser output.
#[derive(Debug)]
#[napi(object, js_name = "ParseOutput")]
pub struct ParseOutput {
    /// Parsed DIR artifact projection.
    pub parsed: DirParsed,
    /// Diagnostics emitted by parsing.
    pub diagnostics: Vec<Diagnostic>,
}

impl ParseOutput {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::ParseOutput) -> Self {
        Self {
            parsed: DirParsed::from_bridge(value.parsed),
            diagnostics: value
                .diagnostics
                .into_iter()
                .map(Diagnostic::from_bridge)
                .collect(),
        }
    }
}
