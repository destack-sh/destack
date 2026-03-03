use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;

/// FFI slice of raw values for native bindings.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NativeSlice<T> {
    /// Pointer to the element data.
    pub data: *mut T,
    /// Number of elements in the slice.
    pub len: u32,
}

// safety: raw slice pointers are externally synchronized
unsafe impl<T> Send for NativeSlice<T> {}
// safety: raw slice pointers are externally synchronized
unsafe impl<T> Sync for NativeSlice<T> {}

impl<T> NativeSlice<T> {
    /// View the slice as an immutable slice.
    pub unsafe fn as_slice<'a>(self) -> RuntimeResult<&'a [T]> {
        if self.data.is_null() && self.len != 0 {
            return Err(RuntimeError::from(PlatformError::null_pointer("slice.data")).boxed());
        }
        // safety: caller guarantees the slice is valid for the lifetime
        Ok(unsafe { std::slice::from_raw_parts(self.data, self.len as usize) })
    }

    /// View the slice as a mutable slice.
    pub unsafe fn as_mut_slice<'a>(self) -> RuntimeResult<&'a mut [T]> {
        if self.data.is_null() && self.len != 0 {
            return Err(RuntimeError::from(PlatformError::null_pointer("slice.data")).boxed());
        }
        // safety: caller guarantees the slice is valid for the lifetime
        Ok(unsafe { std::slice::from_raw_parts_mut(self.data, self.len as usize) })
    }
}
