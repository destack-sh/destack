use std::ptr;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;

/// FFI string reference for native bindings.
/// Immutable by design: native bindings must not mutate shared runtime strings.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PlatformStringRef {
    /// Pointer to UTF-8 bytes.
    pub data: *const u8,
    /// Length of the UTF-8 byte slice.
    pub len: u32,
}

// safety: points into PlatformContext-owned strings that stay immutable for the platform lifetime
unsafe impl Send for PlatformStringRef {}
// safety: points into PlatformContext-owned strings that stay immutable for the platform lifetime
unsafe impl Sync for PlatformStringRef {}

impl PlatformStringRef {
    /// View the reference as a UTF-8 string.
    pub unsafe fn as_str<'a>(self) -> RuntimeResult<&'a str> {
        if self.data.is_null() && self.len != 0 {
            return Err(RuntimeError::platform(PlatformError::null_pointer("string.data")).boxed());
        }
        // map the raw bytes into a string
        let bytes = unsafe { std::slice::from_raw_parts(self.data, self.len as usize) };
        std::str::from_utf8(bytes).map_err(|_| {
            RuntimeError::platform(PlatformError::invalid_argument_value(
                "value",
                "invalid utf8 string",
            ))
            .boxed()
        })
    }
}

impl From<&str> for PlatformStringRef {
    /// Build a string reference from a string slice.
    fn from(value: &str) -> Self {
        Self {
            data: value.as_ptr(),
            len: value.len() as u32,
        }
    }
}

impl From<&String> for PlatformStringRef {
    /// Build a string reference from an owned string reference.
    fn from(value: &String) -> Self {
        Self::from(value.as_str())
    }
}

/// FFI slice of string references.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PlatformStringSlice {
    /// Pointer to string references.
    pub data: *const PlatformStringRef,
    /// Number of string references.
    pub len: u32,
}

// safety: points into PlatformContext-owned string references that stay immutable
unsafe impl Send for PlatformStringSlice {}
// safety: points into PlatformContext-owned string references that stay immutable
unsafe impl Sync for PlatformStringSlice {}

impl PlatformStringSlice {
    /// Build a slice from string references.
    pub fn from_slice(values: &[PlatformStringRef]) -> Self {
        let data = if values.is_empty() {
            ptr::null()
        } else {
            values.as_ptr()
        };

        Self {
            data,
            len: values.len() as u32,
        }
    }
}
