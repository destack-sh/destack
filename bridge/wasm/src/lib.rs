use wasm_bindgen::prelude::wasm_bindgen;

/// Return the crate version.
#[wasm_bindgen]
pub fn version() -> String {
    destack_bridge_core::version().to_string()
}
