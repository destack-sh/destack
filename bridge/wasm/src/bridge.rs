use wasm_bindgen::prelude::wasm_bindgen;

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
        destack_bridge_core::version().to_string()
    }
}

/// Return the crate version.
#[wasm_bindgen]
pub fn version() -> String {
    destack_bridge_core::version().to_string()
}
