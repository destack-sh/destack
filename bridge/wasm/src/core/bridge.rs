use wasm_bindgen::prelude::{JsValue, wasm_bindgen};

/// Return the crate version.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Convert one bridge error into a JS error.
pub(crate) fn js_error(error: impl ToString) -> JsValue {
    JsValue::from_str(&error.to_string())
}
