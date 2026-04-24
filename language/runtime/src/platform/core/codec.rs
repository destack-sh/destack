use std::mem::MaybeUninit;

use super::{NativeAbiCodec, VmAbiCodec};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeAbi, NativeSlice, NativeStringRef, NativeStringSlice, VmAbi};
use crate::platform::fs::{
    OsPath, OsPathBytesVm, OsPathUtf16Vm, OsPathVm, PathBytes, PathBytesAbi, PathBytesVm,
    PathUtf16, PathUtf16Abi, PathUtf16Vm, core as core_fs,
};
use crate::platform::{
    NativeArray, PlatformError, VmAggregateCodec, VmArray, VmCollectionElement, VmSlice,
};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

/// Mechanical native binding codec for one binding call.
pub(crate) struct NativeBindingCodec<'a> {
    /// The active native binding call.
    binding: &'a BindingCallContext,
}

impl<'a> NativeBindingCodec<'a> {
    /// Create one native binding codec for one binding call.
    pub(crate) fn new(binding: &'a BindingCallContext) -> Self {
        Self { binding }
    }

    /// Decode one native binding value into one materialized Rust value.
    pub(crate) unsafe fn decode<T: NativeAbiCodec>(&self, value: T) -> RuntimeResult<T::Value> {
        unsafe { value.into_value() }
    }

    /// Encode one materialized Rust value into one native binding value.
    pub(crate) fn encode<T: NativeAbiCodec>(&self, value: T::Value) -> T {
        T::from_value(self.binding, value)
    }
}

/// Mechanical VM decode codec for one VM call context.
pub(crate) struct VmDecodeCodec<'call, 'vm> {
    /// The active VM call context.
    context: &'call mut vm::ExternalCallContext<'vm>,
}

impl<'call, 'vm> VmDecodeCodec<'call, 'vm> {
    /// Create one VM decode codec for one call context.
    pub(crate) fn new(context: &'call mut vm::ExternalCallContext<'vm>) -> Self {
        Self { context }
    }

    /// Decode one VM binding value into one materialized Rust value.
    pub(crate) fn decode<T: VmAbiCodec>(&mut self, value: T) -> RuntimeResult<T::Value> {
        let context = self.context.read();
        value.into_value(&context)
    }
}

/// Mechanical VM encode codec for one VM call context.
pub(crate) struct VmEncodeCodec<'call, 'vm> {
    /// The active VM call context.
    context: &'call mut vm::ExternalCallContext<'vm>,
}

impl<'call, 'vm> VmEncodeCodec<'call, 'vm> {
    /// Create one VM encode codec for one call context.
    pub(crate) fn new(context: &'call mut vm::ExternalCallContext<'vm>) -> Self {
        Self { context }
    }

    /// Encode one materialized Rust value into one VM binding value.
    pub(crate) fn encode<T: VmAbiCodec>(&mut self, value: T::Value) -> RuntimeResult<T> {
        let mut context = self.context.write();
        T::from_value(&mut context, value)
    }
}

/// Invoke one callback that initializes one out pointer.
pub(crate) fn call_out<T>(call: impl FnOnce(*mut T) -> RuntimeResult<()>) -> RuntimeResult<T> {
    let mut out = MaybeUninit::<T>::uninit();
    call(out.as_mut_ptr())?;

    Ok(unsafe { out.assume_init() })
}

/// Decode one required UTF-8 string slice from one shared blob.
#[cfg(target_os = "android")]
pub(crate) fn decode_required_string(
    blob: &[u8],
    offset: u32,
    len: u32,
    field: &'static str,
    source: &str,
) -> RuntimeResult<String> {
    let start = offset as usize;
    let end = start.saturating_add(len as usize);

    if end > blob.len() {
        return Err(RuntimeError::from(PlatformError::invalid_data(format!(
            "{source} returned one out-of-range {field} string slice"
        )))
        .boxed());
    }

    let value = std::str::from_utf8(&blob[start..end]).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_data(format!(
            "{source} returned one non-utf8 {field} string"
        )))
        .boxed()
    })?;

    Ok(value.to_string())
}

/// Decode one optional UTF-8 string slice from one shared blob.
#[cfg(target_os = "android")]
pub(crate) fn decode_optional_string(
    blob: &[u8],
    offset: u32,
    len: u32,
    field: &'static str,
    source: &str,
) -> RuntimeResult<Option<String>> {
    if len == 0 {
        return Ok(None);
    }

    Ok(Some(decode_required_string(
        blob, offset, len, field, source,
    )?))
}

/// Read one VM string handle and store it in binding-local string storage.
pub(crate) fn store_string_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: vm::StringHandle,
) -> RuntimeResult<NativeStringRef> {
    let context = context.read();
    let value = context
        .string_ref(value)
        .map_err(|error| RuntimeError::from(error).boxed())?;

    Ok(binding.store_string(value.as_str()))
}

/// Read one optional VM string handle and store it in binding-local string storage.
pub(crate) fn optional_store_string_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: Option<vm::StringHandle>,
) -> RuntimeResult<Option<NativeStringRef>> {
    value
        .map(|value| store_string_from_vm(binding, context, value))
        .transpose()
}

/// Intern one native string reference into one VM string handle.
pub(crate) fn intern_string_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeStringRef,
) -> RuntimeResult<vm::StringHandle> {
    let value = unsafe { value.as_str()? };

    context
        .string_handle(value)
        .map_err(Box::<RuntimeError>::from)
}

/// Intern one optional native string reference into one optional VM string handle.
pub(crate) fn optional_intern_string_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: Option<NativeStringRef>,
) -> RuntimeResult<Option<vm::StringHandle>> {
    value
        .map(|value| intern_string_to_vm(context, value))
        .transpose()
}

/// Read one VM string-handle slice and store it in binding-local string-slice storage.
pub(crate) fn store_string_slice_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    values: VmSlice<vm::StringHandle>,
) -> RuntimeResult<NativeStringSlice> {
    let context = context.read();
    binding.store_string_slice_with(values.len as usize, |builder| {
        // decode each string directly into call-arena storage
        values.try_for_each_value_with_context(&context, |context, value| {
            let value = context
                .string_ref(value)
                .map_err(|error| RuntimeError::from(error).boxed())?;
            builder.push(binding.store_string(value.as_str()));
            Ok(())
        })
    })
}

/// Read one VM byte slice and store it in binding-local slice storage.
pub(crate) fn store_bytes_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: VmSlice<u8>,
) -> RuntimeResult<NativeSlice<u8>> {
    let context = context.read();
    let value = value.bytes(&context)?;

    Ok(binding.store_slice_copy(value.as_ref()))
}

/// Read one optional VM byte slice and store it in binding-local slice storage.
pub(crate) fn optional_store_bytes_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: Option<VmSlice<u8>>,
) -> RuntimeResult<Option<NativeSlice<u8>>> {
    value
        .map(|value| store_bytes_from_vm(binding, context, value))
        .transpose()
}

/// Read one VM value slice and store it in binding-local slice storage.
pub(crate) fn store_values_from_vm<T: Copy + VmAggregateCodec + VmCollectionElement + 'static>(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: VmSlice<T>,
) -> RuntimeResult<NativeSlice<T>> {
    let context = context.read();
    binding.store_slice_with(value.len as usize, |builder| {
        // decode each value directly into call-arena storage
        value.try_for_each_value(&context, |item| {
            builder.push(item);
            Ok(())
        })
    })
}

/// Read one VM byte array and store it in binding-local array storage.
pub(crate) fn store_bytes_array_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: VmArray<u8>,
) -> RuntimeResult<NativeArray<u8>> {
    let context = context.read();
    let value = value.bytes(&context)?;

    Ok(binding.store_array_copy(value.as_ref()))
}

/// Read one VM value array and store it in binding-local array storage.
pub(crate) fn store_values_array_from_vm<
    T: Copy + VmAggregateCodec + VmCollectionElement + 'static,
>(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: VmArray<T>,
) -> RuntimeResult<NativeArray<T>> {
    let context = context.read();
    binding.store_array_with(value.len as usize, |builder| {
        // decode each value directly into call-arena storage
        value.try_for_each_value(&context, |item| {
            builder.push(item);
            Ok(())
        })
    })
}

/// Encode one native byte slice as one VM byte slice.
pub(crate) fn bytes_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    let value = unsafe { value.as_slice()? };
    let mut context = context.write();

    VmSlice::from_bytes(&mut context, value)
}

/// Encode one native byte array as one VM byte array.
pub(crate) fn bytes_array_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeArray<u8>,
) -> RuntimeResult<VmArray<u8>> {
    let value = unsafe { value.as_slice()? };
    let mut context = context.write();

    VmArray::from_bytes(&mut context, value)
}

/// Encode one native byte-array array as one VM byte-array array.
pub(crate) fn bytes_array_array_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    values: NativeArray<NativeArray<u8>>,
) -> RuntimeResult<VmArray<VmArray<u8>>> {
    let values = unsafe { values.as_slice()? };
    let mut context = context.write();
    let len = values.len();
    let len_u32 = u32::try_from(len).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "array",
            "array length exceeds u32",
        ))
        .boxed()
    })?;
    let data = context
        .allocate_heap_value_slots(len)
        .map_err(Box::<RuntimeError>::from)?;

    // encode each byte array directly into raw VM storage
    for (index, value) in values.iter().enumerate() {
        let value = VmArray::from_bytes(&mut context, unsafe { value.as_slice()? })?;
        let value = value.encode_with_context(&mut context)?;
        context
            .write_heap_value(data, index, value)
            .map_err(Box::<RuntimeError>::from)?;
    }

    Ok(VmArray {
        data: vm::Value::heap_reference(data),
        len: len_u32,
        capacity: len_u32,
        _marker: std::marker::PhantomData,
    })
}

/// Encode one native value slice as one VM value slice.
pub(crate) fn values_to_vm<T: Copy + VmAggregateCodec + VmCollectionElement>(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeSlice<T>,
) -> RuntimeResult<VmSlice<T>> {
    let value = unsafe { value.as_slice()? };
    let mut context = context.write();

    VmSlice::from_values(&mut context, value)
}

/// Encode one native value array as one VM value array.
pub(crate) fn values_array_to_vm<T: Copy + VmAggregateCodec + VmCollectionElement>(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeArray<T>,
) -> RuntimeResult<VmArray<T>> {
    let value = unsafe { value.as_slice()? };
    let mut context = context.write();

    VmArray::from_values(&mut context, value)
}

/// Map one native value slice into one VM aggregate slice.
pub(crate) fn map_native_slice_to_vm<T, U>(
    context: &mut vm::ExternalCallContext<'_>,
    values: NativeSlice<T>,
    mut map: impl FnMut(&mut vm::ExternalCallContext<'_>, &T) -> RuntimeResult<U>,
) -> RuntimeResult<VmSlice<U>>
where
    U: Copy + VmAggregateCodec + VmCollectionElement,
{
    let values = unsafe { values.as_slice()? };
    let len = values.len();
    let mut builder = {
        let mut write = context.write();
        VmSlice::<U>::builder(&mut write, len)?
    };

    // map each element directly into final VM storage
    for value in values {
        let value = map(context, value)?;
        let mut write = context.write();
        builder.push(&mut write, value)?;
    }
    builder.finish()
}

/// Map one native value array into one VM aggregate array.
pub(crate) fn map_native_array_to_vm<T, U>(
    context: &mut vm::ExternalCallContext<'_>,
    values: NativeArray<T>,
    map: impl FnMut(&mut vm::ExternalCallContext<'_>, &T) -> RuntimeResult<U>,
) -> RuntimeResult<VmArray<U>>
where
    U: Copy + VmAggregateCodec + VmCollectionElement,
{
    let values = unsafe { values.as_slice()? };
    let len_u32 = u32::try_from(values.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "array",
            "array length exceeds u32",
        ))
        .boxed()
    })?;
    let slice = map_native_slice_to_vm(
        context,
        NativeSlice {
            data: values.as_ptr() as *mut T,
            len: len_u32,
        },
        map,
    )?;

    Ok(VmArray {
        data: slice.data,
        len: slice.len,
        capacity: slice.len,
        _marker: std::marker::PhantomData,
    })
}

/// Encode one native string array as one VM string-handle array.
pub(crate) fn string_array_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    values: NativeArray<NativeStringRef>,
) -> RuntimeResult<VmArray<vm::StringHandle>> {
    let values = unsafe { values.as_slice()? };
    let mut context = context.write();
    let len = values.len();
    let len_u32 = u32::try_from(len).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "array",
            "array length exceeds u32",
        ))
        .boxed()
    })?;
    let data = context
        .allocate_heap_value_slots(len)
        .map_err(Box::<RuntimeError>::from)?;

    // encode each string directly into raw VM storage
    for (index, value) in values.iter().enumerate() {
        let handle = context
            .string_handle(unsafe { value.as_str()? })
            .map_err(Box::<RuntimeError>::from)?;
        let handle = handle.value();
        context
            .write_heap_value(data, index, handle)
            .map_err(Box::<RuntimeError>::from)?;
    }

    Ok(VmArray {
        data: vm::Value::heap_reference(data),
        len: len_u32,
        capacity: len_u32,
        _marker: std::marker::PhantomData,
    })
}

/// Encode one native string slice as one VM string-handle slice.
pub(crate) fn string_slice_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    values: NativeStringSlice,
) -> RuntimeResult<VmSlice<vm::StringHandle>> {
    let values = unsafe { values.as_slice()? };
    let mut context = context.write();
    let len = values.len();
    let len_u32 = u32::try_from(len).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "slice",
            "slice length exceeds u32",
        ))
        .boxed()
    })?;
    let data = context
        .allocate_heap_value_slots(len)
        .map_err(Box::<RuntimeError>::from)?;

    // encode each string directly into raw VM storage
    for (index, value) in values.iter().enumerate() {
        let handle = context
            .string_handle(unsafe { value.as_str()? })
            .map_err(Box::<RuntimeError>::from)?;
        let handle = handle.value();
        context
            .write_heap_value(data, index, handle)
            .map_err(Box::<RuntimeError>::from)?;
    }

    Ok(VmSlice {
        data: vm::Value::heap_reference(data),
        len: len_u32,
        _marker: std::marker::PhantomData,
    })
}

/// Write one native byte slice into one mutable VM byte slice.
pub(crate) fn write_bytes_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    output: VmSlice<u8>,
    value: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let value = unsafe { value.as_slice()? };
    let mut context = context.write();

    output.write_bytes(&mut context, value)
}

/// Read one VM path-bytes payload and store it in binding-local path storage.
pub(crate) fn store_path_bytes_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: PathBytesVm,
) -> RuntimeResult<PathBytes> {
    let context = context.read();
    let bytes = path.0.bytes(&context)?;

    Ok(PathBytesAbi::<NativeAbi>(
        binding.store_array_copy(bytes.as_ref()),
    ))
}

/// Read one VM path-utf16 payload and store it in binding-local path storage.
pub(crate) fn store_path_utf16_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: PathUtf16Vm,
) -> RuntimeResult<PathUtf16> {
    let context = context.read();
    let units = binding.store_array_with(path.0.len as usize, |builder| {
        path.0.try_for_each_value(&context, |unit| {
            builder.push(unit);
            Ok(())
        })
    })?;

    Ok(PathUtf16Abi::<NativeAbi>(units))
}

/// Read one VM path payload and store it in binding-local path storage.
pub(crate) fn store_os_path_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<OsPath> {
    match path {
        OsPathVm::OsPathBytes(path_bytes) => {
            let bytes = store_path_bytes_from_vm(binding, context, path_bytes.bytes)?;

            Ok(core_fs::path_ref_from_bytes(bytes))
        }
        OsPathVm::OsPathUtf16(path_utf16) => {
            let utf16 = store_path_utf16_from_vm(binding, context, path_utf16.utf16)?;

            Ok(core_fs::path_ref_from_utf16(utf16))
        }
    }
}

/// Encode one native path-bytes payload as one VM value.
pub(crate) fn path_bytes_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    path: PathBytes,
) -> RuntimeResult<PathBytesVm> {
    let bytes = unsafe { path.0.as_slice()? };
    let mut context = context.write();
    let array = VmArray::from_bytes(&mut context, bytes)?;

    Ok(PathBytesAbi::<VmAbi>(array))
}

/// Encode one native path-utf16 payload as one VM value.
pub(crate) fn path_utf16_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    path: PathUtf16,
) -> RuntimeResult<PathUtf16Vm> {
    let units = unsafe { path.0.as_slice()? };
    let mut context = context.write();
    let array = VmArray::from_values(&mut context, units)?;

    Ok(PathUtf16Abi::<VmAbi>(array))
}

/// Encode one native path payload as one VM value.
pub(crate) fn os_path_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPath,
) -> RuntimeResult<OsPathVm> {
    match path {
        OsPath::OsPathBytes(path_bytes) => {
            let bytes = path_bytes_to_vm(context, path_bytes.bytes)?;
            let mut write = context.write();

            Ok(OsPathVm::OsPathBytes(OsPathBytesVm {
                kind: write
                    .string_handle("bytes")
                    .map_err(Box::<RuntimeError>::from)?,
                bytes,
            }))
        }
        OsPath::OsPathUtf16(path_utf16) => {
            let utf16 = path_utf16_to_vm(context, path_utf16.utf16)?;
            let mut write = context.write();

            Ok(OsPathVm::OsPathUtf16(OsPathUtf16Vm {
                kind: write
                    .string_handle("utf16")
                    .map_err(Box::<RuntimeError>::from)?,
                utf16,
            }))
        }
    }
}

/// Allocate one native read buffer for one VM slice.
pub(crate) fn allocate_vm_read_buffer(
    binding: &BindingCallContext,
    buffer: VmSlice<u8>,
) -> NativeSlice<u8> {
    binding.store_zeroed_byte_slice(buffer.len as usize)
}

/// Copy one native read buffer back into one VM slice.
pub(crate) fn write_vm_read_buffer(
    context: &mut vm::ExternalCallContext<'_>,
    buffer: VmSlice<u8>,
    native: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let bytes = unsafe { native.as_slice()? };
    let mut context = context.write();

    buffer.write_bytes(&mut context, bytes)
}

/// Decode one VM slice of byte slices into plain VM slice values.
pub(crate) fn decode_vm_byte_slices(
    context: &mut vm::ExternalCallContext<'_>,
    buffers: VmSlice<VmSlice<u8>>,
    field: &'static str,
) -> RuntimeResult<Vec<VmSlice<u8>>> {
    let context = context.read();
    let values = buffers.values(&context)?;
    let mut decoded = Vec::with_capacity(values.len());

    // decode each slice handle
    for value in values {
        decoded.push(VmSlice::from_value(&context, value, field, "Slice<uint8>")?);
    }

    Ok(decoded)
}

/// Allocate native read buffers for one VM slice of byte slices.
#[allow(clippy::type_complexity)]
pub(crate) fn allocate_vm_read_buffers(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    buffers: VmSlice<VmSlice<u8>>,
    field: &'static str,
) -> RuntimeResult<(NativeSlice<NativeSlice<u8>>, Vec<VmSlice<u8>>)> {
    let vm_buffers = decode_vm_byte_slices(context, buffers, field)?;
    let native_buffers = binding.store_slice_with(vm_buffers.len(), |builder| {
        // allocate matching native buffers
        for buffer in &vm_buffers {
            builder.push(binding.store_zeroed_byte_slice(buffer.len as usize));
        }

        Ok(())
    })?;

    Ok((native_buffers, vm_buffers))
}

/// Copy native read buffers back into one VM slice of byte slices.
pub(crate) fn write_vm_read_buffers(
    context: &mut vm::ExternalCallContext<'_>,
    vm_buffers: Vec<VmSlice<u8>>,
    native_buffers: NativeSlice<NativeSlice<u8>>,
    field: &'static str,
) -> RuntimeResult<()> {
    let native_buffers = unsafe { native_buffers.as_slice()? };

    // shape must match
    if native_buffers.len() != vm_buffers.len() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "buffer length mismatch",
        ))
        .boxed());
    }

    // copy each native buffer back into VM
    for (vm_buffer, native_buffer) in vm_buffers.into_iter().zip(native_buffers.iter()) {
        let bytes = unsafe { native_buffer.as_slice()? };
        let mut context = context.write();
        vm_buffer.write_bytes(&mut context, bytes)?;
    }

    Ok(())
}

/// Read one VM slice of byte slices and store it in binding-local slice storage.
pub(crate) fn store_vm_byte_slices(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    buffers: VmSlice<VmSlice<u8>>,
    field: &'static str,
) -> RuntimeResult<NativeSlice<NativeSlice<u8>>> {
    let vm_buffers = decode_vm_byte_slices(context, buffers, field)?;
    let context = context.read();

    binding.store_slice_with(vm_buffers.len(), |builder| {
        // copy each VM buffer into binding-owned storage
        for buffer in vm_buffers {
            let bytes = buffer.read_bytes(&context)?;
            builder.push(binding.store_slice(bytes));
        }

        Ok(())
    })
}
