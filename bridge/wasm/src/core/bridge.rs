use wasm_bindgen::prelude::{JsValue, wasm_bindgen};

/// The static backend marker for WebAssembly clients.
pub const BACKEND: &str = "wasm";

/// Shared bridge metadata exposed to WebAssembly clients.
#[derive(Debug)]
#[wasm_bindgen]
pub struct Bridge;

#[wasm_bindgen]
impl Bridge {
    /// Create a bridge metadata handle.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self
    }

    /// Return the backend marker.
    #[wasm_bindgen]
    pub fn backend(&self) -> String {
        BACKEND.to_string()
    }

    /// Return the crate version.
    #[wasm_bindgen]
    pub fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}

/// Return the crate version.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Convert one bridge error into a JS error.
pub(crate) fn js_error(error: impl ToString) -> JsValue {
    JsValue::from_str(&error.to_string())
}
