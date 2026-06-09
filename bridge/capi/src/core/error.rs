use std::ffi::{CString, c_char};
use std::ptr;

use super::destroy_string;

/// C ABI error handle.
#[repr(C)]
#[derive(Debug)]
pub struct DestackError {
    /// Owned error message.
    pub(crate) message: *mut c_char,
}

/// Return one error message.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_error_message(error: *const DestackError) -> *const c_char {
    if error.is_null() {
        return ptr::null();
    }

    unsafe { (*error).message.cast_const() }
}

/// Destroy one error handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_error_destroy(error: *mut DestackError) {
    if error.is_null() {
        return;
    }

    let error = unsafe { Box::from_raw(error) };
    destroy_string(error.message);
}

/// Clear one error output pointer.
pub(crate) fn clear_error(error: *mut *mut DestackError) {
    if !error.is_null() {
        unsafe {
            *error = ptr::null_mut();
        }
    }
}

/// Set one error output pointer.
pub(crate) fn set_error(error: *mut *mut DestackError, message: String) {
    if error.is_null() {
        return;
    }

    let message = match CString::new(message) {
        Ok(message) => message.into_raw(),
        Err(_error) => c"bridge error contained a nul byte".to_owned().into_raw(),
    };
    unsafe {
        *error = Box::into_raw(Box::new(DestackError { message }));
    }
}
