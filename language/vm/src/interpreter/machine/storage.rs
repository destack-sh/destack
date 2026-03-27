use crate::diagnostic::Error;
use crate::executable::{UNKNOWN_ARRAY_LENGTH, UNKNOWN_FIELD_COUNT};
use destack_heap::{RawPointer, ReferenceMap, Value, ValueTag};
use destack_mir as mir;

use super::access::{decode_pointer_bits, invalid_pointer_type};
use crate::interpreter::StepState;

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
    let size = match tree.get(ty) {
        mir::Type::Void => 0,
        mir::Type::Boolean => 1,
        mir::Type::Int { width, .. } => (*width as usize).div_ceil(8),
        mir::Type::Isize
        | mir::Type::Usize
        | mir::Type::TypeDescriptor
        | mir::Type::TypeId
        | mir::Type::Reference { .. }
        | mir::Type::FunctionPointer { .. }
        | mir::Type::TensorReference { .. } => tree.pointer_bytes() as usize,
        mir::Type::Float { width } => (*width as usize).div_ceil(8),
        mir::Type::Newtype { inner, .. } => return raw_type_size(tree, *inner),
        mir::Type::Array { .. }
        | mir::Type::Tuple { .. }
        | mir::Type::Struct { .. }
        | mir::Type::FunctionValue { .. }
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

/// Resolve the byte size for one managed pointee type.
pub(crate) fn managed_type_size(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<usize, Error> {
    let size = match tree.get(ty) {
        mir::Type::Reference {
            kind: mir::ReferenceKind::Managed,
            ..
        }
        | mir::Type::TensorReference {
            kind: mir::ReferenceKind::Managed,
            ..
        } => tree.data_layout.managed_reference_layout.bytes as usize,
        _ => raw_type_size(tree, ty)?,
    };

    Ok(size)
}

/// Build one managed reference map for a runtime array allocation.
pub(crate) fn managed_array_reference_map(
    tree: &mir::NodeTree,
    element_type: mir::LocalNodeId<mir::Type>,
    count: usize,
) -> ReferenceMap {
    if count == 0 {
        return ReferenceMap::empty();
    }

    let element_size = match managed_type_size(tree, element_type) {
        Ok(element_size) => element_size,
        Err(_) => return ReferenceMap::empty(),
    };

    let mut offsets = Vec::new();
    append_managed_reference_map_offsets(tree, element_type, 0, &mut offsets);

    if offsets.is_empty() {
        ReferenceMap::empty()
    } else {
        ReferenceMap::RepeatedReferenceOffsets {
            count: count as u32,
            element_size: element_size as u32,
            offsets,
        }
    }
}

fn append_managed_reference_map_offsets(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
    base_offset: u32,
    offsets: &mut Vec<u32>,
) {
    match tree.get(ty) {
        mir::Type::Reference {
            kind: mir::ReferenceKind::Managed,
            ..
        }
        | mir::Type::TensorReference {
            kind: mir::ReferenceKind::Managed,
            ..
        } => {
            offsets.push(base_offset);
        }
        mir::Type::Newtype { inner, .. } => {
            append_managed_reference_map_offsets(tree, *inner, base_offset, offsets);
        }
        mir::Type::Struct { .. }
        | mir::Type::Tuple { .. }
        | mir::Type::FunctionValue { .. }
        | mir::Type::Vector { .. }
        | mir::Type::Tensor { .. } => {
            let Some(layout) = tree.type_layout(ty) else {
                return;
            };

            for field in &layout.fields {
                append_managed_reference_map_offsets(
                    tree,
                    field.ty,
                    base_offset.saturating_add(field.offset),
                    offsets,
                );
            }
        }
        mir::Type::Array {
            element, length, ..
        } => {
            let Some(layout) = tree.type_layout(ty) else {
                return;
            };
            let mir::LayoutType::Array { element_stride, .. } = &layout.layout_type else {
                return;
            };

            for index in 0..*length as u32 {
                let element_base =
                    base_offset.saturating_add(index.saturating_mul(*element_stride));
                append_managed_reference_map_offsets(tree, *element, element_base, offsets);
            }
        }
        _ => {}
    }
}

/// Resolve one raw struct or tuple field type and byte offset.
fn layout_field_info(
    tree: &mir::NodeTree,
    aggregate_type: mir::LocalNodeId<mir::Type>,
    index: u32,
    expected: &'static str,
) -> Result<(mir::LocalNodeId<mir::Type>, usize), Error> {
    match tree.get(aggregate_type) {
        mir::Type::Struct { fields, .. } => {
            let field_id =
                fields
                    .get(index as usize)
                    .copied()
                    .ok_or(Error::InvalidFieldAccess {
                        index,
                        field_count: fields.len(),
                    })?;
            let field_ty = tree.get(field_id).ty;
            let layout = tree
                .type_layout(aggregate_type)
                .ok_or(Error::InvalidManagedReference)?;
            let field = layout
                .fields
                .get(index as usize)
                .ok_or(Error::InvalidFieldAccess {
                    index,
                    field_count: layout.fields.len(),
                })?;

            Ok((field_ty, field.offset as usize))
        }
        mir::Type::Tuple { elements, .. } => {
            let field_ty =
                elements
                    .get(index as usize)
                    .copied()
                    .ok_or(Error::InvalidFieldAccess {
                        index,
                        field_count: elements.len(),
                    })?;
            let layout = tree
                .type_layout(aggregate_type)
                .ok_or(Error::InvalidManagedReference)?;
            let field = layout
                .fields
                .get(index as usize)
                .ok_or(Error::InvalidFieldAccess {
                    index,
                    field_count: layout.fields.len(),
                })?;

            Ok((field_ty, field.offset as usize))
        }
        mir::Type::Newtype { inner, .. } => layout_field_info(tree, *inner, index, expected),
        _ => Err(Error::TypeMismatch {
            expected: expected.to_string(),
            actual: format!("{aggregate_type:?}"),
        }),
    }
}

/// Resolve one raw struct or tuple field type and byte offset.
pub(super) fn raw_field_info(
    tree: &mir::NodeTree,
    aggregate_type: mir::LocalNodeId<mir::Type>,
    index: u32,
) -> Result<(mir::LocalNodeId<mir::Type>, usize), Error> {
    layout_field_info(tree, aggregate_type, index, "raw aggregate")
}

/// Resolve one managed struct or tuple field type and byte offset.
pub(super) fn managed_field_info(
    tree: &mir::NodeTree,
    aggregate_type: mir::LocalNodeId<mir::Type>,
    index: u32,
) -> Result<(mir::LocalNodeId<mir::Type>, usize), Error> {
    layout_field_info(tree, aggregate_type, index, "managed aggregate")
}

/// Resolve one raw array element type and byte stride.
fn array_element_info(
    tree: &mir::NodeTree,
    array_type: mir::LocalNodeId<mir::Type>,
    expected: &'static str,
    expected_layout: &'static str,
) -> Result<(mir::LocalNodeId<mir::Type>, usize), Error> {
    match tree.get(array_type) {
        mir::Type::Array { element, .. } => {
            let layout = tree
                .type_layout(array_type)
                .ok_or(Error::InvalidManagedReference)?;
            let mir::LayoutType::Array {
                element_stride,
                element_type,
                ..
            } = &layout.layout_type
            else {
                return Err(Error::TypeMismatch {
                    expected: expected_layout.to_string(),
                    actual: format!("{array_type:?}"),
                });
            };
            debug_assert_eq!(*element, *element_type);

            Ok((*element_type, *element_stride as usize))
        }
        mir::Type::Newtype { inner, .. } => {
            array_element_info(tree, *inner, expected, expected_layout)
        }
        _ => Err(Error::TypeMismatch {
            expected: expected.to_string(),
            actual: format!("{array_type:?}"),
        }),
    }
}

/// Resolve one raw array element type and byte stride.
pub(super) fn raw_element_info(
    tree: &mir::NodeTree,
    array_type: mir::LocalNodeId<mir::Type>,
) -> Result<(mir::LocalNodeId<mir::Type>, usize), Error> {
    array_element_info(tree, array_type, "raw array", "raw array layout")
}

/// Resolve one managed array element type and byte stride.
pub(super) fn managed_element_info(
    tree: &mir::NodeTree,
    array_type: mir::LocalNodeId<mir::Type>,
) -> Result<(mir::LocalNodeId<mir::Type>, usize), Error> {
    array_element_info(tree, array_type, "managed array", "managed array layout")
}

/// Read one raw byte window into an owned buffer.
fn read_raw_bytes(
    state: &StepState<'_, '_>,
    pointer: RawPointer,
    byte_offset: usize,
    byte_len: usize,
) -> Result<Vec<u8>, Error> {
    let bytes = state
        .heap_ref()
        .raw_bytes(pointer)
        .ok_or(Error::InvalidManagedReference)?;

    let end = byte_offset
        .checked_add(byte_len)
        .ok_or(Error::InvalidFieldAccess {
            index: byte_offset as u32,
            field_count: bytes.len(),
        })?;

    let window = bytes
        .get(byte_offset..end)
        .ok_or(Error::InvalidFieldAccess {
            index: byte_offset as u32,
            field_count: bytes.len(),
        })?;

    Ok(window.to_vec())
}

/// Write one raw byte window from the given buffer.
fn write_raw_bytes(
    state: &mut StepState<'_, '_>,
    pointer: RawPointer,
    byte_offset: usize,
    bytes: &[u8],
) -> Result<(), Error> {
    let byte_len = state
        .heap_ref()
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

    for (index, byte) in bytes.iter().copied().enumerate() {
        if !state
            .heap()
            .set_raw_byte(pointer, byte_offset + index, byte)
        {
            return Err(Error::InvalidFieldAccess {
                index: (byte_offset + index) as u32,
                field_count: byte_len,
            });
        }
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
        mir::Type::FunctionPointer { .. } => {
            let mut raw = [0u8; 8];
            raw[..bytes.len()].copy_from_slice(bytes);
            Ok(decode_pointer_bits(u64::from_le_bytes(raw), tree.get(ty)))
        }
        mir::Type::Newtype { inner, .. } => decode_raw_value(tree, *inner, bytes),
        _ => Err(Error::TypeMismatch {
            expected: "scalar or reference raw load".to_string(),
            actual: format!("{ty:?}"),
        }),
    }
}

/// Decode one storage byte window into a VM value.
pub(crate) fn decode_storage_value(
    state: &mut StepState<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    bytes: &[u8],
) -> Result<Value, Error> {
    // classify the storage type before mutating the step state again
    let classify = match state.tree().get(ty) {
        mir::Type::Newtype { inner, .. } => Some(Err(*inner)),
        mir::Type::Array { .. }
        | mir::Type::Tuple { .. }
        | mir::Type::Struct { .. }
        | mir::Type::FunctionValue { .. }
        | mir::Type::Vector { .. }
        | mir::Type::Tensor { .. } => Some(Ok(())),
        _ => None,
    };

    // recurse through newtype wrappers without cloning the whole MIR type
    if let Some(Err(inner)) = classify {
        return decode_storage_value(state, inner, bytes);
    }

    // decode aggregate storage component by component
    if let Some(Ok(())) = classify {
        let component_specs = {
            let tree = state.tree();
            let component_count = aggregate_component_count(tree, ty)?;

            (0..component_count)
                .map(|index| {
                    let (component_type, component_offset) =
                        aggregate_component_info(tree, ty, index as u32)?;
                    let component_byte_len = raw_type_size(tree, component_type)?;

                    Ok::<_, Error>((
                        index as u32,
                        component_type,
                        component_offset,
                        component_byte_len,
                    ))
                })
                .collect::<Result<Vec<_>, _>>()?
        };
        let mut values = Vec::with_capacity(component_specs.len());

        // decode each aggregate component in semantic order
        for (index, component_type, component_offset, component_byte_len) in component_specs {
            let component_end = component_offset.checked_add(component_byte_len).ok_or(
                Error::InvalidFieldAccess {
                    index,
                    field_count: bytes.len(),
                },
            )?;
            let component_window =
                bytes
                    .get(component_offset..component_end)
                    .ok_or(Error::InvalidFieldAccess {
                        index,
                        field_count: bytes.len(),
                    })?;
            let value = decode_storage_value(state, component_type, component_window)?;

            values.push(value);
        }

        return Ok(allocate_aggregate_value(state, values));
    }

    decode_raw_value(state.tree(), ty, bytes)
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
        mir::Type::FunctionPointer { .. } => {
            let raw = value
                .as_function_pointer()
                .ok_or_else(|| Error::TypeMismatch {
                    expected: "function pointer".to_string(),
                    actual: format!("{value:?}"),
                })?;
            (raw.id as u64).to_le_bytes()[..byte_len].to_vec()
        }
        mir::Type::Newtype { inner, .. } => return encode_raw_value(tree, *inner, value),
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
    // classify the storage type before mutating the step state again
    let classify = match state.tree().get(ty) {
        mir::Type::Newtype { inner, .. } => Some(Err(*inner)),
        mir::Type::Array { .. }
        | mir::Type::Tuple { .. }
        | mir::Type::Struct { .. }
        | mir::Type::FunctionValue { .. }
        | mir::Type::Vector { .. }
        | mir::Type::Tensor { .. } => Some(Ok(())),
        _ => None,
    };

    // recurse through newtype wrappers without cloning the whole MIR type
    if let Some(Err(inner)) = classify {
        return encode_storage_value(state, inner, value);
    }

    // encode aggregate storage component by component
    if let Some(Ok(())) = classify {
        let (byte_len, component_specs) = {
            let tree = state.tree();
            let component_count = aggregate_component_count(tree, ty)?;
            let byte_len = raw_type_size(tree, ty)?;
            let component_specs = (0..component_count)
                .map(|index| {
                    let (component_type, component_offset) =
                        aggregate_component_info(tree, ty, index as u32)?;

                    Ok::<_, Error>((index as u32, component_type, component_offset))
                })
                .collect::<Result<Vec<_>, _>>()?;

            (byte_len, component_specs)
        };
        let component_values = aggregate_component_values(state, value, component_specs.len())?;
        let mut bytes = vec![0; byte_len];

        // encode each aggregate component into its layout slot
        for ((index, component_type, component_offset), component_value) in component_specs
            .into_iter()
            .zip(component_values.into_iter())
        {
            let component_bytes = encode_storage_value(state, component_type, component_value)?;
            let component_end = component_offset.checked_add(component_bytes.len()).ok_or(
                Error::InvalidFieldAccess {
                    index,
                    field_count: byte_len,
                },
            )?;

            if component_end > byte_len {
                return Err(Error::InvalidFieldAccess {
                    index,
                    field_count: byte_len,
                });
            }

            bytes[component_offset..component_end].copy_from_slice(&component_bytes);
        }

        return Ok(bytes);
    }

    encode_raw_value(state.tree(), ty, value)
}

/// Return the semantic component count for one aggregate storage type.
fn aggregate_component_count(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<usize, Error> {
    let count = match tree.get(ty) {
        mir::Type::Array { length, .. } => *length as usize,
        mir::Type::Tuple { elements, .. } => elements.len(),
        mir::Type::Struct { fields, .. } => fields.len(),
        mir::Type::FunctionValue { .. } => 2,
        mir::Type::Vector { lanes, .. } => *lanes as usize,
        mir::Type::Tensor { shape, layout, .. } => compute_tensor_element_count(shape, layout),
        mir::Type::Newtype { inner, .. } => return aggregate_component_count(tree, *inner),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "aggregate storage type".to_string(),
                actual: format!("{ty:?}"),
            });
        }
    };

    Ok(count)
}

/// Resolve one aggregate component type and byte offset for aggregate storage.
fn aggregate_component_info(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
    index: u32,
) -> Result<(mir::LocalNodeId<mir::Type>, usize), Error> {
    match tree.get(ty) {
        mir::Type::Struct { .. } | mir::Type::Tuple { .. } => raw_field_info(tree, ty, index),
        mir::Type::Array { .. } => {
            let (element_type, element_stride) = raw_element_info(tree, ty)?;
            let element_offset = usize::try_from(index)
                .ok()
                .and_then(|index| index.checked_mul(element_stride))
                .ok_or(Error::InvalidArrayAccess {
                    index: index as u64,
                    length: aggregate_component_count(tree, ty)? as u64,
                })?;

            Ok((element_type, element_offset))
        }
        mir::Type::FunctionValue { signature } => {
            let layout = tree.type_layout(ty).ok_or(Error::InvalidManagedReference)?;
            let environment = tree.function_value_environment_type();

            // function values use semantic component order, not concrete field order
            let component_type = match index {
                0 => *signature,
                1 => environment,
                _ => {
                    return Err(Error::InvalidFieldAccess {
                        index,
                        field_count: 2,
                    });
                }
            };
            let field = layout
                .fields
                .iter()
                .find(|field| field.source_index == Some(index))
                .or_else(|| layout.fields.get(index as usize))
                .ok_or(Error::InvalidFieldAccess {
                    index,
                    field_count: layout.fields.len(),
                })?;

            Ok((component_type, field.offset as usize))
        }
        mir::Type::Vector { element, .. } => {
            let element_size = raw_type_size(tree, *element)?;
            let element_offset = usize::try_from(index)
                .ok()
                .and_then(|index| index.checked_mul(element_size))
                .ok_or(Error::InvalidArrayAccess {
                    index: index as u64,
                    length: aggregate_component_count(tree, ty)? as u64,
                })?;

            Ok((*element, element_offset))
        }
        mir::Type::Tensor {
            element,
            shape,
            layout,
            ..
        } => {
            let element_size = raw_type_size(tree, *element)?;
            let element_count = compute_tensor_element_count(shape, layout);
            let element_offset = usize::try_from(index)
                .ok()
                .and_then(|index| index.checked_mul(element_size))
                .ok_or(Error::InvalidArrayAccess {
                    index: index as u64,
                    length: element_count as u64,
                })?;

            Ok((*element, element_offset))
        }
        mir::Type::Newtype { inner, .. } => aggregate_component_info(tree, *inner, index),
        _ => Err(Error::TypeMismatch {
            expected: "aggregate storage type".to_string(),
            actual: format!("{ty:?}"),
        }),
    }
}

/// Count tensor elements for one static tensor shape and layout.
fn compute_tensor_element_count(
    shape: &[mir::TensorDimension],
    layout: &mir::TensorLayout,
) -> usize {
    let shape: Vec<u64> = shape
        .iter()
        .map(|dimension| match dimension {
            mir::TensorDimension::Static(value) => *value,
            mir::TensorDimension::Dynamic => 0,
        })
        .collect();

    match layout {
        mir::TensorLayout::RowMajor | mir::TensorLayout::ColumnMajor => shape
            .iter()
            .copied()
            .product::<u64>()
            .min(usize::MAX as u64)
            as usize,
        mir::TensorLayout::Strided { strides } => {
            let strides: Vec<u64> = strides
                .iter()
                .map(|dimension| match dimension {
                    mir::TensorDimension::Static(value) => *value,
                    mir::TensorDimension::Dynamic => 0,
                })
                .collect();
            let mut max_index = 0u64;

            // compute the highest reachable logical element index
            for (dimension, stride) in shape.iter().copied().zip(strides.iter().copied()) {
                if dimension == 0 {
                    continue;
                }

                max_index = max_index.saturating_add((dimension - 1).saturating_mul(stride));
            }

            max_index.saturating_add(1).min(usize::MAX as u64) as usize
        }
    }
}

/// Read one aggregate value into semantic component values.
pub(crate) fn aggregate_component_values(
    state: &StepState<'_, '_>,
    value: Value,
    expected_count: usize,
) -> Result<Vec<Value>, Error> {
    if value.tag() != ValueTag::Aggregate {
        return Err(Error::TypeMismatch {
            expected: "aggregate".to_string(),
            actual: format!("{value:?}"),
        });
    }

    let handle = value.as_managed_reference().unwrap();
    let values = state
        .heap_ref()
        .packed_values_to_vec(handle)
        .ok_or(Error::InvalidManagedReference)?;

    if values.len() != expected_count {
        return Err(Error::TypeMismatch {
            expected: format!("aggregate with {expected_count} fields"),
            actual: format!("aggregate with {} fields", values.len()),
        });
    }

    Ok(values)
}

/// Allocate one aggregate value from semantic components.
fn allocate_aggregate_value(state: &mut StepState<'_, '_>, values: Vec<Value>) -> Value {
    match values.as_slice() {
        [value] => state.allocate_single(*value),
        [first, second] => state.allocate_pair(*first, *second),
        _ => state.allocate_aggregate(values),
    }
}

/// Load one typed value from raw heap bytes.
pub(crate) fn load_from_raw_pointer_typed(
    state: &mut StepState<'_, '_>,
    ptr: Value,
    pointee: mir::LocalNodeId<mir::Type>,
) -> Result<Value, Error> {
    if ptr.tag() != ValueTag::RawPointer {
        return Err(Error::InvalidPointerType {
            actual: format!("{ptr:?}"),
        });
    }

    let pointer = ptr.as_raw_pointer().unwrap();
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let byte_len = raw_type_size(state.tree(), pointee)?;
    let bytes = read_raw_bytes(state, pointer, 0, byte_len)?;

    decode_storage_value(state, pointee, &bytes)
}

/// Store one typed value into raw heap bytes.
pub(crate) fn store_to_raw_pointer_typed(
    state: &mut StepState<'_, '_>,
    ptr: Value,
    pointee: mir::LocalNodeId<mir::Type>,
    value: Value,
) -> Result<(), Error> {
    if ptr.tag() != ValueTag::RawPointer {
        return Err(invalid_pointer_type(ptr));
    }

    let pointer = ptr.as_raw_pointer().unwrap();
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let bytes = encode_storage_value(state, pointee, value)?;

    write_raw_bytes(state, pointer, 0, &bytes)
}
