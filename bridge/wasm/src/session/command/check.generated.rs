// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::{Diagnostic, DirChecked};

/// One checker output.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct CheckOutput {
    checked: DirChecked,
    diagnostics: Vec<Diagnostic>,
}

#[wasm_bindgen]
impl CheckOutput {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(checked: DirChecked, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            checked,
            diagnostics,
        }
    }

    /// Checked DIR artifact projection.
    #[wasm_bindgen(getter, js_name = "checked")]
    pub fn checked(&self) -> DirChecked {
        self.checked.clone()
    }

    /// Diagnostics emitted by checking.
    #[wasm_bindgen(getter, js_name = "diagnostics")]
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.diagnostics.clone()
    }
}

impl CheckOutput {
    /// Convert one bridge value into one WASM value.
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
