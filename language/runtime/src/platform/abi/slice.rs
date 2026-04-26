use std::borrow::Cow;
use std::marker::PhantomData;

use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformError;

use super::{VmAggregateCodec, VmCollectionElement, VmCollectionStorage, VmValueCodec};

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

        Ok(unsafe { std::slice::from_raw_parts(self.data, self.len as usize) })
    }

    /// View the slice as a mutable slice.
    pub unsafe fn as_mut_slice<'a>(self) -> RuntimeResult<&'a mut [T]> {
        if self.data.is_null() && self.len != 0 {
            return Err(RuntimeError::from(PlatformError::null_pointer("slice.data")).boxed());
        }

        Ok(unsafe { std::slice::from_raw_parts_mut(self.data, self.len as usize) })
    }
}

/// VM slice representation for platform bindings.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VmSlice<T> {
    /// Backing storage for the element payload.
    pub data: vm::Word,
    /// Number of elements in the slice.
    pub len: u32,
    /// Marker for the element type.
    pub _marker: PhantomData<T>,
}

/// Builder for one VM slice payload.
#[derive(Debug)]
pub(crate) struct VmSliceBuilder<T> {
    /// Backing storage for the final element payload.
    data: vm::Word,
    /// Number of elements expected in the slice.
    len: u32,
    /// Number of elements written so far.
    written: usize,
    /// Marker for the element type.
    _marker: PhantomData<T>,
}

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

/// Narrow one encoded VM collection length to `u32`.
fn abi_len_u32(len: usize, label: &str) -> RuntimeResult<u32> {
    u32::try_from(len).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            format!("{label} length exceeds u32"),
        ))
        .boxed()
    })
}

impl<T> VmSlice<T> {
    /// Decode a VM slice from one typed slice value.
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

        // decode backing storage + length
        let data_value = value_ref.field_value(0).map_err(|_error| {
            RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed()
        })?;
        let len_value = value_ref.field_value(1).map_err(|_error| {
            RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed()
        })?;

        let len = len_value.as_uint();

        Ok(Self {
            data: data_value,
            len: vm_len_u32(len, name, expected)?,
            _marker: PhantomData,
        })
    }

    /// Encode this VM slice into one typed slice value.
    pub fn to_value(
        self,
        context: &mut vm::ExternalWriteContext<'_, '_>,
    ) -> RuntimeResult<vm::Word> {
        context
            .materialize_builtin_slice_value(self.data, self.len as usize)
            .map_err(|error| RuntimeError::from(error).boxed())
    }
}

impl<T: VmCollectionElement> VmSlice<T> {
    /// Read the encoded VM values stored in this slice.
    pub fn values(
        &self,
        context: &vm::ExternalReadContext<'_, '_>,
    ) -> RuntimeResult<Vec<vm::Word>> {
        // empty collections do not touch backing storage
        if self.len == 0 {
            return Ok(Vec::new());
        }

        // route byte payloads through raw byte storage
        if is_byte_element_type::<T>() {
            let data = self.data.as_raw_pointer();
            let values = context
                .raw_values(data)
                .map_err(|error| RuntimeError::from(error).boxed())?;

            if values.len() != self.len as usize {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "slice",
                    "slice length mismatch",
                ))
                .boxed());
            }

            return Ok(values);
        }

        let data = self.data.as_heap_reference();
        let values = context
            .heap_values(data)
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

    /// Begin one exact-size VM slice builder.
    pub(crate) fn builder(
        context: &mut vm::ExternalWriteContext<'_, '_>,
        len: usize,
    ) -> RuntimeResult<VmSliceBuilder<T>> {
        let len_u32 = abi_len_u32(len, "slice")?;

        // empty collections use one null backing pointer
        if len == 0 {
            let data = if is_byte_element_type::<T>() {
                vm::Word::raw_pointer(vm::RawPointer::NULL)
            } else {
                vm::Word::heap_reference(vm::HeapReference::NULL)
            };

            return Ok(VmSliceBuilder {
                data,
                len: len_u32,
                written: 0,
                _marker: PhantomData,
            });
        }

        // route byte payloads through raw byte storage
        if is_byte_element_type::<T>() {
            let data = context
                .allocate_zeroed_raw_bytes(len)
                .map_err(|error| RuntimeError::from(error).boxed())?;

            return Ok(VmSliceBuilder {
                data: vm::Word::raw_pointer(data),
                len: len_u32,
                written: 0,
                _marker: PhantomData,
            });
        }

        let data = context
            .allocate_heap_value_slots(len)
            .map_err(|error| RuntimeError::from(error).boxed())?;

        Ok(VmSliceBuilder {
            data: vm::Word::heap_reference(data),
            len: len_u32,
            written: 0,
            _marker: PhantomData,
        })
    }
}

impl<T: VmCollectionElement> VmSliceBuilder<T> {
    /// Push one decoded element into the final VM slice storage.
    pub(crate) fn push(
        &mut self,
        context: &mut vm::ExternalWriteContext<'_, '_>,
        value: T,
    ) -> RuntimeResult<()> {
        // enforce the declared element count exactly
        if self.written >= self.len as usize {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "slice",
                "slice builder overflow",
            ))
            .boxed());
        }

        // route byte payloads through raw byte storage
        if is_byte_element_type::<T>() {
            // safety: the type check above guarantees `T` is exactly `u8`
            let byte = unsafe { std::mem::transmute_copy::<T, u8>(&value) };
            let data = self.data.as_raw_pointer();
            context
                .write_raw_byte(data, self.written, byte)
                .map_err(|error| RuntimeError::from(error).boxed())?;
            self.written += 1;

            return Ok(());
        }

        let value = T::encode_with_context(value, context)?;
        let data = self.data.as_heap_reference();
        context
            .write_heap_value(data, self.written, value)
            .map_err(|error| RuntimeError::from(error).boxed())?;
        self.written += 1;

        Ok(())
    }

    /// Finish the slice once all elements have been written.
    pub(crate) fn finish(self) -> RuntimeResult<VmSlice<T>> {
        // require exact initialization before exposing the slice
        if self.written != self.len as usize {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "slice",
                "slice builder length mismatch",
            ))
            .boxed());
        }

        Ok(VmSlice {
            data: self.data,
            len: self.len,
            _marker: self._marker,
        })
    }
}

/// Report whether one VM slice element type uses raw byte storage.
fn is_byte_element_type<T: VmCollectionElement>() -> bool {
    T::STORAGE == VmCollectionStorage::Bytes
}

/// View one typed slice as raw bytes when the element type is `u8`.
fn byte_slice_from_values<T: VmCollectionElement>(values: &[T]) -> Option<&[u8]> {
    // non-byte element types stay on the packed-value path
    if !is_byte_element_type::<T>() {
        return None;
    }

    // safety: the type check above guarantees `T` is exactly `u8`
    Some(unsafe { std::slice::from_raw_parts(values.as_ptr() as *const u8, values.len()) })
}

/// Reinterpret one owned byte vector as one owned `u8` vector.
fn values_from_byte_vec<T: VmCollectionElement>(bytes: Vec<u8>) -> Option<Vec<T>> {
    // non-byte element types stay on the packed-value path
    if !is_byte_element_type::<T>() {
        return None;
    }

    // safety: the type check above guarantees `T` is exactly `u8`
    Some(unsafe { std::mem::transmute::<Vec<u8>, Vec<T>>(bytes) })
}

impl<T: VmCollectionElement> VmSlice<T> {
    /// Visit each decoded VM slice value with context access.
    pub fn try_for_each_value_with_context(
        &self,
        context: &vm::ExternalReadContext<'_, '_>,
        mut visit: impl FnMut(&vm::ExternalReadContext<'_, '_>, T) -> RuntimeResult<()>,
    ) -> RuntimeResult<()> {
        // empty collections do not touch backing storage
        if self.len == 0 {
            return Ok(());
        }

        // route byte payloads through raw byte storage
        if is_byte_element_type::<T>() {
            let bytes = VmSlice::<u8> {
                data: self.data,
                len: self.len,
                _marker: PhantomData,
            }
            .read_bytes(context)?;

            // safety: the type check above guarantees `T` is exactly `u8`
            for byte in bytes {
                let value = unsafe { std::mem::transmute_copy::<u8, T>(&byte) };
                visit(context, value)?;
            }

            return Ok(());
        }

        // validate the packed heap value length first
        let expected_byte_len = (self.len as usize)
            .checked_mul(vm::Word::BYTE_LEN)
            .ok_or_else(|| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "slice",
                    "slice byte length overflow",
                ))
                .boxed()
            })?;
        let data = self.data.as_heap_reference();
        let values = context
            .heap_values(data)
            .map_err(|error| RuntimeError::from(error).boxed())?;
        let byte_len = values
            .len()
            .checked_mul(vm::Word::BYTE_LEN)
            .ok_or_else(|| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "slice",
                    "slice byte length overflow",
                ))
                .boxed()
            })?;

        if byte_len != expected_byte_len || values.len() != self.len as usize {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "slice",
                "slice length mismatch",
            ))
            .boxed());
        }

        // decode each packed value lane directly
        for index in 0..self.len as usize {
            let value = context
                .heap_value_at(data, index)
                .map_err(|error| RuntimeError::from(error).boxed())?;
            let value = T::decode_with_context(context, value)?;
            visit(context, value)?;
        }

        Ok(())
    }

    /// Visit each decoded VM slice value without allocating an intermediate Vec.
    pub fn try_for_each_value(
        &self,
        context: &vm::ExternalReadContext<'_, '_>,
        mut visit: impl FnMut(T) -> RuntimeResult<()>,
    ) -> RuntimeResult<()> {
        self.try_for_each_value_with_context(context, |_context, value| visit(value))
    }

    /// Allocate a VM slice from decoded values.
    pub fn from_values(
        context: &mut vm::ExternalWriteContext<'_, '_>,
        values: &[T],
    ) -> RuntimeResult<Self> {
        // route byte payloads through raw byte storage
        if let Some(bytes) = byte_slice_from_values(values) {
            let slice = VmSlice::<u8>::from_bytes(context, bytes)?;

            return Ok(Self {
                data: slice.data,
                len: slice.len,
                _marker: PhantomData,
            });
        }

        let mut builder = Self::builder(context, values.len())?;

        // encode each element directly into the final storage
        for value in values.iter().copied() {
            builder.push(context, value)?;
        }

        builder.finish()
    }

    /// Read the VM slice into a Vec of decoded values.
    pub fn read_values(&self, context: &vm::ExternalReadContext<'_, '_>) -> RuntimeResult<Vec<T>> {
        // route byte payloads through raw byte storage
        if is_byte_element_type::<T>() {
            let bytes = VmSlice::<u8> {
                data: self.data,
                len: self.len,
                _marker: PhantomData,
            }
            .read_bytes(context)?;

            let values = values_from_byte_vec(bytes).ok_or_else(|| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "slice",
                    "byte slice type mismatch",
                ))
                .boxed()
            })?;

            return Ok(values);
        }

        let mut decoded = Vec::with_capacity(self.len as usize);

        self.try_for_each_value(context, |value| {
            decoded.push(value);
            Ok(())
        })?;

        Ok(decoded)
    }

    /// Write decoded values into the VM slice.
    pub fn write_values(
        &self,
        context: &mut vm::ExternalWriteContext<'_, '_>,
        values: &[T],
    ) -> RuntimeResult<()> {
        // empty collections do not touch backing storage
        if self.len == 0 {
            if values.is_empty() {
                return Ok(());
            }

            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "slice",
                "slice length mismatch",
            ))
            .boxed());
        }

        // route byte payloads through raw byte storage
        if let Some(bytes) = byte_slice_from_values(values) {
            return VmSlice::<u8> {
                data: self.data,
                len: self.len,
                _marker: PhantomData,
            }
            .write_bytes(context, bytes);
        }

        // validate the logical element count first
        if values.len() != self.len as usize {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "slice",
                "slice length mismatch",
            ))
            .boxed());
        }

        // encode each packed element directly into the existing heap storage
        let data = self.data.as_heap_reference();
        for (index, value) in values.iter().copied().enumerate() {
            let encoded = T::encode_with_context(value, context)?;
            context
                .write_heap_value(data, index, encoded)
                .map_err(|error| RuntimeError::from(error).boxed())?;
        }

        Ok(())
    }
}

impl<T: Copy> VmAggregateCodec for VmSlice<T> {
    fn decode_with_context(
        context: &vm::ExternalReadContext<'_, '_>,
        value: vm::Word,
    ) -> RuntimeResult<Self> {
        VmSlice::from_value(context, value, "value", "slice")
    }

    fn decode_value_ref_with_context(
        context: &vm::ExternalReadContext<'_, '_>,
        value_ref: &vm::VmValueRef<'_, '_>,
    ) -> RuntimeResult<Self> {
        if value_ref.field_count() != 2 {
            return Err(
                RuntimeError::from(PlatformError::invalid_argument_type("value", "slice")).boxed(),
            );
        }

        let data = value_ref
            .field_value(0)
            .map_err(Box::<RuntimeError>::from)?;
        let len = <u32 as VmAggregateCodec>::decode_field_with_context(context, value_ref, 1)?;

        Ok(Self {
            data,
            len,
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

impl<T: Copy> VmCollectionElement for VmSlice<T> {}

impl VmSlice<u8> {
    /// Allocate a VM slice from raw bytes.
    pub fn from_bytes(
        context: &mut vm::ExternalWriteContext<'_, '_>,
        bytes: &[u8],
    ) -> RuntimeResult<Self> {
        let len = abi_len_u32(bytes.len(), "slice")?;
        let data = if bytes.is_empty() {
            vm::RawPointer::NULL
        } else {
            context
                .allocate_raw_bytes(bytes)
                .map_err(|error| RuntimeError::from(error).boxed())?
        };

        Ok(Self {
            data: vm::Word::raw_pointer(data),
            len,
            _marker: PhantomData,
        })
    }

    /// Borrow or copy a byte slice from the VM.
    pub fn bytes<'a>(
        &self,
        context: &'a vm::ExternalReadContext<'_, '_>,
    ) -> RuntimeResult<Cow<'a, [u8]>> {
        // empty collections do not touch backing storage
        if self.len == 0 {
            return Ok(Cow::Borrowed(&[]));
        }

        let expected_len = self.len as usize;
        let data = self.data.as_raw_pointer();

        // prefer true byte storage when the raw allocation size matches
        if let Ok(byte_len) = context.raw_byte_len(data)
            && byte_len == expected_len
        {
            return context
                .raw_bytes_ref(data)
                .map_err(|error| RuntimeError::from(error).boxed());
        }

        // compatibility: some older callers encoded byte slices as packed values
        Ok(Cow::Owned(self.read_values(context)?))
    }

    /// Read a byte slice from the VM.
    pub fn read_bytes(&self, context: &vm::ExternalReadContext<'_, '_>) -> RuntimeResult<Vec<u8>> {
        Ok(self.bytes(context)?.into_owned())
    }

    /// Write a byte slice into the VM.
    pub fn write_bytes(
        &self,
        context: &mut vm::ExternalWriteContext<'_, '_>,
        bytes: &[u8],
    ) -> RuntimeResult<()> {
        // empty collections do not touch backing storage
        if self.len == 0 {
            if bytes.is_empty() {
                return Ok(());
            }

            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "slice",
                "slice length mismatch",
            ))
            .boxed());
        }

        let expected_len = self.len as usize;
        let data = self.data.as_raw_pointer();

        // validate the logical byte length first
        if bytes.len() != expected_len {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "slice",
                "slice length mismatch",
            ))
            .boxed());
        }

        // prefer true byte storage when the raw allocation size matches
        if let Ok(byte_len) = context.raw_byte_len(data)
            && byte_len == expected_len
        {
            context
                .write_raw_bytes(data, bytes)
                .map_err(|error| RuntimeError::from(error).boxed())?;

            return Ok(());
        }

        // compatibility: rewrite legacy packed-value byte slices as packed u8 values
        let encoded = bytes
            .iter()
            .copied()
            .map(VmValueCodec::encode)
            .collect::<Vec<_>>();
        context
            .write_raw_values(data, &encoded)
            .map_err(|error| RuntimeError::from(error).boxed())?;

        Ok(())
    }
}
