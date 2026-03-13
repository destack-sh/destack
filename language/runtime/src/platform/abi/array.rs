use std::marker::PhantomData;

use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{PlatformError, VmAggregateCodec, VmSlice};

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
        if self.len == 0 {
            // safety: dangling pointer is valid for zero-length slices
            return Ok(unsafe {
                std::slice::from_raw_parts(std::ptr::NonNull::dangling().as_ptr(), 0)
            });
        }
        if self.data.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("array.data")).boxed());
        }
        // safety: caller guarantees the slice is valid for the lifetime
        Ok(unsafe { std::slice::from_raw_parts(self.data, self.len as usize) })
    }

    /// View the array as a mutable slice.
    pub unsafe fn as_mut_slice<'a>(self) -> RuntimeResult<&'a mut [T]> {
        if self.len == 0 {
            // safety: dangling pointer is valid for zero-length slices
            return Ok(unsafe {
                std::slice::from_raw_parts_mut(std::ptr::NonNull::dangling().as_ptr(), 0)
            });
        }
        if self.data.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("array.data")).boxed());
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
        context: &vm::ExternalCallContext<'_>,
        value: vm::Value,
        name: &str,
        expected: &str,
    ) -> RuntimeResult<Self> {
        // value must be an aggregate triple
        if value.tag() != vm::ValueTag::Aggregate {
            return Err(
                RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed(),
            );
        }

        // unpack aggregate slots
        let slots = context
            .aggregate_slots(value)
            .map_err(|error| RuntimeError::from(error).boxed())?;
        if slots.len() != 3 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                name,
                format!("expected {expected} with 3 fields"),
            ))
            .boxed());
        }

        // decode length + capacity + pointer
        let (len, len_width) = slots[0].as_uint_with_width().ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed()
        })?;
        if len_width != 32 {
            return Err(
                RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed(),
            );
        }
        let (capacity, capacity_width) = slots[1].as_uint_with_width().ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed()
        })?;
        if capacity_width != 32 {
            return Err(
                RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed(),
            );
        }
        let data = slots[2].as_raw_pointer().ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed()
        })?;

        Ok(Self {
            data,
            len: len as u32,
            capacity: capacity as u32,
            _marker: PhantomData::<T>,
        })
    }

    /// Encode this VM array into an aggregate value.
    pub fn to_value(self, context: &mut vm::ExternalCallContext<'_>) -> RuntimeResult<vm::Value> {
        let len = vm::Value::uint(self.len as u64, 32);
        let capacity = vm::Value::uint(self.capacity as u64, 32);
        let data = vm::Value::raw_pointer(self.data);
        context
            .allocate_aggregate(vec![len, capacity, data])
            .map_err(|error| RuntimeError::from(error).boxed())
    }

    /// Read the raw VM values stored in this array.
    pub fn raw_values(
        &self,
        context: &vm::ExternalCallContext<'_>,
    ) -> RuntimeResult<Vec<vm::Value>> {
        VmSlice {
            data: self.data,
            len: self.len,
            _marker: PhantomData::<T>,
        }
        .raw_values(context)
    }
}

impl<T: VmAggregateCodec> VmArray<T> {
    /// Allocate a VM array from decoded values.
    pub fn from_values(
        context: &mut vm::ExternalCallContext<'_>,
        values: &[T],
    ) -> RuntimeResult<Self> {
        let slice = VmSlice::from_values(context, values)?;
        Ok(Self {
            data: slice.data,
            len: slice.len,
            capacity: slice.len,
            _marker: PhantomData::<T>,
        })
    }

    /// Read the VM array into a Vec of decoded values.
    pub fn read_values(&self, context: &vm::ExternalCallContext<'_>) -> RuntimeResult<Vec<T>> {
        VmSlice {
            data: self.data,
            len: self.len,
            _marker: PhantomData::<T>,
        }
        .read_values(context)
    }

    /// Write decoded values into the VM array.
    pub fn write_values(
        &self,
        context: &mut vm::ExternalCallContext<'_>,
        values: &[T],
    ) -> RuntimeResult<()> {
        VmSlice {
            data: self.data,
            len: self.len,
            _marker: PhantomData::<T>,
        }
        .write_values(context, values)
    }
}

impl<T: Copy> VmAggregateCodec for VmArray<T> {
    fn decode_with_context(
        context: &vm::ExternalCallContext<'_>,
        value: vm::Value,
    ) -> RuntimeResult<Self> {
        VmArray::from_value(context, value, "value", "array")
    }

    fn encode_with_context(
        self,
        context: &mut vm::ExternalCallContext<'_>,
    ) -> RuntimeResult<vm::Value> {
        self.to_value(context)
    }
}

impl VmArray<u8> {
    /// Allocate a VM array from raw bytes.
    pub fn from_bytes(
        context: &mut vm::ExternalCallContext<'_>,
        bytes: &[u8],
    ) -> RuntimeResult<Self> {
        let slice = VmSlice::from_bytes(context, bytes)?;

        Ok(Self {
            data: slice.data,
            len: slice.len,
            capacity: slice.len,
            _marker: PhantomData::<u8>,
        })
    }

    /// Read a byte array from the VM.
    pub fn read_bytes(&self, context: &vm::ExternalCallContext<'_>) -> RuntimeResult<Vec<u8>> {
        VmSlice {
            data: self.data,
            len: self.len,
            _marker: PhantomData::<u8>,
        }
        .read_bytes(context)
    }

    /// Write a byte array into the VM.
    pub fn write_bytes(
        &self,
        context: &mut vm::ExternalCallContext<'_>,
        bytes: &[u8],
    ) -> RuntimeResult<()> {
        VmSlice {
            data: self.data,
            len: self.len,
            _marker: PhantomData::<u8>,
        }
        .write_bytes(context, bytes)
    }
}
