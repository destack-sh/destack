use std::ffi::{CStr, CString, c_char};
use std::{ptr, slice};

/// Destroy one owned C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_string_destroy(value: *mut c_char) {
    destroy_string(value);
}

/// Read one C string.
pub(crate) fn read_string(value: *const c_char) -> Result<String, String> {
    if value.is_null() {
        return Err("string pointer is null".to_string());
    }

    let value = unsafe { CStr::from_ptr(value) };
    let value = value.to_str().map_err(|error| error.to_string())?;

    Ok(value.to_string())
}

/// Read one byte slice.
pub(crate) fn read_bytes(bytes: *const u8, len: usize) -> Result<Vec<u8>, String> {
    if len == 0 {
        return Ok(Vec::new());
    }
    if bytes.is_null() {
        return Err("byte pointer is null".to_string());
    }

    let bytes = unsafe { slice::from_raw_parts(bytes, len) };

    Ok(bytes.to_vec())
}

/// Convert one Rust string into an owned C string.
pub(crate) fn c_string(value: String) -> Result<*mut c_char, String> {
    CString::new(value)
        .map(CString::into_raw)
        .map_err(|error| error.to_string())
}

/// Destroy one owned C string.
pub(crate) fn destroy_string(value: *mut c_char) {
    if value.is_null() {
        return;
    }

    unsafe {
        drop(CString::from_raw(value));
    }
}

/// Convert one vector into an owned C array.
pub(crate) fn owned_array<T>(values: Vec<T>) -> (*mut T, usize) {
    let len = values.len();
    let values = values.into_boxed_slice();
    let ptr = Box::into_raw(values).cast::<T>();

    (ptr, len)
}

/// Destroy one owned C array.
pub(crate) unsafe fn destroy_array<T>(ptr: *mut T, len: usize, mut destroy: impl FnMut(&mut T)) {
    if ptr.is_null() {
        return;
    }

    let slice = ptr::slice_from_raw_parts_mut(ptr, len);
    let mut values = unsafe { Box::from_raw(slice) };
    for value in values.iter_mut() {
        destroy(value);
    }
}

/// Write one output pointer.
pub(crate) fn write_out<T>(out: *mut T, value: T, message: &str) -> Result<(), String> {
    if out.is_null() {
        return Err(message.to_string());
    }

    unsafe {
        *out = value;
    }

    Ok(())
}
