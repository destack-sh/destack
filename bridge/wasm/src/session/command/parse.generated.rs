// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::{Diagnostic, DirParsed};

/// One parser output.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct ParseOutput {
    parsed: DirParsed,
    diagnostics: Vec<Diagnostic>,
}

#[wasm_bindgen]
impl ParseOutput {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(parsed: DirParsed, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            parsed,
            diagnostics,
        }
    }

    /// Parsed DIR artifact projection.
    #[wasm_bindgen(getter, js_name = "parsed")]
    pub fn parsed(&self) -> DirParsed {
        self.parsed.clone()
    }

    /// Diagnostics emitted by parsing.
    #[wasm_bindgen(getter, js_name = "diagnostics")]
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.diagnostics.clone()
    }
}

impl ParseOutput {
    /// Convert one bridge value into one WASM value.
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
