use wasm_bindgen::JsValue;

/// Convert an error into a JavaScript error value.
pub(super) fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

/// Deserialize optional JSON-like JS input with a default fallback.
pub(super) fn parse_optional_input<T>(input: Option<JsValue>) -> Result<T, JsValue>
where
    T: Default + serde::de::DeserializeOwned,
{
    let Some(input) = input else {
        return Ok(T::default());
    };

    if input.is_null() || input.is_undefined() {
        return Ok(T::default());
    }

    serde_wasm_bindgen::from_value(input).map_err(js_error)
}

/// Serialize Rust output as JSON-like JS values.
pub(super) fn to_js_value<T: serde::Serialize>(value: &T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(value).map_err(js_error)
}
