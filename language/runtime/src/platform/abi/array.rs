use std::marker::PhantomData;

use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;

/// FFI array for raw native bindings.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NativeArray<T> {
    /// Pointer to the element data.
    pub data: *mut T,
    /// Number of elements in the array.
    pub len: u32,
    /// Allocated capacity in elements.
    pub capacity: u32,
}

// safety: raw array pointers are externally synchronized
unsafe impl<T> Send for NativeArray<T> {}
// safety: raw array pointers are externally synchronized
unsafe impl<T> Sync for NativeArray<T> {}

impl<T> NativeArray<T> {
    /// View the array as an immutable slice.
    pub unsafe fn as_slice<'a>(self) -> RuntimeResult<&'a [T]> {
        if self.data.is_null() && self.len != 0 {
            return Err(RuntimeError::platform(PlatformError::null_pointer("array.data")).boxed());
        }
        // safety: caller guarantees the slice is valid for the lifetime
        Ok(unsafe { std::slice::from_raw_parts(self.data, self.len as usize) })
    }

    /// View the array as a mutable slice.
    pub unsafe fn as_mut_slice<'a>(self) -> RuntimeResult<&'a mut [T]> {
        if self.data.is_null() && self.len != 0 {
            return Err(RuntimeError::platform(PlatformError::null_pointer("array.data")).boxed());
        }
        // safety: caller guarantees the slice is valid for the lifetime
        Ok(unsafe { std::slice::from_raw_parts_mut(self.data, self.len as usize) })
    }
}

/// VM array representation for platform bindings.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VmArray<T> {
    /// Pointer to the element storage in the VM heap.
    pub data: vm::RawPointer,
    /// Number of elements in the array.
    pub len: u32,
    /// Allocated capacity in elements.
    pub capacity: u32,
    /// Marker for the element type.
    pub _marker: PhantomData<T>,
}

impl<T> VmArray<T> {
    /// Decode a VM array from an aggregate value.
    pub fn from_value(
        context: &mut vm::RuntimeContext<'_>,
        value: vm::Value,
        name: &str,
        expected: &str,
    ) -> RuntimeResult<Self> {
        // value must be an aggregate triple
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
        if slots.len() != 3 {
            return Err(
                RuntimeError::platform(PlatformError::invalid_argument_value(
                    name,
                    format!("expected {expected} with 3 fields"),
                ))
                .boxed(),
            );
        }

        // decode length + capacity + pointer
        let (len, len_width) = slots[0].as_uint_with_width().ok_or_else(|| {
            RuntimeError::platform(PlatformError::invalid_argument_type(name, expected)).boxed()
        })?;
        if len_width != 32 {
            return Err(RuntimeError::platform(PlatformError::invalid_argument_type(
                name, expected,
            ))
            .boxed());
        }
        let (capacity, capacity_width) = slots[1].as_uint_with_width().ok_or_else(|| {
            RuntimeError::platform(PlatformError::invalid_argument_type(name, expected)).boxed()
        })?;
        if capacity_width != 32 {
            return Err(RuntimeError::platform(PlatformError::invalid_argument_type(
                name, expected,
            ))
            .boxed());
        }
        let data = slots[2].as_raw_pointer().ok_or_else(|| {
            RuntimeError::platform(PlatformError::invalid_argument_type(name, expected)).boxed()
        })?;

        Ok(Self {
            data,
            len: len as u32,
            capacity: capacity as u32,
            _marker: PhantomData,
        })
    }

    /// Encode this VM array into an aggregate value.
    pub fn to_value(self, context: &mut vm::RuntimeContext<'_>) -> vm::Value {
        let len = vm::Value::uint(self.len as u64, 32);
        let capacity = vm::Value::uint(self.capacity as u64, 32);
        let data = vm::Value::raw_pointer(self.data);
        context.allocate_aggregate(vec![len, capacity, data])
    }
}
