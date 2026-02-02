use std::marker::PhantomData;

use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;

/// FFI slice of raw values.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PlatformSlice<T> {
    /// Pointer to the element data.
    pub data: *mut T,
    /// Number of elements in the slice.
    pub len: u32,
}

// safety: raw slice pointers are externally synchronized
unsafe impl<T> Send for PlatformSlice<T> {}
// safety: raw slice pointers are externally synchronized
unsafe impl<T> Sync for PlatformSlice<T> {}

impl<T> PlatformSlice<T> {
    /// View the slice as an immutable slice.
    pub unsafe fn as_slice<'a>(self) -> RuntimeResult<&'a [T]> {
        if self.data.is_null() && self.len != 0 {
            return Err(RuntimeError::platform(PlatformError::null_pointer("slice.data")).boxed());
        }
        // safety: caller guarantees the slice is valid for the lifetime
        Ok(unsafe { std::slice::from_raw_parts(self.data, self.len as usize) })
    }

    /// View the slice as a mutable slice.
    pub unsafe fn as_mut_slice<'a>(self) -> RuntimeResult<&'a mut [T]> {
        if self.data.is_null() && self.len != 0 {
            return Err(RuntimeError::platform(PlatformError::null_pointer("slice.data")).boxed());
        }
        // safety: caller guarantees the slice is valid for the lifetime
        Ok(unsafe { std::slice::from_raw_parts_mut(self.data, self.len as usize) })
    }
}

/// VM slice representation for platform bindings.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VmSlice<T> {
    /// Pointer to the element storage in the VM heap.
    pub data: vm::RawPointer,
    /// Number of elements in the slice.
    pub len: u32,
    /// Marker for the element type.
    pub _marker: PhantomData<T>,
}

impl<T> VmSlice<T> {
    /// Decode a VM slice from an aggregate value.
    pub fn from_value(
        context: &mut vm::RuntimeContext<'_>,
        value: vm::Value,
        name: &str,
        expected: &str,
    ) -> RuntimeResult<Self> {
        // value must be an aggregate pair
        if value.tag() != vm::ValueTag::Aggregate {
            return Err(RuntimeError::platform(PlatformError::invalid_argument_type(
                name, expected,
            ))
            .boxed());
        }

        // unpack aggregate slots
        let slots = context
            .aggregate_slots(value)
            .map_err(|error| RuntimeError::vm(error).boxed())?;
        if slots.len() != 2 {
            return Err(
                RuntimeError::platform(PlatformError::invalid_argument_value(
                    name,
                    format!("expected {expected} with 2 fields"),
                ))
                .boxed(),
            );
        }

        // decode pointer + length
        let data = slots[0].as_raw_pointer().ok_or_else(|| {
            RuntimeError::platform(PlatformError::invalid_argument_type(name, expected)).boxed()
        })?;
        let (len, width) = slots[1].as_uint_with_width().ok_or_else(|| {
            RuntimeError::platform(PlatformError::invalid_argument_type(name, expected)).boxed()
        })?;
        if width != 32 {
            return Err(RuntimeError::platform(PlatformError::invalid_argument_type(
                name, expected,
            ))
            .boxed());
        }

        Ok(Self {
            data,
            len: len as u32,
            _marker: PhantomData,
        })
    }

    /// Encode this VM slice into an aggregate value.
    pub fn to_value(self, context: &mut vm::RuntimeContext<'_>) -> vm::Value {
        let data = vm::Value::raw_pointer(self.data);
        let len = vm::Value::uint(self.len as u64, 32);
        context.allocate_pair(data, len)
    }
}
