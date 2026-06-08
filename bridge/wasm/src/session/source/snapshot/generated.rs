// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::SourceFile;

/// Source truth used to open a live session.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct SourceSnapshot {
    files: Vec<SourceFile>,
}

#[wasm_bindgen]
impl SourceSnapshot {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(files: Vec<SourceFile>) -> Self {
        Self { files }
    }

    /// Files visible to the session source root.
    #[wasm_bindgen(getter, js_name = "files")]
    pub fn files(&self) -> Vec<SourceFile> {
        self.files.clone()
    }
}

impl SourceSnapshot {
    /// Convert this WASM value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::SourceSnapshot {
        bridge::SourceSnapshot {
            files: self
                .files
                .into_iter()
                .map(|item| item.into_bridge())
                .collect(),
        }
    }
}
