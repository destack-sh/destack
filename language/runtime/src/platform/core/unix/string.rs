use std::ffi::{CStr, CString, c_char};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;

/// Convert one utf-8 string into one owned C string.
pub(crate) fn c_string_from_str(value: &str, field: &'static str) -> RuntimeResult<CString> {
    CString::new(value).map_err(|_| {
        core_platform::invalid_argument(field, "value must not contain interior NUL bytes")
    })
}

/// Decode one nullable C string pointer into one owned Rust string.
pub(crate) fn string_from_c_str(pointer: *const c_char) -> Option<String> {
    if pointer.is_null() {
        return None;
    }

    let value = unsafe { CStr::from_ptr(pointer) };
    let value = value.to_str().ok()?;

    Some(value.to_string())
}
