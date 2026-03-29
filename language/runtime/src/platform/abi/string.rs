use std::ptr;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;

/// Narrow one ABI string length to `u32`.
fn abi_len_u32(len: usize, label: &str) -> u32 {
    u32::try_from(len).unwrap_or_else(|_| panic!("native ABI {label} length exceeds u32: {len}"))
}

/// FFI string reference for native bindings.
/// Immutable by design: native bindings must not mutate shared runtime strings.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NativeStringRef {
    /// Pointer to UTF-8 bytes.
    pub data: *const u8,
    /// Length of the UTF-8 byte slice.
    pub len: u32,
}

// safety: points into runtime-owned strings that stay immutable for the reference lifetime
unsafe impl Send for NativeStringRef {}
// safety: points into runtime-owned strings that stay immutable for the reference lifetime
unsafe impl Sync for NativeStringRef {}

impl NativeStringRef {
    /// View the reference as a UTF-8 string.
    pub unsafe fn as_str<'a>(self) -> RuntimeResult<&'a str> {
        if self.len == 0 {
            return Ok("");
        }

        if self.data.is_null() && self.len != 0 {
            return Err(RuntimeError::from(PlatformError::null_pointer("string.data")).boxed());
        }

        // map the raw bytes into a string
        let bytes = unsafe { std::slice::from_raw_parts(self.data, self.len as usize) };
        std::str::from_utf8(bytes).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "value",
                "invalid utf8 string",
            ))
            .boxed()
        })
    }
}

impl From<&str> for NativeStringRef {
    /// Build a string reference from a string slice.
    fn from(value: &str) -> Self {
        Self {
            data: value.as_ptr(),
            len: abi_len_u32(value.len(), "string"),
        }
    }
}

impl From<&String> for NativeStringRef {
    /// Build a string reference from an owned string reference.
    fn from(value: &String) -> Self {
        Self::from(value.as_str())
    }
}

/// FFI slice of string references.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NativeStringSlice {
    /// Pointer to string references.
    pub data: *const NativeStringRef,
    /// Number of string references.
    pub len: u32,
}

// safety: points into runtime-owned string references that stay immutable
unsafe impl Send for NativeStringSlice {}
// safety: points into runtime-owned string references that stay immutable
unsafe impl Sync for NativeStringSlice {}

impl NativeStringSlice {
    /// Build a slice from string references.
    pub fn from_slice(values: &[NativeStringRef]) -> Self {
        let data = if values.is_empty() {
            ptr::null()
        } else {
            values.as_ptr()
        };

        Self {
            data,
            len: abi_len_u32(values.len(), "string slice"),
        }
    }

    /// View the slice as an immutable slice.
    pub unsafe fn as_slice<'a>(self) -> RuntimeResult<&'a [NativeStringRef]> {
        if self.len == 0 {
            return Ok(&[]);
        }

        if self.data.is_null() && self.len != 0 {
            return Err(RuntimeError::from(PlatformError::null_pointer("slice.data")).boxed());
        }

        // safety: caller guarantees the slice is valid for the lifetime
        Ok(unsafe { std::slice::from_raw_parts(self.data, self.len as usize) })
    }
}

#[cfg(test)]
mod tests {
    use super::{NativeStringRef, NativeStringSlice};

    /// Empty native strings decode without touching null pointers.
    #[test]
    fn test_empty_native_string_ref_allows_null_pointer() {
        let value = NativeStringRef {
            data: std::ptr::null(),
            len: 0,
        };

        let value = unsafe { value.as_str() }.expect("empty string should decode");

        assert_eq!(value, "");
    }

    /// Empty native string slices decode without touching null pointers.
    #[test]
    fn test_empty_native_string_slice_allows_null_pointer() {
        let value = NativeStringSlice {
            data: std::ptr::null(),
            len: 0,
        };

        let value = unsafe { value.as_slice() }.expect("empty slice should decode");

        assert!(value.is_empty());
    }
}
