use std::marker::PhantomData;

use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{PlatformError, VmValueCodec};

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
        context: &mut vm::ExternalCallContext<'_>,
        value: vm::Value,
        name: &str,
        expected: &str,
    ) -> RuntimeResult<Self> {
        // value must be an aggregate pair
        if value.tag() != vm::ValueTag::Aggregate {
            return Err(
                RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed(),
            );
        }

        // unpack aggregate slots
        let slots = context
            .aggregate_slots(value)
            .map_err(|error| RuntimeError::from(error).boxed())?;
        if slots.len() != 2 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                name,
                format!("expected {expected} with 2 fields"),
            ))
            .boxed());
        }

        // decode pointer + length
        let data = slots[0].as_raw_pointer().ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed()
        })?;
        let (len, width) = slots[1].as_uint_with_width().ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed()
        })?;
        if width != 32 {
            return Err(
                RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed(),
            );
        }

        Ok(Self {
            data,
            len: len as u32,
            _marker: PhantomData,
        })
    }

    /// Encode this VM slice into an aggregate value.
    pub fn to_value(self, context: &mut vm::ExternalCallContext<'_>) -> vm::Value {
        let data = vm::Value::raw_pointer(self.data);
        let len = vm::Value::uint(self.len as u64, 32);
        context.allocate_pair(data, len)
    }

    /// Read the raw VM values stored in this slice.
    pub fn raw_values(
        &self,
        context: &vm::ExternalCallContext<'_>,
    ) -> RuntimeResult<Vec<vm::Value>> {
        let values = context
            .raw_values(self.data)
            .map_err(|error| RuntimeError::from(error).boxed())?;

        if values.len() != self.len as usize {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "slice",
                "slice length mismatch",
            ))
            .boxed());
        }

        Ok(values)
    }
}

impl<T: VmValueCodec> VmSlice<T> {
    /// Allocate a VM slice from decoded values.
    pub fn from_values(
        context: &mut vm::ExternalCallContext<'_>,
        values: &[T],
    ) -> RuntimeResult<Self> {
        let encoded = values.iter().copied().map(T::encode).collect::<Vec<_>>();
        let data = context.allocate_raw_values(encoded);
        Ok(Self {
            data,
            len: values.len() as u32,
            _marker: PhantomData,
        })
    }

    /// Read the VM slice into a Vec of decoded values.
    pub fn read_values(&self, context: &vm::ExternalCallContext<'_>) -> RuntimeResult<Vec<T>> {
        // read raw values from the heap
        let values = context
            .raw_values(self.data)
            .map_err(|error| RuntimeError::from(error).boxed())?;

        // validate length
        if values.len() != self.len as usize {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "slice",
                "slice length mismatch",
            ))
            .boxed());
        }

        // decode each value
        let mut decoded = Vec::with_capacity(values.len());
        for value in values {
            decoded.push(T::decode(value)?);
        }

        Ok(decoded)
    }

    /// Write decoded values into the VM slice.
    pub fn write_values(
        &self,
        context: &mut vm::ExternalCallContext<'_>,
        values: &[T],
    ) -> RuntimeResult<()> {
        if values.len() != self.len as usize {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "slice",
                "slice length mismatch",
            ))
            .boxed());
        }

        let encoded = values.iter().copied().map(T::encode).collect::<Vec<_>>();
        context
            .write_raw_values(self.data, &encoded)
            .map_err(|error| RuntimeError::from(error).boxed())?;
        Ok(())
    }
}

impl VmSlice<u8> {
    /// Allocate a VM slice from raw bytes.
    pub fn from_bytes(context: &mut vm::ExternalCallContext<'_>, bytes: &[u8]) -> Self {
        let data = context.allocate_raw_bytes(bytes);
        Self {
            data,
            len: bytes.len() as u32,
            _marker: PhantomData,
        }
    }

    /// Read a byte slice from the VM.
    pub fn read_bytes(&self, context: &vm::ExternalCallContext<'_>) -> RuntimeResult<Vec<u8>> {
        // read raw bytes from the heap
        let bytes = match context.raw_bytes(self.data) {
            Ok(bytes) => bytes,
            Err(_) => {
                let values = self.read_values(context)?;
                return Ok(values);
            }
        };

        if bytes.len() != self.len as usize {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "slice",
                "slice length mismatch",
            ))
            .boxed());
        }

        Ok(bytes)
    }

    /// Write a byte slice into the VM.
    pub fn write_bytes(
        &self,
        context: &mut vm::ExternalCallContext<'_>,
        bytes: &[u8],
    ) -> RuntimeResult<()> {
        if bytes.len() != self.len as usize {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "slice",
                "slice length mismatch",
            ))
            .boxed());
        }

        if context.write_raw_bytes(self.data, bytes).is_ok() {
            return Ok(());
        }

        let encoded = bytes
            .iter()
            .copied()
            .map(VmValueCodec::encode)
            .collect::<Vec<_>>();
        context
            .write_raw_values(self.data, &encoded)
            .map_err(|error| RuntimeError::from(error).boxed())?;
        Ok(())
    }
}
