use std::mem::MaybeUninit;

use super::{NativeAbiCodec, VmAbiCodec};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeAbi, VmAbi};
use crate::platform::fs::{
    OsPath, OsPathBytesVm, OsPathUtf16Vm, OsPathVm, PathBytes, PathBytesAbi, PathBytesVm,
    PathUtf16, PathUtf16Abi, PathUtf16Vm, core as core_fs,
};
use crate::platform::{
    NativeArray, PlatformError, VmAggregateCodec, VmArray, VmCollectionElement, VmSlice,
};
use crate::runtime::{BindingCallContext, NativeSlice, NativeStringRef, NativeStringSlice};
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
    context: &'call vm::ExternalCallContext<'vm>,
}

impl<'call, 'vm> VmDecodeCodec<'call, 'vm> {
    /// Create one VM decode codec for one call context.
    pub(crate) fn new(context: &'call vm::ExternalCallContext<'vm>) -> Self {
        Self { context }
    }

    /// Decode one VM binding value into one materialized Rust value.
    pub(crate) fn decode<T: VmAbiCodec>(&self, value: T) -> RuntimeResult<T::Value> {
        value.into_value(self.context)
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
        T::from_value(self.context, value)
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
    let values = values.read_values(context)?;
    let mut native_values = Vec::with_capacity(values.len());

    // decode each string
    for value in values {
        native_values.push(store_string_from_vm(binding, context, value)?);
    }

    Ok(binding.store_string_slice(native_values))
}

/// Read one VM byte slice and store it in binding-local slice storage.
pub(crate) fn store_bytes_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: VmSlice<u8>,
) -> RuntimeResult<NativeSlice<u8>> {
    let value = value.read_bytes(context)?;

    Ok(binding.store_slice(value))
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
    let value = value.read_values(context)?;

    Ok(binding.store_slice(value))
}

/// Read one VM byte array and store it in binding-local array storage.
pub(crate) fn store_bytes_array_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: VmArray<u8>,
) -> RuntimeResult<NativeArray<u8>> {
    let value = value.read_bytes(context)?;

    Ok(binding.store_array(value))
}

/// Read one VM value array and store it in binding-local array storage.
pub(crate) fn store_values_array_from_vm<
    T: Copy + VmAggregateCodec + VmCollectionElement + 'static,
>(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: VmArray<T>,
) -> RuntimeResult<NativeArray<T>> {
    let value = value.read_values(context)?;

    Ok(binding.store_array(value))
}

/// Encode one native byte slice as one VM byte slice.
pub(crate) fn bytes_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    let value = unsafe { value.as_slice()? };

    VmSlice::from_bytes(context, value)
}

/// Encode one native byte array as one VM byte array.
pub(crate) fn bytes_array_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeArray<u8>,
) -> RuntimeResult<VmArray<u8>> {
    let value = unsafe { value.as_slice()? };

    VmArray::from_bytes(context, value)
}

/// Encode one native byte-array array as one VM byte-array array.
pub(crate) fn bytes_array_array_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    values: NativeArray<NativeArray<u8>>,
) -> RuntimeResult<VmArray<VmArray<u8>>> {
    let values = unsafe { values.as_slice()? };
    let mut encoded = Vec::with_capacity(values.len());

    // encode each byte array
    for value in values {
        encoded.push(bytes_array_to_vm(context, *value)?);
    }

    VmArray::from_values(context, &encoded)
}

/// Encode one native value slice as one VM value slice.
pub(crate) fn values_to_vm<T: Copy + VmAggregateCodec + VmCollectionElement>(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeSlice<T>,
) -> RuntimeResult<VmSlice<T>> {
    let value = unsafe { value.as_slice()? };

    VmSlice::from_values(context, value)
}

/// Encode one native value array as one VM value array.
pub(crate) fn values_array_to_vm<T: Copy + VmAggregateCodec + VmCollectionElement>(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeArray<T>,
) -> RuntimeResult<VmArray<T>> {
    let value = unsafe { value.as_slice()? };

    VmArray::from_values(context, value)
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
    let mut encoded = Vec::with_capacity(values.len());

    // encode each element
    for value in values {
        encoded.push(map(context, value)?);
    }

    VmSlice::from_values(context, &encoded)
}

/// Map one native value array into one VM aggregate array.
pub(crate) fn map_native_array_to_vm<T, U>(
    context: &mut vm::ExternalCallContext<'_>,
    values: NativeArray<T>,
    mut map: impl FnMut(&mut vm::ExternalCallContext<'_>, &T) -> RuntimeResult<U>,
) -> RuntimeResult<VmArray<U>>
where
    U: Copy + VmAggregateCodec + VmCollectionElement,
{
    let values = unsafe { values.as_slice()? };
    let mut encoded = Vec::with_capacity(values.len());

    // encode each element
    for value in values {
        encoded.push(map(context, value)?);
    }

    VmArray::from_values(context, &encoded)
}

/// Encode one native string array as one VM string-handle array.
pub(crate) fn string_array_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    values: NativeArray<NativeStringRef>,
) -> RuntimeResult<VmArray<vm::StringHandle>> {
    let values = unsafe { values.as_slice()? };
    let mut handles = Vec::with_capacity(values.len());

    // encode each string
    for value in values {
        handles.push(intern_string_to_vm(context, *value)?);
    }

    VmArray::from_values(context, &handles)
}

/// Encode one native string slice as one VM string-handle slice.
pub(crate) fn string_slice_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    values: NativeStringSlice,
) -> RuntimeResult<VmSlice<vm::StringHandle>> {
    let values = unsafe { values.as_slice()? };
    let mut handles = Vec::with_capacity(values.len());

    // encode each string
    for value in values {
        handles.push(intern_string_to_vm(context, *value)?);
    }

    VmSlice::from_values(context, &handles)
}

/// Write one native byte slice into one mutable VM byte slice.
pub(crate) fn write_bytes_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    output: VmSlice<u8>,
    value: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let value = unsafe { value.as_slice()? };

    output.write_bytes(context, value)
}

/// Read one VM path-bytes payload and store it in binding-local path storage.
pub(crate) fn store_path_bytes_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: PathBytesVm,
) -> RuntimeResult<PathBytes> {
    let bytes = path.0.read_bytes(context)?;

    Ok(PathBytesAbi::<NativeAbi>(binding.store_array(bytes)))
}

/// Read one VM path-utf16 payload and store it in binding-local path storage.
pub(crate) fn store_path_utf16_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: PathUtf16Vm,
) -> RuntimeResult<PathUtf16> {
    let units = path.0.read_values(context)?;

    Ok(PathUtf16Abi::<NativeAbi>(binding.store_array(units)))
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
    let array = VmArray::from_bytes(context, bytes)?;

    Ok(PathBytesAbi::<VmAbi>(array))
}

/// Encode one native path-utf16 payload as one VM value.
pub(crate) fn path_utf16_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    path: PathUtf16,
) -> RuntimeResult<PathUtf16Vm> {
    let units = unsafe { path.0.as_slice()? };
    let array = VmArray::from_values(context, units)?;

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

            Ok(OsPathVm::OsPathBytes(OsPathBytesVm {
                kind: context
                    .string_handle("bytes")
                    .map_err(Box::<RuntimeError>::from)?,
                bytes,
            }))
        }
        OsPath::OsPathUtf16(path_utf16) => {
            let utf16 = path_utf16_to_vm(context, path_utf16.utf16)?;

            Ok(OsPathVm::OsPathUtf16(OsPathUtf16Vm {
                kind: context
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
    let length = buffer.len as usize;

    binding.store_slice(vec![0u8; length])
}

/// Copy one native read buffer back into one VM slice.
pub(crate) fn write_vm_read_buffer(
    context: &mut vm::ExternalCallContext<'_>,
    buffer: VmSlice<u8>,
    native: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let bytes = unsafe { native.as_slice()? };

    buffer.write_bytes(context, bytes)
}

/// Decode one VM slice of byte slices into plain VM slice values.
pub(crate) fn decode_vm_byte_slices(
    context: &mut vm::ExternalCallContext<'_>,
    buffers: VmSlice<VmSlice<u8>>,
    field: &'static str,
) -> RuntimeResult<Vec<VmSlice<u8>>> {
    let values = buffers.raw_values(context)?;
    let mut decoded = Vec::with_capacity(values.len());

    // decode each slice handle
    for value in values {
        decoded.push(VmSlice::from_value(context, value, field, "Slice<uint8>")?);
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
    let mut native_buffers = Vec::with_capacity(vm_buffers.len());

    // allocate matching native buffers
    for buffer in &vm_buffers {
        let length = buffer.len as usize;
        native_buffers.push(binding.store_slice(vec![0u8; length]));
    }

    Ok((binding.store_slice(native_buffers), vm_buffers))
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
        vm_buffer.write_bytes(context, bytes)?;
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
    let mut native_buffers = Vec::with_capacity(vm_buffers.len());

    // copy each VM buffer into binding-owned storage
    for buffer in vm_buffers {
        let bytes = buffer.read_bytes(context)?;
        native_buffers.push(binding.store_slice(bytes));
    }

    Ok(binding.store_slice(native_buffers))
}
