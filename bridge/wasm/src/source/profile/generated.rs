// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

/// External profile id crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct ProfileId {
    id: String,
}

#[wasm_bindgen]
impl ProfileId {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(id: String) -> Self {
        Self { id }
    }

    /// Canonical lowercase hex profile id.
    #[wasm_bindgen(getter, js_name = "id")]
    pub fn id(&self) -> String {
        self.id.clone()
    }
}

impl ProfileId {
    /// Convert this WASM value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::ProfileId {
        bridge::ProfileId { id: self.id }
    }
}

impl ProfileId {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::ProfileId) -> Self {
        Self { id: value.id }
    }
}
