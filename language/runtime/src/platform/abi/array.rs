use std::marker::PhantomData;

use destack_vm as vm;

use super::slice::VmSliceBuilder;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformError;

use super::{VmAggregateCodec, VmCollectionElement, VmSlice};

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

/// Narrow one decoded VM collection length to `u32`.
fn vm_len_u32(len: u64, name: &str, expected: &str) -> RuntimeResult<u32> {
    u32::try_from(len).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            name,
            format!("{expected} length exceeds u32"),
        ))
        .boxed()
    })
}

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
    /// Backing storage for the element payload.
    pub data: vm::Word,
    /// Number of elements in the array.
    pub len: u32,
    /// Allocated capacity in elements.
    pub capacity: u32,
    /// Marker for the element type.
    pub _marker: PhantomData<T>,
}

/// Builder for one VM array payload.
#[derive(Debug)]
pub(crate) struct VmArrayBuilder<T> {
    /// The underlying slice builder.
    slice: VmSliceBuilder<T>,
}

impl<T> VmArray<T> {
    /// Decode a VM array from one array value.
    pub fn from_value(
        context: &vm::ExternalReadContext<'_, '_>,
        value: vm::Word,
        name: &str,
        expected: &str,
    ) -> RuntimeResult<Self> {
        let value_ref = context.value_ref(value).map_err(|_error| {
            RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed()
        })?;

        // validate the aggregate arity first
        let field_count = value_ref.field_count();
        if field_count != 2 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                name,
                format!("expected {expected} with 2 fields"),
            ))
            .boxed());
        }

        // decode backing slice + capacity
        let slice_value = value_ref.field_value(0).map_err(|_error| {
            RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed()
        })?;
        let capacity_value = value_ref.field_value(1).map_err(|_error| {
            RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed()
        })?;
        let slice = VmSlice::<T>::from_value(context, slice_value, name, expected)?;

        let capacity = capacity_value.as_uint();
        Ok(Self {
            data: slice.data,
            len: slice.len,
            capacity: vm_len_u32(capacity, name, expected)?,
            _marker: PhantomData::<T>,
        })
    }

    /// Encode this VM array into one array value.
    pub fn to_value(
        self,
        context: &mut vm::ExternalWriteContext<'_, '_>,
    ) -> RuntimeResult<vm::Word> {
        let slice = VmSlice {
            data: self.data,
            len: self.len,
            _marker: PhantomData::<T>,
        }
        .to_value(context)?;

        context
            .materialize_builtin_array_value(slice, self.capacity as usize)
            .map_err(|error| RuntimeError::from(error).boxed())
    }
}

impl<T: VmCollectionElement> VmArray<T> {
    /// Read the encoded VM values stored in this array.
    pub fn values(
        &self,
        context: &vm::ExternalReadContext<'_, '_>,
    ) -> RuntimeResult<Vec<vm::Word>> {
        VmSlice {
            data: self.data,
            len: self.len,
            _marker: PhantomData::<T>,
        }
        .values(context)
    }

    /// Begin one exact-size VM array builder.
    pub(crate) fn builder(
        context: &mut vm::ExternalWriteContext<'_, '_>,
        len: usize,
    ) -> RuntimeResult<VmArrayBuilder<T>> {
        let slice = VmSlice::builder(context, len)?;

        Ok(VmArrayBuilder { slice })
    }
}

impl<T: VmCollectionElement> VmArrayBuilder<T> {
    /// Push one decoded element into the final VM array storage.
    pub(crate) fn push(
        &mut self,
        context: &mut vm::ExternalWriteContext<'_, '_>,
        value: T,
    ) -> RuntimeResult<()> {
        self.slice.push(context, value)
    }

    /// Finish the array once all elements have been written.
    pub(crate) fn finish(self) -> RuntimeResult<VmArray<T>> {
        let slice = self.slice.finish()?;

        Ok(VmArray {
            data: slice.data,
            len: slice.len,
            capacity: slice.len,
            _marker: slice._marker,
        })
    }
}

impl<T: VmCollectionElement> VmArray<T> {
    /// Allocate a VM array from decoded values.
    pub fn from_values(
        context: &mut vm::ExternalWriteContext<'_, '_>,
        values: &[T],
    ) -> RuntimeResult<Self> {
        let mut builder = Self::builder(context, values.len())?;

        // encode each element directly into the final storage
        for value in values.iter().copied() {
            builder.push(context, value)?;
        }

        builder.finish()
    }

    /// Read the VM array into a Vec of decoded values.
    pub fn read_values(&self, context: &vm::ExternalReadContext<'_, '_>) -> RuntimeResult<Vec<T>> {
        VmSlice {
            data: self.data,
            len: self.len,
            _marker: PhantomData::<T>,
        }
        .read_values(context)
    }

    /// Visit each decoded VM array value without allocating an intermediate Vec.
    pub fn try_for_each_value(
        &self,
        context: &vm::ExternalReadContext<'_, '_>,
        visit: impl FnMut(T) -> RuntimeResult<()>,
    ) -> RuntimeResult<()> {
        VmSlice {
            data: self.data,
            len: self.len,
            _marker: PhantomData::<T>,
        }
        .try_for_each_value(context, visit)
    }

    /// Visit each decoded VM array value with context access.
    pub fn try_for_each_value_with_context(
        &self,
        context: &vm::ExternalReadContext<'_, '_>,
        visit: impl FnMut(&vm::ExternalReadContext<'_, '_>, T) -> RuntimeResult<()>,
    ) -> RuntimeResult<()> {
        VmSlice {
            data: self.data,
            len: self.len,
            _marker: PhantomData::<T>,
        }
        .try_for_each_value_with_context(context, visit)
    }

    /// Write decoded values into the VM array.
    pub fn write_values(
        &self,
        context: &mut vm::ExternalWriteContext<'_, '_>,
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
        context: &vm::ExternalReadContext<'_, '_>,
        value: vm::Word,
    ) -> RuntimeResult<Self> {
        VmArray::from_value(context, value, "value", "array")
    }

    fn decode_value_ref_with_context(
        context: &vm::ExternalReadContext<'_, '_>,
        value_ref: &vm::VmValueRef<'_, '_>,
    ) -> RuntimeResult<Self> {
        if value_ref.field_count() != 2 {
            return Err(
                RuntimeError::from(PlatformError::invalid_argument_type("value", "array")).boxed(),
            );
        }

        let slice =
            <VmSlice<T> as VmAggregateCodec>::decode_field_with_context(context, value_ref, 0)?;
        let capacity_value = value_ref
            .field_value(1)
            .map_err(Box::<RuntimeError>::from)?;
        let capacity = capacity_value.as_uint();

        Ok(Self {
            data: slice.data,
            len: slice.len,
            capacity: vm_len_u32(capacity, "value", "array")?,
            _marker: PhantomData,
        })
    }

    fn encode_with_context(
        self,
        context: &mut vm::ExternalWriteContext<'_, '_>,
    ) -> RuntimeResult<vm::Word> {
        self.to_value(context)
    }
}

impl<T: Copy> VmCollectionElement for VmArray<T> {}

impl VmArray<u8> {
    /// Allocate a VM array from raw bytes.
    pub fn from_bytes(
        context: &mut vm::ExternalWriteContext<'_, '_>,
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

    /// Borrow or copy a byte array from the VM.
    pub fn bytes<'a>(
        &self,
        context: &'a vm::ExternalReadContext<'_, '_>,
    ) -> RuntimeResult<std::borrow::Cow<'a, [u8]>> {
        VmSlice {
            data: self.data,
            len: self.len,
            _marker: PhantomData::<u8>,
        }
        .bytes(context)
    }

    /// Read a byte array from the VM.
    pub fn read_bytes(&self, context: &vm::ExternalReadContext<'_, '_>) -> RuntimeResult<Vec<u8>> {
        Ok(self.bytes(context)?.into_owned())
    }

    /// Write a byte array into the VM.
    pub fn write_bytes(
        &self,
        context: &mut vm::ExternalWriteContext<'_, '_>,
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
