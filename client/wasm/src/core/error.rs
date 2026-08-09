#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use destack_artifact::BuildId;

use wasm_bindgen::prelude::{JsValue, wasm_bindgen};

/// Initialize the build identity from the exact WebAssembly module bytes.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[wasm_bindgen(js_name = initializeBuild)]
pub fn initialize_build(bytes: &[u8]) -> Result<(), JsValue> {
    BuildId::initialize_current(bytes).map_err(js_error)?;

    Ok(())
}

/// Return the crate version.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Convert one client error into a JS error.
pub(crate) fn js_error(error: impl ToString) -> JsValue {
    JsValue::from_str(&error.to_string())
}
