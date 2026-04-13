use crate::diagnostic::Error;
use crate::executable::{TypedAccess, UNKNOWN_ARRAY_LENGTH, UNKNOWN_FIELD_COUNT, repr_type};
use destack_heap::{ManagedReference, RawPointer, ReferenceMap, Value, ValueTag};
use destack_mir as mir;

use super::access::{decode_pointer_bits, invalid_pointer_type};
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

/// Return the callable box payload layout for one function value type.
fn function_value_payload_layout(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<(mir::LocalNodeId<mir::Type>, usize, usize, usize), Error> {
    let mir::Type::Closure { signature } = tree.get(ty) else {
        return Err(Error::TypeMismatch {
            expected: "function value type".to_string(),
            actual: format!("{ty:?}"),
        });
    };

    let signature = (*signature)
        .ty()
        .ok_or_else(|| Error::ConcreteMirRequired {
            context: "closure signature".to_string(),
        })?;
    let signature_size = raw_type_size(tree, signature)?;
    let signature_alignment = raw_type_alignment(tree, signature)?;
    let environment_size = Value::BYTE_LEN;
    let environment_alignment = std::mem::align_of::<Value>();

    let function_offset = 0usize;
    let environment_offset = align_offset(signature_size, environment_alignment);
    let alignment = signature_alignment.max(environment_alignment).max(1);
    let byte_len = align_offset(environment_offset + environment_size, alignment);

    Ok((signature, function_offset, environment_offset, byte_len))
}

/// Decode one boxed callable object into function and environment values.
fn decode_function_value_storage(
    state: &mut StepState<'_, '_>,
    handle: ManagedReference,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<(Value, Value), Error> {
    let (signature_type, function_offset, environment_offset, byte_len) =
        function_value_payload_layout(state.tree(), ty)?;
    let bytes = state
        .heap()
        .managed_bytes(handle)
        .ok_or(Error::InvalidManagedReference)?;

    if bytes.len() != byte_len {
        return Err(Error::InvalidManagedReference);
    }

    let function_byte_len = raw_type_size(state.tree(), signature_type)?;
    let function_end = function_offset
        .checked_add(function_byte_len)
        .ok_or(Error::InvalidManagedReference)?;
    let environment_end = environment_offset
        .checked_add(Value::BYTE_LEN)
        .ok_or(Error::InvalidManagedReference)?;
    let function_bytes = bytes
        .get(function_offset..function_end)
        .ok_or(Error::InvalidManagedReference)?;
    let environment_bytes = bytes
        .get(environment_offset..environment_end)
        .ok_or(Error::InvalidManagedReference)?;

    let function_value = decode_raw_value(state.tree(), signature_type, function_bytes)?;
    let environment_value =
        Value::from_byte_slice(environment_bytes).ok_or(Error::InvalidManagedReference)?;

    Ok((function_value, environment_value))
}

/// Allocate one boxed callable object from function and environment values.
pub(crate) fn allocate_function_value(
    state: &mut StepState<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    function_value: Value,
    environment_value: Value,
) -> Result<Value, Error> {
    let (signature_type, function_offset, environment_offset, byte_len) =
        function_value_payload_layout(state.tree(), ty)?;
    let function_bytes = encode_raw_value(state.tree(), signature_type, function_value)?;
    let environment_bytes = environment_value.to_byte_array();
    let mut bytes = vec![0; byte_len];

    let function_end = function_offset
        .checked_add(function_bytes.len())
        .ok_or(Error::InvalidManagedReference)?;
    let environment_end = environment_offset
        .checked_add(environment_bytes.len())
        .ok_or(Error::InvalidManagedReference)?;
    if function_end > bytes.len() || environment_end > bytes.len() {
        return Err(Error::InvalidManagedReference);
    }

    bytes[function_offset..function_end].copy_from_slice(&function_bytes);
    bytes[environment_offset..environment_end].copy_from_slice(&environment_bytes);

    let reference_map = ReferenceMap::ValueOffsets {
        offsets: vec![environment_offset as u32],
    };
    let layout_id = state.tree().type_layout_id(ty);
    let handle = state
        .heap_mut()
        .allocate_managed_bytes_typed(&bytes, reference_map, layout_id, ty.id)
        .map_err(Error::from)?;

    Ok(Value::managed_reference(handle))
}

/// Validate a field index against a known field count.
#[inline(always)]
pub(super) fn check_field_index(
    state: &StepState<'_, '_>,
    index: u32,
    field_count: u32,
) -> Result<(), Error> {
    // skip checks when bounds are disabled
    if !state.bounds_checks {
        return Ok(());
    }

    // skip checks when the field count is unknown
    if field_count == UNKNOWN_FIELD_COUNT {
        return Ok(());
    }

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
    array_length: u64,
) -> Result<(), Error> {
    // skip checks when bounds are disabled
    if !state.bounds_checks {
        return Ok(());
    }

    // skip checks when the length is unknown
    if array_length == UNKNOWN_ARRAY_LENGTH {
        return Ok(());
    }

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
        | mir::Type::Closure { .. }
        | mir::Type::FunctionPointer { .. }
        | mir::Type::TensorReference { .. } => tree.pointer_bytes() as usize,
        mir::Type::Float { width } => (*width as usize).div_ceil(8),
        mir::Type::Newtype { .. } => unreachable!("repr_type must peel newtypes"),
        mir::Type::Array { .. }
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

/// Resolve the byte alignment for one raw pointee type.
pub(crate) fn raw_type_alignment(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<usize, Error> {
    let ty = repr_type(tree, ty);

    let alignment = match tree.get(ty) {
        mir::Type::Void => 1,
        mir::Type::Boolean => 1,
        mir::Type::Int { width, .. } => (*width as usize).div_ceil(8).max(1),
        mir::Type::Isize
        | mir::Type::Usize
        | mir::Type::TypeDescriptor
        | mir::Type::TypeId
        | mir::Type::Reference { .. }
        | mir::Type::Closure { .. }
        | mir::Type::FunctionPointer { .. }
        | mir::Type::TensorReference { .. } => tree.pointer_bytes() as usize,
        mir::Type::Float { width } => (*width as usize).div_ceil(8).max(1),
        mir::Type::Newtype { .. } => unreachable!("repr_type must peel newtypes"),
        mir::Type::Array { .. }
        | mir::Type::Tuple { .. }
        | mir::Type::Struct { .. }
        | mir::Type::Vector { .. }
        | mir::Type::Tensor { .. } => tree
            .type_layout(ty)
            .map(|layout| layout.alignment as usize)
            .ok_or_else(|| Error::TypeMismatch {
                expected: "layout-backed raw type".to_string(),
                actual: format!("{ty:?}"),
            })?,
    };

    Ok(alignment)
}

/// Write one raw byte window from the given buffer.
fn write_raw_bytes(
    state: &mut StepState<'_, '_>,
    pointer: RawPointer,
    byte_offset: usize,
    bytes: &[u8],
) -> Result<(), Error> {
    let byte_len = state
        .heap()
        .raw_byte_len(pointer)
        .ok_or(Error::InvalidManagedReference)?;
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

    if !state.heap_mut().set_raw_bytes(pointer, byte_offset, bytes) {
        return Err(Error::InvalidFieldAccess {
            index: byte_offset as u32,
            field_count: byte_len,
        });
    }

    Ok(())
}

/// Decode one raw byte window into a VM value.
pub(crate) fn decode_raw_value(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
    bytes: &[u8],
) -> Result<Value, Error> {
    match tree.get(ty) {
        mir::Type::Void => Ok(Value::VOID),
        mir::Type::Boolean => Ok(Value::bool(bytes.first().copied().unwrap_or(0) != 0)),
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

            Ok(decode_pointer_bits(raw, tree.get(ty)))
        }
        mir::Type::Closure { .. } => {
            let mut raw = [0u8; 8];
            raw[..bytes.len()].copy_from_slice(bytes);
            Ok(Value::managed_reference(ManagedReference::from_bits(
                u64::from_le_bytes(raw),
            )))
        }
        mir::Type::FunctionPointer { .. } => {
            let mut raw = [0u8; 8];
            raw[..bytes.len()].copy_from_slice(bytes);
            Ok(decode_pointer_bits(u64::from_le_bytes(raw), tree.get(ty)))
        }
        mir::Type::Newtype { inner, .. } => decode_raw_value(
            tree,
            (*inner).ty().ok_or_else(|| Error::ConcreteMirRequired {
                context: "newtype inner".to_string(),
            })?,
            bytes,
        ),
        _ => Err(Error::TypeMismatch {
            expected: "scalar or reference raw load".to_string(),
            actual: format!("{ty:?}"),
        }),
    }
}

/// Materialize one VM value from one typed storage byte window.
pub(crate) fn materialize_value_from_storage(
    state: &mut StepState<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    bytes: &[u8],
) -> Result<Value, Error> {
    let layout = state.layout(ty)?;

    // decode composite storage component by component
    if !layout.is_scalar() {
        return allocate_stack_storage_value_by_index(state, ty, |state, index, component_type| {
            let component = state.storage_component_layout(ty, index)?;
            let component_end = component.offset.checked_add(component.byte_len).ok_or(
                Error::InvalidFieldAccess {
                    index,
                    field_count: bytes.len(),
                },
            )?;
            let component_window =
                bytes
                    .get(component.offset..component_end)
                    .ok_or(Error::InvalidFieldAccess {
                        index,
                        field_count: bytes.len(),
                    })?;

            materialize_value_from_storage(state, component_type, component_window)
        });
    }

    decode_raw_value(state.tree(), ty, bytes)
}

/// Resolve the compiled type for one managed allocation.
pub(crate) fn managed_storage_type(
    state: &StepState<'_, '_>,
    handle: ManagedReference,
) -> Result<mir::LocalNodeId<mir::Type>, Error> {
    let type_id = state
        .heap()
        .managed_type_id(handle)
        .ok_or(Error::InvalidManagedReference)?;
    let ty = mir::LocalNodeId::new(type_id);

    let _ = state.layout(ty)?;

    Ok(ty)
}

/// Return cloned typed-storage bytes for one whole-object value when available.
pub(crate) fn clone_typed_storage_bytes_from_value(
    state: &mut StepState<'_, '_>,
    value: Value,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<Option<Vec<u8>>, Error> {
    if let Some(handle) = value.as_managed_reference() {
        let source_type = managed_storage_type(state, handle)?;
        if source_type != ty {
            return Ok(None);
        }

        let bytes = state
            .heap()
            .managed_bytes(handle)
            .ok_or(Error::InvalidManagedReference)?
            .into_owned();
        return Ok(Some(bytes));
    }

    if let Some(pointer) = value.as_stack_pointer() {
        if pointer.slot_offset != 0 {
            return Ok(None);
        }

        let frame = state.frame_by_index(pointer.frame_idx)?;
        let allocation = frame
            .stack_allocation(pointer.slot)
            .ok_or(Error::InvalidManagedReference)?;
        if allocation.storage_type() != ty {
            return Ok(None);
        }

        return Ok(Some(allocation.clone_bytes()));
    }

    Ok(None)
}

/// Materialize one composite value into typed stack storage.
pub(crate) fn allocate_stack_storage_value_by_index<F>(
    state: &mut StepState<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    mut component_value: F,
) -> Result<Value, Error>
where
    F: FnMut(&mut StepState<'_, '_>, u32, mir::LocalNodeId<mir::Type>) -> Result<Value, Error>,
{
    let layout = state.layout(ty)?.clone();
    let component_count = layout
        .component_count()
        .ok_or_else(|| Error::TypeMismatch {
            expected: "composite type".to_string(),
            actual: format!("{ty:?}"),
        })?;

    let component_specs = (0..component_count)
        .map(|index| {
            let component = state.storage_component_layout(ty, index as u32)?;

            Ok::<_, Error>((
                index as u32,
                component.ty,
                component.offset,
                component.byte_len,
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut allocation = crate::interpreter::StackAllocation::new(layout.byte_len, ty);

    // encode each component into its layout slot
    for (index, component_type, component_offset, component_byte_len) in component_specs {
        let component_value = component_value(state, index, component_type)?;
        let component_end =
            component_offset
                .checked_add(component_byte_len)
                .ok_or(Error::InvalidFieldAccess {
                    index,
                    field_count: layout.byte_len,
                })?;

        if component_end > layout.byte_len {
            return Err(Error::InvalidFieldAccess {
                index,
                field_count: layout.byte_len,
            });
        }

        let component_window = allocation
            .bytes_mut()
            .get_mut(component_offset..component_end)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: layout.byte_len,
            })?;

        write_storage_value_into(state, component_type, component_value, component_window)?;
    }

    let frame_index = state.frame_index;
    let slot = state
        .current_frame_mut()
        .allocate_stack_allocation(allocation);
    let pointer = destack_heap::StackPointer::new(frame_index, slot);

    Ok(Value::stack_pointer(pointer))
}

/// Allocate one zeroed typed stack storage value.
pub(crate) fn allocate_zeroed_stack_storage(
    state: &mut StepState<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<Value, Error> {
    let layout = state.layout(ty)?.clone();
    let allocation = crate::interpreter::StackAllocation::new(layout.byte_len, ty);
    let frame_index = state.frame_index;
    let slot = state
        .current_frame_mut()
        .allocate_stack_allocation(allocation);
    let pointer = destack_heap::StackPointer::new(frame_index, slot);

    Ok(Value::stack_pointer(pointer))
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
        mir::Type::Reference { address_space, .. } => {
            let raw = match address_space {
                mir::AddressSpace::Shared => value
                    .as_shared_pointer()
                    .map(|pointer| pointer.bits())
                    .ok_or_else(|| Error::TypeMismatch {
                        expected: "shared pointer".to_string(),
                        actual: format!("{value:?}"),
                    })?,
                mir::AddressSpace::Stack => value
                    .as_stack_pointer()
                    .map(|pointer| Value::stack_pointer(pointer).raw_data())
                    .ok_or_else(|| Error::TypeMismatch {
                        expected: "stack pointer".to_string(),
                        actual: format!("{value:?}"),
                    })?,
                mir::AddressSpace::Local => value
                    .as_local_pointer()
                    .map(|pointer| Value::local_pointer(pointer).raw_data())
                    .ok_or_else(|| Error::TypeMismatch {
                        expected: "local pointer".to_string(),
                        actual: format!("{value:?}"),
                    })?,
                mir::AddressSpace::Global | mir::AddressSpace::Constant => value
                    .as_global_pointer()
                    .map(|pointer| {
                        Value::global_pointer_with_offset(pointer.id, pointer.slot_offset)
                            .raw_data()
                    })
                    .ok_or_else(|| Error::TypeMismatch {
                        expected: "global pointer".to_string(),
                        actual: format!("{value:?}"),
                    })?,
                _ => value
                    .as_managed_reference()
                    .map(|handle| handle.bits())
                    .or_else(|| value.as_raw_pointer().map(|pointer| pointer.bits()))
                    .ok_or_else(|| Error::TypeMismatch {
                        expected: "reference".to_string(),
                        actual: format!("{value:?}"),
                    })?,
            };
            raw.to_le_bytes()[..byte_len].to_vec()
        }
        mir::Type::Closure { .. } => {
            let raw = value
                .as_managed_reference()
                .map(|handle| handle.bits())
                .ok_or_else(|| Error::TypeMismatch {
                    expected: "boxed function value".to_string(),
                    actual: format!("{value:?}"),
                })?;
            raw.to_le_bytes()[..byte_len].to_vec()
        }
        mir::Type::FunctionPointer { .. } => {
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

/// Write one typed storage value into one destination byte window.
pub(crate) fn write_storage_value_into(
    state: &mut StepState<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    value: Value,
    destination: &mut [u8],
) -> Result<(), Error> {
    if state.layout(ty)?.is_scalar() {
        let bytes = encode_storage_value(state, ty, value)?;
        if bytes.len() != destination.len() {
            return Err(Error::InvalidManagedReference);
        }

        destination.copy_from_slice(&bytes);
        return Ok(());
    }

    let Some(source_bytes) = clone_typed_storage_bytes_from_value(state, value, ty)? else {
        return Err(Error::TypeMismatch {
            expected: "typed composite component".to_string(),
            actual: format!("{value:?}"),
        });
    };
    if source_bytes.len() != destination.len() {
        return Err(Error::InvalidManagedReference);
    }

    destination.copy_from_slice(source_bytes.as_ref());

    Ok(())
}

/// Decode one boxed callable object into function and environment values.
pub(crate) fn decode_function_value(
    state: &mut StepState<'_, '_>,
    value: Value,
) -> Result<(Value, Value), Error> {
    if value.tag() != ValueTag::ManagedReference {
        return Err(Error::TypeMismatch {
            expected: "boxed function value".to_string(),
            actual: format!("{value:?}"),
        });
    }

    let handle = value
        .as_managed_reference()
        .ok_or_else(|| Error::TypeMismatch {
            expected: "boxed function value".to_string(),
            actual: format!("{value:?}"),
        })?;
    let ty = managed_storage_type(state, handle)?;
    let ty = repr_type(state.tree(), ty);

    if !matches!(state.tree().get(ty), mir::Type::Closure { .. }) {
        return Err(Error::TypeMismatch {
            expected: "function value type".to_string(),
            actual: format!("{ty:?}"),
        });
    }

    decode_function_value_storage(state, handle, ty)
}

/// Load one typed value from raw heap bytes.
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

    let owned_window = {
        let bytes = state
            .heap()
            .raw_bytes(pointer)
            .ok_or(Error::InvalidManagedReference)?;
        let window = bytes
            .get(..access.byte_len)
            .ok_or(Error::InvalidFieldAccess {
                index: 0,
                field_count: bytes.len(),
            })?;

        if access.is_scalar {
            return decode_raw_value(state.tree(), access.value_type, window);
        }

        window.to_vec()
    };

    materialize_value_from_storage(state, access.value_type, &owned_window)
}

/// Store one typed value into raw heap bytes.
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

    if !access.is_scalar
        && let Some(handle) = value.as_managed_reference()
    {
        let source_type = managed_storage_type(state, handle)?;
        if source_type == access.value_type {
            let source_bytes = state
                .heap()
                .managed_bytes(handle)
                .ok_or(Error::InvalidManagedReference)?
                .into_owned();

            if source_bytes.len() != access.byte_len {
                return Err(Error::InvalidManagedReference);
            }

            return write_raw_bytes(state, pointer, 0, &source_bytes);
        }
    }

    let bytes = encode_storage_value(state, access.value_type, value)?;

    write_raw_bytes(state, pointer, 0, &bytes)
}
