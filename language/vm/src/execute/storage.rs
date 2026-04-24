use crate::diagnostic::Error;
use crate::module::{TypedAccess, repr_type};
use crate::{Value, ValueTag};
use destack_heap::{HeapReference, Payload, RawPointer};
use destack_mir as mir;

use super::access::{decode_pointer_bits, invalid_pointer_type};
use super::{frame_pointer_value, global_pointer_value, stack_pointer_value};
use crate::interpreter::StepState;

/// Align one byte offset up to the requested alignment.
#[inline(always)]
fn align_offset(offset: usize, alignment: usize) -> usize {
    if alignment <= 1 {
        return offset;
    }

    let remainder = offset % alignment;
    if remainder == 0 {
        offset
    } else {
        offset + (alignment - remainder)
    }
}

/// Read one exact raw byte range into owned storage.
fn read_raw_bytes(
    state: &StepState<'_, '_>,
    pointer: RawPointer,
    start: usize,
    byte_len: usize,
) -> Result<Vec<u8>, Error> {
    // validate the requested range against the live allocation
    let heap = state.heap();
    let available_len = heap.raw_byte_len(pointer)?;
    let end = start
        .checked_add(byte_len)
        .ok_or(Error::InvalidHeapReference)?;
    if end > available_len {
        return Err(Error::InvalidHeapReference);
    }

    // fill one exact owned range from the heap
    let mut bytes = vec![0u8; byte_len];
    heap.read_raw_bytes_into(pointer, start, &mut bytes)
        .map_err(Error::from)?;

    Ok(bytes)
}

/// Return the callable box payload layout.
fn callable_payload_layout(tree: &mir::NodeTree) -> (usize, usize, usize) {
    let pointer_bytes = tree.pointer_bytes() as usize;
    let function_offset = 0usize;
    let environment_offset = align_offset(pointer_bytes, pointer_bytes);
    let byte_len = environment_offset + pointer_bytes;

    (function_offset, environment_offset, byte_len)
}

/// Return the environment type for one bound function.
fn callable_environment_type(
    tree: &mir::NodeTree,
    function_id: mir::LocalNodeId<mir::Function>,
) -> Result<mir::LocalNodeId<mir::Type>, Error> {
    let function = tree.get(function_id);
    let environment = function.environment.ok_or(Error::InvalidInstruction)?;

    environment.ty().ok_or_else(|| Error::ConcreteMirRequired {
        context: "callable environment type".to_string(),
    })
}

/// Decode one callable object into function and environment values.
fn decode_callable_payload(
    state: &mut StepState<'_, '_>,
    handle: HeapReference,
) -> Result<(Value, Value), Error> {
    let (function_offset, environment_offset, byte_len) = callable_payload_layout(state.tree());
    let bytes = state
        .read_heap_bytes(handle, 0, byte_len)
        .map_err(Error::from)?;

    if bytes.len() != byte_len {
        return Err(Error::InvalidHeapReference);
    }

    let function_byte_len = state.tree().pointer_bytes() as usize;
    let function_end = function_offset
        .checked_add(function_byte_len)
        .ok_or(Error::InvalidHeapReference)?;
    let environment_end = environment_offset
        .checked_add(function_byte_len)
        .ok_or(Error::InvalidHeapReference)?;
    let function_bytes = bytes
        .get(function_offset..function_end)
        .ok_or(Error::InvalidHeapReference)?;
    let environment_bytes = bytes
        .get(environment_offset..environment_end)
        .ok_or(Error::InvalidHeapReference)?;

    let mut function_raw = [0u8; 8];
    function_raw[..function_bytes.len()].copy_from_slice(function_bytes);
    let function_id = mir::LocalNodeId::new(u64::from_le_bytes(function_raw) as u32);
    let function = Value::function_pointer(function_id);

    let mut environment_raw = [0u8; 8];
    environment_raw[..environment_bytes.len()].copy_from_slice(environment_bytes);
    let environment_type = callable_environment_type(state.tree(), function_id)?;
    let environment_value = if state.layout(environment_type)?.is_scalar() {
        decode_raw_value(state.tree(), environment_type, environment_bytes)?
    } else {
        let environment_handle =
            HeapReference::from_bits(u64::from_le_bytes(environment_raw) as usize);

        Value::heap_reference(environment_handle)
    };

    Ok((function, environment_value))
}

/// Encode one typed value into one owned byte buffer.
pub(crate) fn encode_value_bytes(
    state: &mut StepState<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    value: Value,
) -> Result<Vec<u8>, Error> {
    let layout = state.layout(ty)?.clone();

    // scalars
    if layout.is_scalar() {
        return encode_storage_value(state, ty, value);
    }

    let mut bytes = vec![0u8; layout.byte_len];
    write_storage_value_into(state, ty, value, &mut bytes)?;

    Ok(bytes)
}

/// Encode one callable environment slot.
fn encode_callable_environment(
    state: &mut StepState<'_, '_>,
    function_id: mir::LocalNodeId<mir::Function>,
    environment_value: Value,
) -> Result<Vec<u8>, Error> {
    let environment_type = callable_environment_type(state.tree(), function_id)?;
    if state.layout(environment_type)?.is_scalar() {
        return encode_storage_value(state, environment_type, environment_value);
    }

    let environment_layout_id = state
        .module
        .layout_id_for_type(environment_type)
        .ok_or(Error::InvalidInstruction)?;
    let environment_bytes = encode_value_bytes(state, environment_type, environment_value)?;
    let environment_handle = state
        .heap_mut()
        .allocate(environment_layout_id, Payload::Bytes(&environment_bytes))
        .map_err(Error::from)?;

    Ok(
        (environment_handle.bits() as u64).to_le_bytes()[..state.tree().pointer_bytes() as usize]
            .to_vec(),
    )
}

/// Allocate one callable object from function and environment values.
pub(crate) fn allocate_callable(
    state: &mut StepState<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    function: Value,
    environment_value: Value,
) -> Result<Value, Error> {
    let (function_offset, environment_offset, byte_len) = callable_payload_layout(state.tree());
    let function_id = function
        .as_function_pointer()
        .ok_or_else(|| Error::TypeMismatch {
            expected: "function pointer".to_string(),
            actual: format!("{function:?}"),
        })?;
    let function_bytes = (function_id.id as u64).to_le_bytes();
    let environment_bytes = encode_callable_environment(state, function_id, environment_value)?;
    let mut bytes = vec![0; byte_len];

    let function_end = function_offset
        .checked_add(state.tree().pointer_bytes() as usize)
        .ok_or(Error::InvalidHeapReference)?;
    let environment_end = environment_offset
        .checked_add(state.tree().pointer_bytes() as usize)
        .ok_or(Error::InvalidHeapReference)?;
    if function_end > bytes.len() || environment_end > bytes.len() {
        return Err(Error::InvalidHeapReference);
    }

    bytes[function_offset..function_end]
        .copy_from_slice(&function_bytes[..state.tree().pointer_bytes() as usize]);
    bytes[environment_offset..environment_end].copy_from_slice(&environment_bytes);

    let layout_id = state
        .module
        .layout_id_for_type(ty)
        .ok_or(Error::InvalidInstruction)?;
    let handle = state
        .heap_mut()
        .allocate(layout_id, Payload::Bytes(&bytes))
        .map_err(Error::from)?;

    Ok(Value::heap_reference(handle))
}

/// Validate a field index against a known field count.
#[inline(always)]
pub(super) fn check_field_index(
    state: &StepState<'_, '_>,
    index: u32,
    field_count: Option<u32>,
) -> Result<(), Error> {
    // skip checks when bounds are disabled
    if !state.bounds_checks {
        return Ok(());
    }

    // skip checks when the field count is unknown
    let Some(field_count) = field_count else {
        return Ok(());
    };

    // reject out of bounds indices
    if index >= field_count {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: field_count as usize,
        });
    }

    Ok(())
}

/// Validate an array index against a known length.
#[inline(always)]
pub(super) fn check_array_index(
    state: &StepState<'_, '_>,
    index: u64,
    array_length: Option<u64>,
) -> Result<(), Error> {
    // skip checks when bounds are disabled
    if !state.bounds_checks {
        return Ok(());
    }

    // skip checks when the length is unknown
    let Some(array_length) = array_length else {
        return Ok(());
    };

    // reject out of bounds indices
    if index >= array_length {
        return Err(Error::InvalidArrayAccess {
            index,
            length: array_length,
        });
    }

    Ok(())
}

/// Resolve the byte size for one raw pointee type.
pub(crate) fn raw_type_size(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<usize, Error> {
    let ty = repr_type(tree, ty);
    let size = match tree.get(ty) {
        mir::Type::Void => 0,
        mir::Type::Boolean => 1,
        mir::Type::Int { width, .. } => (*width as usize).div_ceil(8),
        mir::Type::Isize
        | mir::Type::Usize
        | mir::Type::TypeDescriptor
        | mir::Type::TypeId
        | mir::Type::Reference { .. }
        | mir::Type::Callable { .. }
        | mir::Type::FunctionPointer { .. }
        | mir::Type::FunctionSignature { .. }
        | mir::Type::TensorView { .. } => tree.pointer_bytes() as usize,
        mir::Type::Float { width } => (*width as usize).div_ceil(8),
        mir::Type::Newtype { .. } => {
            return Err(Error::TypeMismatch {
                expected: "runtime representation type".to_string(),
                actual: format!("{ty:?}"),
            });
        }
        mir::Type::Array { .. }
        | mir::Type::Slice { .. }
        | mir::Type::Tuple { .. }
        | mir::Type::Struct { .. }
        | mir::Type::Vector { .. }
        | mir::Type::Tensor { .. } => tree
            .type_layout(ty)
            .map(|layout| layout.size as usize)
            .ok_or_else(|| Error::TypeMismatch {
                expected: "layout-backed raw type".to_string(),
                actual: format!("{ty:?}"),
            })?,
    };

    Ok(size)
}

/// Write one raw byte range from the given buffer.
fn write_raw_bytes(
    state: &mut StepState<'_, '_>,
    pointer: RawPointer,
    byte_offset: usize,
    bytes: &[u8],
) -> Result<(), Error> {
    let byte_len = state.heap().raw_byte_len(pointer)?;
    let end = byte_offset
        .checked_add(bytes.len())
        .ok_or(Error::InvalidFieldAccess {
            index: byte_offset as u32,
            field_count: byte_len,
        })?;

    if end > byte_len {
        return Err(Error::InvalidFieldAccess {
            index: byte_offset as u32,
            field_count: byte_len,
        });
    }

    state
        .heap_mut()
        .set_raw_bytes(pointer, byte_offset, bytes)
        .map_err(Error::from)?;

    Ok(())
}

/// Decode one raw byte range into a VM value.
pub(crate) fn decode_raw_value(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
    bytes: &[u8],
) -> Result<Value, Error> {
    let byte_len = raw_type_size(tree, ty)?;
    if bytes.len() != byte_len {
        return Err(Error::TypeMismatch {
            expected: format!("{byte_len} raw bytes"),
            actual: format!("{} raw bytes", bytes.len()),
        });
    }

    let ty = repr_type(tree, ty);

    match tree.get(ty) {
        mir::Type::Void => Ok(Value::VOID),
        mir::Type::Boolean => Ok(Value::bool(bytes[0] != 0)),
        mir::Type::Int { width, is_signed } => {
            let mut raw = [0u8; 8];
            raw[..bytes.len()].copy_from_slice(bytes);
            let raw = u64::from_le_bytes(raw);
            if *is_signed {
                Ok(Value::int(raw as i64, *width as u8))
            } else {
                Ok(Value::uint(raw, *width as u8))
            }
        }
        mir::Type::Isize => {
            let mut raw = [0u8; 8];
            raw[..bytes.len()].copy_from_slice(bytes);
            Ok(Value::int(
                u64::from_le_bytes(raw) as i64,
                tree.pointer_bits() as u8,
            ))
        }
        mir::Type::Usize | mir::Type::TypeDescriptor | mir::Type::TypeId => {
            let mut raw = [0u8; 8];
            raw[..bytes.len()].copy_from_slice(bytes);
            Ok(Value::uint(
                u64::from_le_bytes(raw),
                tree.pointer_bits() as u8,
            ))
        }
        mir::Type::Float { width: 32 } => {
            let mut raw = [0u8; 4];
            raw[..bytes.len()].copy_from_slice(bytes);
            Ok(Value::float32(f32::from_bits(u32::from_le_bytes(raw))))
        }
        mir::Type::Float { width: 64 } => {
            let mut raw = [0u8; 8];
            raw.copy_from_slice(bytes);
            Ok(Value::float64(f64::from_bits(u64::from_le_bytes(raw))))
        }
        mir::Type::Float { width } => Err(Error::TypeMismatch {
            expected: "supported float width".to_string(),
            actual: width.to_string(),
        }),
        mir::Type::Reference { .. } => {
            let mut raw = [0u8; 8];
            raw[..bytes.len()].copy_from_slice(bytes);
            let raw = u64::from_le_bytes(raw);

            decode_pointer_bits(raw, tree.get(ty))
        }
        mir::Type::Callable { .. } => {
            let mut raw = [0u8; 8];
            raw[..bytes.len()].copy_from_slice(bytes);
            Ok(Value::heap_reference(HeapReference::from_bits(
                u64::from_le_bytes(raw) as usize,
            )))
        }
        mir::Type::FunctionSignature { .. } => {
            let mut raw = [0u8; 8];
            raw[..bytes.len()].copy_from_slice(bytes);
            Ok(Value::function_pointer(mir::LocalNodeId::new(
                u64::from_le_bytes(raw) as u32,
            )))
        }
        mir::Type::FunctionPointer { .. } => {
            let mut raw = [0u8; 8];
            raw[..bytes.len()].copy_from_slice(bytes);
            decode_pointer_bits(u64::from_le_bytes(raw), tree.get(ty))
        }
        mir::Type::Newtype { .. } => Err(Error::TypeMismatch {
            expected: "runtime representation type".to_string(),
            actual: format!("{ty:?}"),
        }),
        _ => Err(Error::TypeMismatch {
            expected: "scalar or reference raw load".to_string(),
            actual: format!("{ty:?}"),
        }),
    }
}

/// Allocate one heap object from indexed aggregate values.
pub(crate) fn allocate_heap_value_by_index<F>(
    state: &mut StepState<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    mut value_at_index: F,
) -> Result<Value, Error>
where
    F: FnMut(&mut StepState<'_, '_>, u32, mir::LocalNodeId<mir::Type>) -> Result<Value, Error>,
{
    let layout = state.layout(ty)?.clone();
    let value_specs = storage_index_layouts(state, ty)?;
    let mut bytes = vec![0u8; layout.byte_len];

    // encode each value into its physical byte range
    for (index, value_type, offset, byte_len) in value_specs {
        let value = value_at_index(state, index, value_type)?;
        let value_end = offset
            .checked_add(byte_len)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: layout.byte_len,
            })?;

        if value_end > layout.byte_len {
            return Err(Error::InvalidFieldAccess {
                index,
                field_count: layout.byte_len,
            });
        }

        let value_window = bytes
            .get_mut(offset..value_end)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: layout.byte_len,
            })?;

        write_storage_value_into(state, value_type, value, value_window)?;
    }

    let layout_id = state
        .module
        .layout_id_for_type(ty)
        .ok_or(Error::InvalidInstruction)?;
    let reference = state
        .heap_mut()
        .allocate(layout_id, Payload::Bytes(&bytes))
        .map_err(Error::from)?;

    Ok(Value::heap_reference(reference))
}

/// Return all indexed value byte ranges for one aggregate layout.
fn storage_index_layouts(
    state: &StepState<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<Vec<(u32, mir::LocalNodeId<mir::Type>, usize, usize)>, Error> {
    let layout = state.layout(ty)?;

    if let Some(field_count) = layout.field_count() {
        return (0..field_count)
            .map(|index| {
                let index = index as u32;
                let field = layout
                    .field(index)
                    .ok_or(Error::InvalidFieldAccess { index, field_count })?;

                Ok((index, field.ty, field.offset, field.byte_len))
            })
            .collect();
    }

    let Some(element) = layout.element() else {
        return Err(Error::TypeMismatch {
            expected: "aggregate type".to_string(),
            actual: format!("{ty:?}"),
        });
    };
    let element_count = layout.element_count().unwrap_or(0);

    (0..element_count)
        .map(|index| {
            let index = index as u32;
            let offset =
                element
                    .stride
                    .checked_mul(index as usize)
                    .ok_or(Error::InvalidArrayAccess {
                        index: index.into(),
                        length: element_count as u64,
                    })?;

            Ok((index, element.ty, offset, element.byte_len))
        })
        .collect()
}

/// Allocate one zeroed heap object.
pub(crate) fn allocate_zeroed_heap_value(
    state: &mut StepState<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<Value, Error> {
    let layout_id = state
        .module
        .layout_id_for_type(ty)
        .ok_or(Error::InvalidInstruction)?;
    let reference = state
        .heap_mut()
        .allocate(layout_id, Payload::Zeroed)
        .map_err(Error::from)?;

    Ok(Value::heap_reference(reference))
}

/// Encode one VM value into raw bytes for the given type.
pub(crate) fn encode_raw_value(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
    value: Value,
) -> Result<Vec<u8>, Error> {
    let byte_len = raw_type_size(tree, ty)?;

    let bytes = match tree.get(ty) {
        mir::Type::Void => Vec::new(),
        mir::Type::Boolean => vec![u8::from(value.as_bool().ok_or_else(|| {
            Error::TypeMismatch {
                expected: "bool".to_string(),
                actual: format!("{value:?}"),
            }
        })?)],
        mir::Type::Int { .. } | mir::Type::Isize => {
            let raw = match value.tag() {
                ValueTag::Int => value.raw_data(),
                ValueTag::UInt => value.raw_data(),
                _ => {
                    return Err(Error::TypeMismatch {
                        expected: "integer".to_string(),
                        actual: format!("{value:?}"),
                    });
                }
            };
            raw.to_le_bytes()[..byte_len].to_vec()
        }
        mir::Type::Usize | mir::Type::TypeDescriptor | mir::Type::TypeId => {
            let raw = value.as_uint().ok_or_else(|| Error::TypeMismatch {
                expected: "integer".to_string(),
                actual: format!("{value:?}"),
            })?;
            raw.to_le_bytes()[..byte_len].to_vec()
        }
        mir::Type::Float { width: 32 } => {
            let raw = value.as_float32().ok_or_else(|| Error::TypeMismatch {
                expected: "float32".to_string(),
                actual: format!("{value:?}"),
            })?;
            raw.to_bits().to_le_bytes().to_vec()
        }
        mir::Type::Float { width: 64 } => {
            let raw = value.as_float64().ok_or_else(|| Error::TypeMismatch {
                expected: "float64".to_string(),
                actual: format!("{value:?}"),
            })?;
            raw.to_bits().to_le_bytes().to_vec()
        }
        mir::Type::Float { width } => {
            return Err(Error::TypeMismatch {
                expected: "supported float width".to_string(),
                actual: width.to_string(),
            });
        }
        mir::Type::Reference {
            kind,
            address_space,
            ..
        } => {
            let raw = match (*kind, address_space.clone()) {
                (
                    mir::ReferenceKind::Managed | mir::ReferenceKind::Owned,
                    mir::AddressSpace::Shared,
                ) => value
                    .as_shared_heap_reference()
                    .map(|reference| reference.bits() as u64)
                    .ok_or_else(|| Error::TypeMismatch {
                        expected: "shared heap reference".to_string(),
                        actual: format!("{value:?}"),
                    })?,
                (mir::ReferenceKind::Managed | mir::ReferenceKind::Owned, _) => value
                    .as_heap_reference()
                    .map(|reference| reference.bits() as u64)
                    .ok_or_else(|| Error::TypeMismatch {
                        expected: "heap reference".to_string(),
                        actual: format!("{value:?}"),
                    })?,
                (mir::ReferenceKind::Borrowed, mir::AddressSpace::Shared) => value
                    .as_shared_heap_reference()
                    .map(|reference| reference.bits() as u64)
                    .or_else(|| {
                        value
                            .as_shared_raw_pointer()
                            .map(|pointer| pointer.bits() as u64)
                    })
                    .ok_or_else(|| Error::TypeMismatch {
                        expected: "shared borrowed reference".to_string(),
                        actual: format!("{value:?}"),
                    })?,
                (_, mir::AddressSpace::Shared) => value
                    .as_shared_raw_pointer()
                    .map(|pointer| pointer.bits() as u64)
                    .ok_or_else(|| Error::TypeMismatch {
                        expected: "shared raw pointer".to_string(),
                        actual: format!("{value:?}"),
                    })?,
                (_, mir::AddressSpace::Stack) => {
                    let pointer = value
                        .as_stack_pointer()
                        .ok_or_else(|| Error::TypeMismatch {
                            expected: "stack pointer".to_string(),
                            actual: format!("{value:?}"),
                        })?;

                    stack_pointer_value(pointer)?.raw_data()
                }
                (_, mir::AddressSpace::Frame) => {
                    let pointer = value
                        .as_frame_pointer()
                        .ok_or_else(|| Error::TypeMismatch {
                            expected: "frame pointer".to_string(),
                            actual: format!("{value:?}"),
                        })?;

                    frame_pointer_value(pointer)?.raw_data()
                }
                (_, mir::AddressSpace::Static) => {
                    let pointer = value
                        .as_global_pointer()
                        .ok_or_else(|| Error::TypeMismatch {
                            expected: "global pointer".to_string(),
                            actual: format!("{value:?}"),
                        })?;

                    global_pointer_value(pointer.id, pointer.byte_offset)?.raw_data()
                }
                (mir::ReferenceKind::Borrowed, _) => value
                    .as_heap_reference()
                    .map(|reference| reference.bits() as u64)
                    .or_else(|| value.as_raw_pointer().map(|pointer| pointer.bits() as u64))
                    .ok_or_else(|| Error::TypeMismatch {
                        expected: "borrowed reference".to_string(),
                        actual: format!("{value:?}"),
                    })?,
                _ => value
                    .as_heap_reference()
                    .map(|handle| handle.bits() as u64)
                    .or_else(|| value.as_raw_pointer().map(|pointer| pointer.bits() as u64))
                    .ok_or_else(|| Error::TypeMismatch {
                        expected: "reference".to_string(),
                        actual: format!("{value:?}"),
                    })?,
            };
            raw.to_le_bytes()[..byte_len].to_vec()
        }
        mir::Type::Callable { .. } => {
            let raw = value
                .as_heap_reference()
                .map(|handle| handle.bits() as u64)
                .ok_or_else(|| Error::TypeMismatch {
                    expected: "boxed callable".to_string(),
                    actual: format!("{value:?}"),
                })?;
            raw.to_le_bytes()[..byte_len].to_vec()
        }
        mir::Type::FunctionSignature { .. } | mir::Type::FunctionPointer { .. } => {
            let raw = value
                .as_function_pointer()
                .ok_or_else(|| Error::TypeMismatch {
                    expected: "function pointer".to_string(),
                    actual: format!("{value:?}"),
                })?;
            (raw.id as u64).to_le_bytes()[..byte_len].to_vec()
        }
        mir::Type::Newtype { inner, .. } => {
            let inner = (*inner).ty().ok_or_else(|| Error::ConcreteMirRequired {
                context: "newtype inner".to_string(),
            })?;
            return encode_raw_value(tree, inner, value);
        }
        _ => {
            return Err(Error::TypeMismatch {
                expected: "scalar or reference raw store".to_string(),
                actual: format!("{ty:?}"),
            });
        }
    };

    Ok(bytes)
}

/// Encode one VM value into storage bytes for the given type.
pub(crate) fn encode_storage_value(
    state: &mut StepState<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    value: Value,
) -> Result<Vec<u8>, Error> {
    let layout = state.layout(ty)?;
    if !layout.is_scalar() {
        return Err(Error::TypeMismatch {
            expected: "scalar type".to_string(),
            actual: format!("{ty:?}"),
        });
    }

    encode_raw_value(state.tree(), ty, value)
}

/// Write one value into one destination byte range.
pub(crate) fn write_storage_value_into(
    state: &mut StepState<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    value: Value,
    destination: &mut [u8],
) -> Result<(), Error> {
    if state.layout(ty)?.is_scalar() {
        let bytes = encode_storage_value(state, ty, value)?;
        if bytes.len() != destination.len() {
            return Err(Error::InvalidHeapReference);
        }

        destination.copy_from_slice(&bytes);
        return Ok(());
    }

    if let Some(handle) = value.as_heap_reference() {
        let source_len = state.heap().heap_byte_len(handle)?;
        if source_len != destination.len() {
            return Err(Error::InvalidHeapReference);
        }

        state
            .heap()
            .read_heap_bytes_into(handle, 0, destination)
            .map_err(Error::from)?;

        return Ok(());
    }

    if let Some(handle) = value.as_shared_heap_reference() {
        let source_len = state.shared().heap_byte_len(handle).map_err(Error::from)?;
        if source_len != destination.len() {
            return Err(Error::InvalidHeapReference);
        }

        state
            .shared()
            .read_heap_bytes_into(handle, 0, destination)
            .map_err(Error::from)?;

        return Ok(());
    }

    if let Some(pointer) = value.as_stack_pointer() {
        if pointer.byte_offset != 0 {
            return Err(Error::InvalidHeapReference);
        }

        let frame = state.frame_by_index(pointer.frame_idx)?;
        let allocation = frame
            .stack_allocation(pointer.slot)
            .ok_or(Error::InvalidHeapReference)?;
        if allocation.len() != destination.len() {
            return Err(Error::InvalidHeapReference);
        }

        destination.copy_from_slice(allocation.bytes());

        return Ok(());
    }

    Err(Error::TypeMismatch {
        expected: "typed aggregate value".to_string(),
        actual: format!("{value:?}"),
    })
}

/// Decode one callable object into function and environment values.
pub(crate) fn decode_callable(
    state: &mut StepState<'_, '_>,
    value: Value,
) -> Result<(Value, Value), Error> {
    if let Some(handle) = value.as_heap_reference() {
        return decode_callable_payload(state, handle);
    }

    Err(Error::TypeMismatch {
        expected: "boxed callable".to_string(),
        actual: format!("{value:?}"),
    })
}

/// Load one value from raw heap bytes.
pub(crate) fn load_from_raw_pointer_typed(
    state: &mut StepState<'_, '_>,
    ptr: Value,
    access: TypedAccess,
) -> Result<Value, Error> {
    if ptr.tag() != ValueTag::RawPointer {
        return Err(Error::InvalidPointerType {
            actual: format!("{ptr:?}"),
        });
    }

    let pointer = ptr
        .as_raw_pointer()
        .ok_or_else(|| Error::InvalidPointerType {
            actual: format!("{ptr:?}"),
        })?;
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let byte_len = state.heap().raw_byte_len(pointer)?;

    // reject reads that extend past the raw payload
    if access.byte_len > byte_len {
        return Err(Error::InvalidFieldAccess {
            index: 0,
            field_count: byte_len,
        });
    }

    // read the exact typed bytes without materializing the whole raw payload
    let owned_bytes = read_raw_bytes(state, pointer, 0, access.byte_len)?;

    if !access.is_scalar {
        return Ok(ptr);
    }

    decode_raw_value(state.tree(), access.value_type, &owned_bytes)
}

/// Store one value into raw heap bytes.
pub(crate) fn store_to_raw_pointer_typed(
    state: &mut StepState<'_, '_>,
    ptr: Value,
    access: TypedAccess,
    value: Value,
) -> Result<(), Error> {
    if ptr.tag() != ValueTag::RawPointer {
        return Err(invalid_pointer_type(ptr));
    }

    let pointer = ptr
        .as_raw_pointer()
        .ok_or_else(|| invalid_pointer_type(ptr))?;
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let bytes = encode_value_bytes(state, access.value_type, value)?;

    write_raw_bytes(state, pointer, 0, &bytes)
}
