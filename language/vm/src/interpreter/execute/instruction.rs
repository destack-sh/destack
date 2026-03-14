use crate::diagnostic::Error;
use destack_heap::{
    GlobalPointer, Heap, LocalPointer, ManagedReference, RawPointer, ReferenceMap, ReferenceMeta,
    SharedPointer, StackPointer, StringLayout, Value, ValueTag,
};
use destack_mir as mir;

use super::super::decode::{ThreadedState, UNKNOWN_ARRAY_LENGTH, UNKNOWN_FIELD_COUNT};
use crate::telemetry::stat_inc;

/// Return the packed VM value count for one managed allocation.
#[inline(always)]
fn packed_value_count(heap: &Heap, handle: ManagedReference) -> Result<usize, Error> {
    heap.packed_value_count(handle)
        .ok_or(Error::InvalidManagedReference)
}

/// Report whether one managed allocation stores packed VM values.
#[inline(always)]
fn has_packed_values(heap: &Heap, handle: ManagedReference) -> bool {
    matches!(
        heap.reference_map(handle),
        Some(ReferenceMap::ValueArray { .. })
    )
}

/// Load one packed VM value from one managed allocation.
#[inline(always)]
fn load_packed_value(heap: &Heap, handle: ManagedReference, index: usize) -> Result<Value, Error> {
    heap.packed_value_at(handle, index)
        .ok_or(Error::InvalidManagedReference)
}

/// Store one packed VM value into one managed allocation.
#[inline(always)]
fn store_packed_value(
    heap: &mut Heap,
    handle: ManagedReference,
    index: usize,
    value: Value,
) -> Result<(), Error> {
    if heap.set_packed_value(handle, index, value) {
        return Ok(());
    }

    Err(Error::InvalidManagedReference)
}

/// Load one field from one runtime string header.
#[inline(always)]
fn load_string_field(heap: &Heap, handle: ManagedReference, index: u32) -> Result<Value, Error> {
    let bytes = heap
        .managed_bytes(handle)
        .ok_or(Error::InvalidManagedReference)?;

    StringLayout::read_field(bytes.as_ref(), index).ok_or(Error::InvalidFieldAccess {
        index,
        field_count: StringLayout::FIELD_COUNT,
    })
}

/// Store one field into one runtime string header.
#[inline(always)]
fn store_string_field(
    heap: &mut Heap,
    handle: ManagedReference,
    index: u32,
    value: Value,
) -> Result<(), Error> {
    let mut bytes = heap
        .managed_bytes_to_vec(handle)
        .ok_or(Error::InvalidManagedReference)?;

    if !StringLayout::write_field(&mut bytes, index, value) {
        return Err(Error::TypeMismatch {
            expected: "string field value".to_string(),
            actual: format!("{value:?}"),
        });
    }

    for (offset, byte) in bytes.into_iter().enumerate() {
        if !heap.set_managed_byte(handle, offset, byte) {
            return Err(Error::InvalidManagedReference);
        }
    }

    Ok(())
}

/// Return one byte offset for one runtime string field.
#[inline(always)]
fn string_field_offset(index: u32) -> Result<usize, Error> {
    StringLayout::field_offset(index).ok_or(Error::InvalidFieldAccess {
        index,
        field_count: StringLayout::FIELD_COUNT,
    })
}

/// Load a value from a pointer with one optional raw pointee type.
#[inline(always)]
pub(crate) fn load_from_pointer_with_raw_pointee(
    state: &mut ThreadedState<'_, '_>,
    ptr: Value,
    raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.interpreter.engine.statistics, loads);
    }

    // resolve pointer kind and load
    match ptr.tag() {
        ValueTag::ManagedReference => {
            let handle = ptr.as_managed_reference().unwrap();
            let value_index = handle.byte_offset() / Value::BYTE_LEN;
            load_heap_slot(state, handle, value_index)
        }
        ValueTag::RawPointer => {
            let Some(raw_pointee) = raw_pointee else {
                return Err(Error::InvalidPointerType {
                    actual: "raw pointer without pointee type".to_string(),
                });
            };

            load_from_raw_pointer_typed(state, ptr, raw_pointee)
        }
        ValueTag::StackPointer => {
            let sp = ptr.as_stack_pointer().unwrap();
            load_stack_slot(state, sp, sp.slot_offset)
        }
        ValueTag::LocalPointer => {
            let lp = ptr.as_local_pointer().unwrap();
            load_local_slot(state, lp, lp.slot_offset)
        }
        ValueTag::GlobalPointer => {
            let global = ptr.as_global_pointer().unwrap();
            load_global_slot(state, global)
        }
        _ => Err(Error::InvalidPointerType {
            actual: format!("{ptr:?}"),
        }),
    }
}

/// Store a value to a pointer with one optional raw pointee type.
#[inline(always)]
pub(crate) fn store_to_pointer_with_raw_pointee(
    state: &mut ThreadedState<'_, '_>,
    ptr: Value,
    raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
    val: Value,
) -> Result<(), Error> {
    // track pointer stores
    if state.collect_stats {
        stat_inc!(state.interpreter.engine.statistics, stores);
    }

    // resolve pointer kind and store
    match ptr.tag() {
        ValueTag::ManagedReference => {
            let handle = ptr.as_managed_reference().unwrap();
            let value_index = handle.byte_offset() / Value::BYTE_LEN;
            store_heap_slot(state, handle, value_index, val)
        }
        ValueTag::RawPointer => {
            let Some(raw_pointee) = raw_pointee else {
                return Err(Error::InvalidPointerType {
                    actual: "raw pointer without pointee type".to_string(),
                });
            };

            store_to_raw_pointer_typed(state, ptr, raw_pointee, val)
        }
        ValueTag::StackPointer => {
            let sp = ptr.as_stack_pointer().unwrap();
            store_stack_slot(state, sp, sp.slot_offset, val)
        }
        ValueTag::LocalPointer => {
            let lp = ptr.as_local_pointer().unwrap();
            store_local_slot(state, lp, lp.slot_offset, val)
        }
        ValueTag::GlobalPointer => {
            let global = ptr.as_global_pointer().unwrap();
            let global_def = state.interpreter.isolate.image.tree.get(global.id);
            if !global_def.is_mutable() {
                return Err(Error::ImmutableGlobalWrite { global: global.id });
            }
            store_global_slot(state, global, val)
        }
        _ => Err(Error::InvalidPointerType {
            actual: format!("{ptr:?}"),
        }),
    }
}

/// Load a value from a managed reference.
#[inline(always)]
pub(crate) fn load_from_managed_reference(
    state: &mut ThreadedState<'_, '_>,
    ptr: Value,
) -> Result<Value, Error> {
    // validate pointer tag
    if ptr.tag() != ValueTag::ManagedReference {
        return Err(Error::InvalidPointerType {
            actual: format!("{ptr:?}"),
        });
    }

    // resolve handle
    let handle = ptr.as_managed_reference().unwrap();
    let value_index = handle.byte_offset() / Value::BYTE_LEN;
    load_heap_slot(state, handle, value_index)
}

/// Load a typed value from a managed reference.
#[inline(always)]
pub(crate) fn load_from_managed_reference_typed(
    state: &mut ThreadedState<'_, '_>,
    ptr: Value,
    pointee: mir::LocalNodeId<mir::Type>,
) -> Result<Value, Error> {
    if ptr.tag() != ValueTag::ManagedReference {
        return Err(Error::InvalidPointerType {
            actual: format!("{ptr:?}"),
        });
    }

    let handle = ptr.as_managed_reference().unwrap();
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let heap = state.heap_ref();
    if !heap.is_managed_allocated(handle) {
        return Err(Error::InvalidManagedReference);
    }

    if has_packed_values(heap, handle) {
        let slot_offset = handle.byte_offset() / Value::BYTE_LEN;
        return load_heap_slot(state, handle, slot_offset);
    }

    let pointee = managed_projection_type(&state.interpreter.isolate.image.tree, pointee);
    let byte_len = managed_type_size(&state.interpreter.isolate.image.tree, pointee)?;
    let bytes = heap
        .managed_bytes(handle)
        .ok_or(Error::InvalidManagedReference)?;
    let window = bytes.get(..byte_len).ok_or(Error::InvalidFieldAccess {
        index: 0,
        field_count: bytes.len(),
    })?;

    decode_raw_value(&state.interpreter.isolate.image.tree, pointee, window)
}

/// Load a value from a stack pointer.
#[inline(always)]
pub(crate) fn load_from_stack_pointer(
    state: &mut ThreadedState<'_, '_>,
    ptr: Value,
) -> Result<Value, Error> {
    // validate pointer tag
    if ptr.tag() != ValueTag::StackPointer {
        return Err(Error::InvalidPointerType {
            actual: format!("{ptr:?}"),
        });
    }

    // resolve pointer
    let sp = ptr.as_stack_pointer().unwrap();
    load_stack_slot(state, sp, sp.slot_offset)
}

/// Load a value from a local pointer.
#[inline(always)]
pub(crate) fn load_from_local_pointer(
    state: &mut ThreadedState<'_, '_>,
    ptr: Value,
) -> Result<Value, Error> {
    // validate pointer tag
    if ptr.tag() != ValueTag::LocalPointer {
        return Err(Error::InvalidPointerType {
            actual: format!("{ptr:?}"),
        });
    }

    // resolve pointer
    let lp = ptr.as_local_pointer().unwrap();
    load_local_slot(state, lp, lp.slot_offset)
}

/// Load a value from a global pointer.
#[inline(always)]
pub(crate) fn load_from_global_pointer(
    state: &mut ThreadedState<'_, '_>,
    ptr: Value,
) -> Result<Value, Error> {
    // validate pointer tag
    if ptr.tag() != ValueTag::GlobalPointer {
        return Err(Error::InvalidPointerType {
            actual: format!("{ptr:?}"),
        });
    }

    // resolve pointer
    let global = ptr.as_global_pointer().unwrap();
    load_global_slot(state, global)
}

/// Store a typed value through a managed reference.
#[inline(always)]
pub(crate) fn store_to_managed_reference_typed(
    state: &mut ThreadedState<'_, '_>,
    ptr: Value,
    pointee: mir::LocalNodeId<mir::Type>,
    val: Value,
) -> Result<(), Error> {
    if ptr.tag() != ValueTag::ManagedReference {
        return Err(Error::InvalidPointerType {
            actual: format!("{ptr:?}"),
        });
    }

    let handle = ptr.as_managed_reference().unwrap();
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let is_value_array = {
        let heap = state.heap_ref();
        if !heap.is_managed_allocated(handle) {
            return Err(Error::InvalidManagedReference);
        }
        has_packed_values(heap, handle)
    };

    if is_value_array {
        let slot_offset = handle.byte_offset() / Value::BYTE_LEN;
        return store_heap_slot(state, handle, slot_offset, val);
    }

    let pointee = managed_projection_type(&state.interpreter.isolate.image.tree, pointee);
    let bytes = encode_raw_value(&state.interpreter.isolate.image.tree, pointee, val)?;
    let byte_len = state
        .heap_ref()
        .managed_byte_len(handle)
        .ok_or(Error::InvalidManagedReference)?;
    let start = 0usize;
    let end = bytes.len();

    if end > byte_len {
        return Err(Error::InvalidFieldAccess {
            index: start as u32,
            field_count: byte_len,
        });
    }

    for (index, byte) in bytes.iter().copied().enumerate() {
        if !state.heap().set_managed_byte(handle, index, byte) {
            return Err(Error::InvalidFieldAccess {
                index: index as u32,
                field_count: byte_len,
            });
        }
    }

    Ok(())
}

/// Store a value through a stack pointer.
#[inline(always)]
pub(crate) fn store_to_stack_pointer(
    state: &mut ThreadedState<'_, '_>,
    ptr: Value,
    val: Value,
) -> Result<(), Error> {
    // validate pointer tag
    if ptr.tag() != ValueTag::StackPointer {
        return Err(Error::InvalidPointerType {
            actual: format!("{ptr:?}"),
        });
    }

    // resolve pointer
    let sp = ptr.as_stack_pointer().unwrap();
    store_stack_slot(state, sp, sp.slot_offset, val)
}

/// Store a value through a local pointer.
#[inline(always)]
pub(crate) fn store_to_local_pointer(
    state: &mut ThreadedState<'_, '_>,
    ptr: Value,
    val: Value,
) -> Result<(), Error> {
    // validate pointer tag
    if ptr.tag() != ValueTag::LocalPointer {
        return Err(Error::InvalidPointerType {
            actual: format!("{ptr:?}"),
        });
    }

    // resolve pointer
    let lp = ptr.as_local_pointer().unwrap();
    store_local_slot(state, lp, lp.slot_offset, val)
}

/// Store a value through a global pointer.
#[inline(always)]
pub(crate) fn store_to_global_pointer(
    state: &mut ThreadedState<'_, '_>,
    ptr: Value,
    val: Value,
) -> Result<(), Error> {
    // validate pointer tag
    if ptr.tag() != ValueTag::GlobalPointer {
        return Err(Error::InvalidPointerType {
            actual: format!("{ptr:?}"),
        });
    }

    // resolve pointer
    let global = ptr.as_global_pointer().unwrap();
    let global_def = state.interpreter.isolate.image.tree.get(global.id);
    if !global_def.is_mutable() {
        return Err(Error::ImmutableGlobalWrite { global: global.id });
    }
    store_global_slot(state, global, val)
}

/// Validate a field index against a known field count.
#[inline(always)]
fn check_field_index(
    state: &ThreadedState<'_, '_>,
    index: u32,
    field_count: u32,
) -> Result<(), Error> {
    // skip checks when bounds are disabled
    if !state.bounds_checks {
        return Ok(());
    }

    // skip checks when the layout is unknown
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
fn check_array_index(
    state: &ThreadedState<'_, '_>,
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

fn managed_projection_type(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
) -> mir::LocalNodeId<mir::Type> {
    match tree.get(ty) {
        mir::Type::Struct { fields, .. } => fields
            .first()
            .map(|field| tree.get(*field).ty)
            .unwrap_or(ty),
        mir::Type::Tuple { elements, .. } => elements.first().copied().unwrap_or(ty),
        mir::Type::Array { element, .. } => *element,
        mir::Type::Newtype { inner, .. } => managed_projection_type(tree, *inner),
        _ => ty,
    }
}

/// Resolve one raw struct or tuple field type and byte offset.
fn raw_field_info(
    tree: &mir::NodeTree,
    aggregate_type: mir::LocalNodeId<mir::Type>,
    index: u32,
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
                .iter()
                .find(|field| field.source_index == Some(index))
                .or_else(|| layout.fields.get(index as usize))
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
        mir::Type::Newtype { inner, .. } => raw_field_info(tree, *inner, index),
        _ => Err(Error::TypeMismatch {
            expected: "raw aggregate".to_string(),
            actual: format!("{aggregate_type:?}"),
        }),
    }
}

fn managed_field_info(
    tree: &mir::NodeTree,
    aggregate_type: mir::LocalNodeId<mir::Type>,
    index: u32,
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
                .iter()
                .find(|field| field.source_index == Some(index))
                .or_else(|| layout.fields.get(index as usize))
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
        mir::Type::Newtype { inner, .. } => managed_field_info(tree, *inner, index),
        _ => Err(Error::TypeMismatch {
            expected: "managed aggregate".to_string(),
            actual: format!("{aggregate_type:?}"),
        }),
    }
}

/// Resolve one raw array element type and byte stride.
fn raw_element_info(
    tree: &mir::NodeTree,
    array_type: mir::LocalNodeId<mir::Type>,
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
                    expected: "raw array layout".to_string(),
                    actual: format!("{array_type:?}"),
                });
            };
            debug_assert_eq!(*element, *element_type);
            Ok((*element_type, *element_stride as usize))
        }
        mir::Type::Newtype { inner, .. } => raw_element_info(tree, *inner),
        _ => Err(Error::TypeMismatch {
            expected: "raw array".to_string(),
            actual: format!("{array_type:?}"),
        }),
    }
}

fn managed_element_info(
    tree: &mir::NodeTree,
    array_type: mir::LocalNodeId<mir::Type>,
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
                    expected: "managed array layout".to_string(),
                    actual: format!("{array_type:?}"),
                });
            };
            debug_assert_eq!(*element, *element_type);
            Ok((*element_type, *element_stride as usize))
        }
        mir::Type::Newtype { inner, .. } => managed_element_info(tree, *inner),
        _ => Err(Error::TypeMismatch {
            expected: "managed array".to_string(),
            actual: format!("{array_type:?}"),
        }),
    }
}

/// Read one raw byte window into an owned buffer.
fn read_raw_bytes(
    state: &ThreadedState<'_, '_>,
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
    state: &mut ThreadedState<'_, '_>,
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
        mir::Type::Reference {
            kind,
            address_space,
            mutability,
            is_nullable,
            ..
        } => {
            let mut raw = [0u8; 8];
            raw[..bytes.len()].copy_from_slice(bytes);
            let raw = u64::from_le_bytes(raw);
            let meta = ReferenceMeta::new(*kind, *address_space, *mutability, *is_nullable);

            match (*kind, *address_space) {
                (mir::ReferenceKind::Managed, _) => Ok(Value::managed_reference_with_meta(
                    ManagedReference::from_bits(raw),
                    meta,
                )),
                (_, mir::AddressSpace::Shared) => Ok(Value::shared_pointer_with_meta(
                    SharedPointer::from_bits(raw),
                    meta,
                )),
                _ => Ok(Value::raw_pointer_with_meta(
                    RawPointer::from_bits(raw),
                    meta,
                )),
            }
        }
        mir::Type::FunctionPointer { .. } => {
            let mut raw = [0u8; 8];
            raw[..bytes.len()].copy_from_slice(bytes);
            Ok(Value::function_pointer(mir::LocalNodeId::new(
                u64::from_le_bytes(raw) as u32,
            )))
        }
        mir::Type::Newtype { inner, .. } => decode_raw_value(tree, *inner, bytes),
        _ => Err(Error::TypeMismatch {
            expected: "scalar or reference raw load".to_string(),
            actual: format!("{ty:?}"),
        }),
    }
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

/// Load one typed value from raw heap bytes.
pub(crate) fn load_from_raw_pointer_typed(
    state: &mut ThreadedState<'_, '_>,
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

    let byte_len = raw_type_size(&state.interpreter.isolate.image.tree, pointee)?;
    let bytes = read_raw_bytes(state, pointer, 0, byte_len)?;
    decode_raw_value(&state.interpreter.isolate.image.tree, pointee, &bytes)
}

/// Store one typed value into raw heap bytes.
pub(crate) fn store_to_raw_pointer_typed(
    state: &mut ThreadedState<'_, '_>,
    ptr: Value,
    pointee: mir::LocalNodeId<mir::Type>,
    value: Value,
) -> Result<(), Error> {
    if ptr.tag() != ValueTag::RawPointer {
        return Err(Error::InvalidPointerType {
            actual: format!("{ptr:?}"),
        });
    }

    let pointer = ptr.as_raw_pointer().unwrap();
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let bytes = encode_raw_value(&state.interpreter.isolate.image.tree, pointee, value)?;
    write_raw_bytes(state, pointer, 0, &bytes)
}

/// Get the address of a field from an aggregate or pointer.
#[inline(always)]
pub(crate) fn field_addr(
    state: &mut ThreadedState<'_, '_>,
    aggregate: Value,
    index: u32,
    field_count: u32,
) -> Result<Value, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    // resolve the source and compute the field pointer
    match aggregate.tag() {
        ValueTag::String => {
            let handle = aggregate.as_managed_reference().unwrap();
            let byte_offset = string_field_offset(index)?;
            let byte_offset =
                u32::try_from(byte_offset).map_err(|_| Error::InvalidFieldAccess {
                    index,
                    field_count: StringLayout::FIELD_COUNT,
                })?;
            let handle = ManagedReference::with_byte_offset(handle.id(), byte_offset);
            Ok(Value::managed_reference(handle))
        }
        ValueTag::Aggregate | ValueTag::ManagedReference => {
            let handle = aggregate.as_managed_reference().unwrap();
            let slot_index = resolve_heap_field_slot(state, handle, index, field_count)?;
            let byte_offset = slot_index
                .checked_mul(Value::BYTE_LEN as u32)
                .ok_or(Error::InvalidManagedReference)?;
            let handle = ManagedReference::with_byte_offset(handle.id(), byte_offset);
            Ok(Value::managed_reference(handle))
        }
        ValueTag::RawPointer => Err(Error::InvalidPointerType {
            actual: "raw pointer requires typed field access".to_string(),
        }),
        ValueTag::StackPointer => {
            let sp = aggregate.as_stack_pointer().unwrap();
            let slot_index = resolve_stack_field_slot(state, sp, index, field_count)?;
            Ok(Value::stack_pointer(StackPointer::with_offset(
                sp.frame_idx,
                sp.slot,
                slot_index,
            )))
        }
        ValueTag::LocalPointer => {
            let pointer = aggregate.as_local_pointer().unwrap();
            field_addr_local(state, pointer, index, field_count)
        }
        ValueTag::GlobalPointer => {
            let global = aggregate.as_global_pointer().unwrap();
            let slot_index = resolve_global_field_slot(state, global, index, field_count)?;
            Ok(Value::global_pointer_with_offset(global.id, slot_index))
        }
        _ => Err(Error::TypeMismatch {
            expected: "aggregate or pointer".to_string(),
            actual: format!("{aggregate:?}"),
        }),
    }
}

/// Get the address of a field from a managed reference.
#[inline(always)]
pub(crate) fn field_addr_managed(
    state: &mut ThreadedState<'_, '_>,
    handle: ManagedReference,
    pointee: mir::LocalNodeId<mir::Type>,
    index: u32,
    field_count: u32,
) -> Result<Value, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    let heap = state.heap_ref();
    if !heap.is_managed_allocated(handle) {
        return Err(Error::InvalidManagedReference);
    }

    if has_packed_values(heap, handle) {
        let slot_index = resolve_heap_field_slot(state, handle, index, field_count)?;
        let byte_offset = slot_index
            .checked_mul(Value::BYTE_LEN as u32)
            .ok_or(Error::InvalidManagedReference)?;
        return Ok(Value::managed_reference(
            ManagedReference::with_byte_offset(handle.id(), byte_offset),
        ));
    }

    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let (_, field_offset) =
        managed_field_info(&state.interpreter.isolate.image.tree, pointee, index)?;
    let byte_offset =
        handle
            .byte_offset()
            .checked_add(field_offset)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: field_count as usize,
            })?;

    Ok(Value::managed_reference(
        ManagedReference::with_byte_offset(handle.id(), byte_offset as u32),
    ))
}

/// Get the address of a field from a raw pointer.
#[inline(always)]
pub(crate) fn field_addr_raw(
    state: &mut ThreadedState<'_, '_>,
    pointer: RawPointer,
    pointee: mir::LocalNodeId<mir::Type>,
    index: u32,
    field_count: u32,
) -> Result<Value, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // resolve byte offset
    let (_, field_offset) = raw_field_info(&state.interpreter.isolate.image.tree, pointee, index)?;
    let byte_offset =
        pointer
            .byte_offset()
            .checked_add(field_offset)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: field_count as usize,
            })?;
    Ok(Value::raw_pointer(RawPointer::with_byte_offset(
        pointer.id(),
        byte_offset as u32,
    )))
}

/// Get the address of a field from a stack pointer.
#[inline(always)]
pub(crate) fn field_addr_stack(
    state: &mut ThreadedState<'_, '_>,
    pointer: StackPointer,
    index: u32,
    field_count: u32,
) -> Result<Value, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    // resolve slot offset
    let slot_index = resolve_stack_field_slot(state, pointer, index, field_count)?;
    Ok(Value::stack_pointer(StackPointer::with_offset(
        pointer.frame_idx,
        pointer.slot,
        slot_index,
    )))
}

/// Get the address of a field from a local pointer.
#[inline(always)]
pub(crate) fn field_addr_local(
    state: &mut ThreadedState<'_, '_>,
    pointer: LocalPointer,
    index: u32,
    field_count: u32,
) -> Result<Value, Error> {
    // load the local value
    let value = load_local_slot(state, pointer, pointer.slot_offset)?;

    // resolve the field address from the value
    field_addr(state, value, index, field_count)
}

/// Get the address of a field from a global pointer.
#[inline(always)]
pub(crate) fn field_addr_global(
    state: &mut ThreadedState<'_, '_>,
    pointer: GlobalPointer,
    index: u32,
    field_count: u32,
) -> Result<Value, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    // load the global value
    let value = state
        .interpreter
        .isolate
        .globals
        .get(pointer.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: pointer.id })?;

    // project string fields directly into managed header bytes
    if value.tag() == ValueTag::String {
        let handle = value.as_managed_reference().unwrap();
        let field_offset = string_field_offset(index)?;
        let field_offset = u32::try_from(field_offset).map_err(|_| Error::InvalidFieldAccess {
            index,
            field_count: StringLayout::FIELD_COUNT,
        })?;

        return Ok(Value::managed_reference(
            ManagedReference::with_byte_offset(handle.id(), field_offset),
        ));
    }

    // resolve aggregate slot offset
    let slot_index = resolve_global_field_slot(state, pointer, index, field_count)?;
    Ok(Value::global_pointer_with_offset(pointer.id, slot_index))
}

/// Get the address of an element from an array or pointer.
#[inline(always)]
pub(crate) fn element_addr(
    state: &mut ThreadedState<'_, '_>,
    array: Value,
    index: u64,
    array_length: u64,
) -> Result<Value, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    // resolve the source and compute the element pointer
    match array.tag() {
        ValueTag::Aggregate | ValueTag::ManagedReference => {
            let handle = array.as_managed_reference().unwrap();
            let slot_index = resolve_heap_element_slot(state, handle, index, array_length)?;
            let byte_offset = slot_index
                .checked_mul(Value::BYTE_LEN as u32)
                .ok_or(Error::InvalidManagedReference)?;
            let handle = ManagedReference::with_byte_offset(handle.id(), byte_offset);
            Ok(Value::managed_reference(handle))
        }
        ValueTag::RawPointer => Err(Error::InvalidPointerType {
            actual: "raw pointer requires typed element access".to_string(),
        }),
        ValueTag::StackPointer => {
            let sp = array.as_stack_pointer().unwrap();
            let slot_index = resolve_stack_element_slot(state, sp, index, array_length)?;
            Ok(Value::stack_pointer(StackPointer::with_offset(
                sp.frame_idx,
                sp.slot,
                slot_index,
            )))
        }
        ValueTag::LocalPointer => {
            let pointer = array.as_local_pointer().unwrap();
            element_addr_local(state, pointer, index, array_length)
        }
        ValueTag::GlobalPointer => {
            let global = array.as_global_pointer().unwrap();
            let slot_index = resolve_global_element_slot(state, global, index, array_length)?;
            Ok(Value::global_pointer_with_offset(global.id, slot_index))
        }
        _ => Err(Error::TypeMismatch {
            expected: "array or pointer".to_string(),
            actual: format!("{array:?}"),
        }),
    }
}

/// Get the address of an element from a managed reference.
#[inline(always)]
pub(crate) fn element_addr_managed(
    state: &mut ThreadedState<'_, '_>,
    handle: ManagedReference,
    pointee: mir::LocalNodeId<mir::Type>,
    index: u64,
    array_length: u64,
) -> Result<Value, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    let heap = state.heap_ref();
    if !heap.is_managed_allocated(handle) {
        return Err(Error::InvalidManagedReference);
    }

    if has_packed_values(heap, handle) {
        let slot_index = resolve_heap_element_slot(state, handle, index, array_length)?;
        let byte_offset = slot_index
            .checked_mul(Value::BYTE_LEN as u32)
            .ok_or(Error::InvalidManagedReference)?;
        return Ok(Value::managed_reference(
            ManagedReference::with_byte_offset(handle.id(), byte_offset),
        ));
    }

    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let (_, element_stride) = managed_element_info(&state.interpreter.isolate.image.tree, pointee)?;
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length,
        })?;
    let byte_offset =
        handle
            .byte_offset()
            .checked_add(element_offset)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: array_length,
            })?;

    Ok(Value::managed_reference(
        ManagedReference::with_byte_offset(handle.id(), byte_offset as u32),
    ))
}

/// Get the address of an element from a raw pointer.
#[inline(always)]
pub(crate) fn element_addr_raw(
    state: &mut ThreadedState<'_, '_>,
    pointer: RawPointer,
    pointee: mir::LocalNodeId<mir::Type>,
    index: u64,
    array_length: u64,
) -> Result<Value, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // resolve byte offset
    let (_, element_stride) = raw_element_info(&state.interpreter.isolate.image.tree, pointee)?;
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length,
        })?;
    let byte_offset =
        pointer
            .byte_offset()
            .checked_add(element_offset)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: array_length,
            })?;
    Ok(Value::raw_pointer(RawPointer::with_byte_offset(
        pointer.id(),
        byte_offset as u32,
    )))
}

/// Get the address of an element from a stack pointer.
#[inline(always)]
pub(crate) fn element_addr_stack(
    state: &mut ThreadedState<'_, '_>,
    pointer: StackPointer,
    index: u64,
    array_length: u64,
) -> Result<Value, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    // resolve slot offset
    let slot_index = resolve_stack_element_slot(state, pointer, index, array_length)?;
    Ok(Value::stack_pointer(StackPointer::with_offset(
        pointer.frame_idx,
        pointer.slot,
        slot_index as usize,
    )))
}

/// Get the address of an element from a local pointer.
#[inline(always)]
pub(crate) fn element_addr_local(
    state: &mut ThreadedState<'_, '_>,
    pointer: LocalPointer,
    index: u64,
    array_length: u64,
) -> Result<Value, Error> {
    // load the local value
    let value = load_local_slot(state, pointer, pointer.slot_offset)?;

    // resolve the element address from the value
    element_addr(state, value, index, array_length)
}

/// Get the address of an element from a global pointer.
#[inline(always)]
pub(crate) fn element_addr_global(
    state: &mut ThreadedState<'_, '_>,
    pointer: GlobalPointer,
    index: u64,
    array_length: u64,
) -> Result<Value, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    // resolve slot offset
    let slot_index = resolve_global_element_slot(state, pointer, index, array_length)?;
    Ok(Value::global_pointer_with_offset(
        pointer.id,
        slot_index as usize,
    ))
}

/// Load a field from a managed heap allocation.
#[inline(always)]
pub(crate) fn load_field_managed(
    state: &mut ThreadedState<'_, '_>,
    handle: ManagedReference,
    pointee: mir::LocalNodeId<mir::Type>,
    index: u32,
    field_count: u32,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.interpreter.engine.statistics, loads);
    }

    let heap = state.heap_ref();
    if !heap.is_managed_allocated(handle) {
        return Err(Error::InvalidManagedReference);
    }

    if has_packed_values(heap, handle) {
        // validate field index when known
        check_field_index(state, index, field_count)?;

        if state.null_checks && handle.is_null() {
            return Err(Error::NullPointerDereference);
        }

        let cell_len = packed_value_count(heap, handle)?;
        let value_index = handle.byte_offset() / Value::BYTE_LEN + index as usize;
        return heap
            .packed_value_at(handle, value_index)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: cell_len,
            });
    }

    check_field_index(state, index, field_count)?;

    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let (field_type, field_offset) =
        managed_field_info(&state.interpreter.isolate.image.tree, pointee, index)?;
    let field_type = managed_projection_type(&state.interpreter.isolate.image.tree, field_type);
    let byte_len = managed_type_size(&state.interpreter.isolate.image.tree, field_type)?;
    let bytes = heap
        .managed_bytes(handle)
        .ok_or(Error::InvalidManagedReference)?;
    let start = field_offset;
    let end = start
        .checked_add(byte_len)
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: bytes.len(),
        })?;
    let window = bytes.get(start..end).ok_or(Error::InvalidFieldAccess {
        index,
        field_count: bytes.len(),
    })?;

    decode_raw_value(&state.interpreter.isolate.image.tree, field_type, window)
}

/// Store a field into a managed heap allocation.
#[inline(always)]
pub(crate) fn store_field_managed(
    state: &mut ThreadedState<'_, '_>,
    handle: ManagedReference,
    pointee: mir::LocalNodeId<mir::Type>,
    index: u32,
    field_count: u32,
    value: Value,
) -> Result<(), Error> {
    let bounds_checks = state.bounds_checks;
    let null_checks = state.null_checks;
    let tree = &state.interpreter.isolate.image.tree;

    // track pointer stores
    if state.collect_stats {
        stat_inc!(state.interpreter.engine.statistics, stores);
    }

    let is_value_array = {
        let heap = state.heap_ref();
        if !heap.is_managed_allocated(handle) {
            return Err(Error::InvalidManagedReference);
        }
        has_packed_values(heap, handle)
    };

    if is_value_array {
        check_field_index(state, index, field_count)?;

        if null_checks && handle.is_null() {
            return Err(Error::NullPointerDereference);
        }

        let heap = state.heap();
        let cell_len = packed_value_count(heap, handle)?;
        let value_index = handle.byte_offset() / Value::BYTE_LEN + index as usize;
        return if heap.set_packed_value(handle, value_index, value) {
            Ok(())
        } else {
            Err(Error::InvalidFieldAccess {
                index,
                field_count: cell_len,
            })
        };
    }

    check_field_index(state, index, field_count)?;

    if null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let (field_type, field_offset) = managed_field_info(tree, pointee, index)?;
    let field_type = managed_projection_type(tree, field_type);
    let bytes = encode_raw_value(tree, field_type, value)?;
    let heap = state.heap();
    let byte_len = heap
        .managed_byte_len(handle)
        .ok_or(Error::InvalidManagedReference)?;
    let start = field_offset;
    let end = start
        .checked_add(bytes.len())
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: byte_len,
        })?;

    if bounds_checks && end > byte_len {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: byte_len,
        });
    }

    for (offset, byte) in bytes.into_iter().enumerate() {
        if !heap.set_managed_byte(handle, start + offset, byte) {
            return Err(Error::InvalidFieldAccess {
                index,
                field_count: byte_len,
            });
        }
    }

    Ok(())
}

/// Load a field from a raw heap allocation.
#[inline(always)]
pub(crate) fn load_field_raw(
    state: &mut ThreadedState<'_, '_>,
    pointer: RawPointer,
    pointee: mir::LocalNodeId<mir::Type>,
    index: u32,
    field_count: u32,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.interpreter.engine.statistics, loads);
    }

    // validate field index when known
    check_field_index(state, index, field_count)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let (field_type, field_offset) =
        raw_field_info(&state.interpreter.isolate.image.tree, pointee, index)?;
    let byte_offset =
        pointer
            .byte_offset()
            .checked_add(field_offset)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: field_count as usize,
            })?;
    let pointer = Value::raw_pointer(RawPointer::with_byte_offset(
        pointer.id(),
        byte_offset as u32,
    ));
    load_from_raw_pointer_typed(state, pointer, field_type)
}

/// Store a field into a raw heap allocation.
#[inline(always)]
pub(crate) fn store_field_raw(
    state: &mut ThreadedState<'_, '_>,
    pointer: RawPointer,
    pointee: mir::LocalNodeId<mir::Type>,
    index: u32,
    field_count: u32,
    value: Value,
) -> Result<(), Error> {
    // track pointer stores
    if state.collect_stats {
        stat_inc!(state.interpreter.engine.statistics, stores);
    }

    // validate field index when known
    check_field_index(state, index, field_count)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let (field_type, field_offset) =
        raw_field_info(&state.interpreter.isolate.image.tree, pointee, index)?;
    let byte_offset =
        pointer
            .byte_offset()
            .checked_add(field_offset)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: field_count as usize,
            })?;
    let pointer = Value::raw_pointer(RawPointer::with_byte_offset(
        pointer.id(),
        byte_offset as u32,
    ));
    store_to_raw_pointer_typed(state, pointer, field_type, value)
}

/// Load a field from a stack allocation.
#[inline(always)]
pub(crate) fn load_field_stack(
    state: &mut ThreadedState<'_, '_>,
    pointer: StackPointer,
    index: u32,
    field_count: u32,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.interpreter.engine.statistics, loads);
    }

    // validate field index when known
    check_field_index(state, index, field_count)?;

    // look up the stack buffer
    let frame = state.frame_by_index(pointer.frame_idx)?;
    let cell = frame
        .stack_buffer(pointer.slot)
        .ok_or(Error::InvalidManagedReference)?;
    let cell_len = cell.len();

    // select field count for diagnostics
    let field_count_for_error = if field_count == UNKNOWN_FIELD_COUNT {
        cell_len
    } else {
        field_count as usize
    };

    // compute the absolute slot offset
    let slot_index = if state.bounds_checks {
        pointer
            .slot_offset
            .checked_add(index as usize)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: field_count_for_error,
            })?
    } else {
        pointer.slot_offset.wrapping_add(index as usize)
    };

    // validate bounds when field count is unknown
    if state.bounds_checks
        && field_count == UNKNOWN_FIELD_COUNT
        && !cell.is_empty()
        && slot_index >= cell.len()
    {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: cell.len(),
        });
    }

    // treat empty slot 0 as void
    if cell_len == 0 && slot_index == 0 {
        return Ok(Value::VOID);
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell.len(), "stack field out of bounds");
        let value = unsafe { *cell.get_unchecked(slot_index) };
        return Ok(value);
    }

    // read the slot when in bounds
    if let Some(value) = cell.get(slot_index).copied() {
        return Ok(value);
    }

    Err(Error::InvalidFieldAccess {
        index,
        field_count: cell.len(),
    })
}

/// Store a field into a stack allocation.
#[inline(always)]
pub(crate) fn store_field_stack(
    state: &mut ThreadedState<'_, '_>,
    pointer: StackPointer,
    index: u32,
    field_count: u32,
    value: Value,
) -> Result<(), Error> {
    // cache bounds checks setting
    let bounds_checks = state.bounds_checks;

    // track pointer stores
    if state.collect_stats {
        stat_inc!(state.interpreter.engine.statistics, stores);
    }

    // validate field index when known
    check_field_index(state, index, field_count)?;

    // look up the stack buffer
    let frame = state.frame_by_index_mut(pointer.frame_idx)?;
    let cell = frame
        .stack_buffer_mut(pointer.slot)
        .ok_or(Error::InvalidManagedReference)?;
    let cell_len = cell.len();

    // select field count for diagnostics
    let field_count_for_error = if field_count == UNKNOWN_FIELD_COUNT {
        cell_len
    } else {
        field_count as usize
    };

    // compute the absolute slot offset
    let slot_index = if bounds_checks {
        pointer
            .slot_offset
            .checked_add(index as usize)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: field_count_for_error,
            })?
    } else {
        pointer.slot_offset.wrapping_add(index as usize)
    };

    // validate bounds when field count is unknown
    if bounds_checks
        && field_count == UNKNOWN_FIELD_COUNT
        && !cell.is_empty()
        && slot_index >= cell.len()
    {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: cell.len(),
        });
    }

    // fast path without bounds checks
    if !bounds_checks {
        debug_assert!(slot_index < cell.len(), "stack field out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        unsafe {
            *cell.get_unchecked_mut(slot_index) = value;
        }
        return Ok(());
    }

    // write the slot when in bounds
    if let Some(slot) = cell.get_mut(slot_index) {
        *slot = value;
        return Ok(());
    }

    Err(Error::InvalidFieldAccess {
        index,
        field_count: cell.len(),
    })
}

/// Load a field from a global allocation.
#[inline(always)]
pub(crate) fn load_field_global(
    state: &mut ThreadedState<'_, '_>,
    pointer: GlobalPointer,
    index: u32,
    field_count: u32,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.interpreter.engine.statistics, loads);
    }

    // validate field index when known
    check_field_index(state, index, field_count)?;

    // load the global value
    let value = state
        .interpreter
        .isolate
        .globals
        .get(pointer.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: pointer.id })?;

    // require aggregate payload
    if !matches!(value.tag(), ValueTag::Aggregate | ValueTag::String) {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: 0,
        });
    }

    if value.tag() == ValueTag::String {
        let handle = value.as_managed_reference().unwrap();
        return load_string_field(state.heap_ref(), handle, index);
    }

    let handle = value.as_managed_reference().unwrap();

    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed allocation
    let heap = state.heap_ref();
    let cell_len = packed_value_count(heap, handle)?;

    // select field count for diagnostics
    let field_count_for_error = if field_count == UNKNOWN_FIELD_COUNT {
        cell_len
    } else {
        field_count as usize
    };

    // compute the absolute slot offset
    let slot_index = if state.bounds_checks {
        pointer
            .slot_offset
            .checked_add(index as usize)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: field_count_for_error,
            })?
    } else {
        pointer.slot_offset.wrapping_add(index as usize)
    };

    // validate bounds when field count is unknown
    if state.bounds_checks
        && field_count == UNKNOWN_FIELD_COUNT
        && cell_len != 0
        && slot_index >= cell_len
    {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: cell_len,
        });
    }

    // treat empty slot 0 as void
    if cell_len == 0 && slot_index == 0 {
        return Ok(Value::VOID);
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell_len, "global field out of bounds");
        let value = load_packed_value(heap, handle, slot_index)?;
        return Ok(value);
    }

    // read the slot when in bounds
    if let Some(value) = heap.packed_value_at(handle, slot_index) {
        return Ok(value);
    }

    Err(Error::InvalidFieldAccess {
        index,
        field_count: cell_len,
    })
}

/// Store a field into a global allocation.
#[inline(always)]
pub(crate) fn store_field_global(
    state: &mut ThreadedState<'_, '_>,
    pointer: GlobalPointer,
    index: u32,
    field_count: u32,
    value: Value,
) -> Result<(), Error> {
    let bounds_checks = state.bounds_checks;
    let null_checks = state.null_checks;

    // track pointer stores
    if state.collect_stats {
        stat_inc!(state.interpreter.engine.statistics, stores);
    }

    // validate field index when known
    check_field_index(state, index, field_count)?;

    // load the global value
    let current = state
        .interpreter
        .isolate
        .globals
        .get(pointer.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: pointer.id })?;

    // require aggregate payload
    if !matches!(current.tag(), ValueTag::Aggregate | ValueTag::String) {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: 0,
        });
    }

    if current.tag() == ValueTag::String {
        let handle = current.as_managed_reference().unwrap();
        store_string_field(state.heap(), handle, index, value)?;
        return Ok(());
    }

    let handle = current.as_managed_reference().unwrap();

    // reject null handles when enabled
    if null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    {
        // look up the managed heap allocation
        let heap = state.heap();
        let allocation_len = packed_value_count(heap, handle)?;

        // select field count for diagnostics
        let field_count_for_error = if field_count == UNKNOWN_FIELD_COUNT {
            allocation_len
        } else {
            field_count as usize
        };

        // compute the absolute slot offset
        let slot_index = if bounds_checks {
            pointer
                .slot_offset
                .checked_add(index as usize)
                .ok_or(Error::InvalidFieldAccess {
                    index,
                    field_count: field_count_for_error,
                })?
        } else {
            pointer.slot_offset.wrapping_add(index as usize)
        };

        // validate bounds when field count is unknown
        if bounds_checks
            && field_count == UNKNOWN_FIELD_COUNT
            && allocation_len != 0
            && slot_index >= allocation_len
        {
            return Err(Error::InvalidFieldAccess {
                index,
                field_count: allocation_len,
            });
        }

        // fast path without bounds checks
        if !bounds_checks {
            debug_assert!(slot_index < allocation_len, "global field out of bounds");
            // #Safety: bounds checks are disabled and slot is trusted
            store_packed_value(heap, handle, slot_index, value)?;
        } else if heap.set_packed_value(handle, slot_index, value) {
            // write the slot when in bounds
        } else {
            return Err(Error::InvalidFieldAccess {
                index,
                field_count: allocation_len,
            });
        }
    }

    state.interpreter.isolate.globals.set(pointer.id, current);
    Ok(())
}

/// Load an element from a managed heap allocation.
#[inline(always)]
pub(crate) fn load_element_managed(
    state: &mut ThreadedState<'_, '_>,
    handle: ManagedReference,
    pointee: mir::LocalNodeId<mir::Type>,
    index: u64,
    array_length: u64,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.interpreter.engine.statistics, loads);
    }

    let heap = state.heap_ref();
    if !heap.is_managed_allocated(handle) {
        return Err(Error::InvalidManagedReference);
    }

    if has_packed_values(heap, handle) {
        check_array_index(state, index, array_length)?;

        if state.null_checks && handle.is_null() {
            return Err(Error::NullPointerDereference);
        }

        let cell_len = packed_value_count(heap, handle)?;
        let value_index = handle.byte_offset() / Value::BYTE_LEN + index as usize;
        return heap
            .packed_value_at(handle, value_index)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: cell_len as u64,
            });
    }

    check_array_index(state, index, array_length)?;

    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let (element_type, element_stride) =
        managed_element_info(&state.interpreter.isolate.image.tree, pointee)?;
    let element_type = managed_projection_type(&state.interpreter.isolate.image.tree, element_type);
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length,
        })?;
    let byte_len = managed_type_size(&state.interpreter.isolate.image.tree, element_type)?;
    let bytes = heap
        .managed_bytes(handle)
        .ok_or(Error::InvalidManagedReference)?;
    let start = element_offset;
    let end = start
        .checked_add(byte_len)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: bytes.len() as u64,
        })?;
    let window = bytes.get(start..end).ok_or(Error::InvalidArrayAccess {
        index,
        length: bytes.len() as u64,
    })?;

    decode_raw_value(&state.interpreter.isolate.image.tree, element_type, window)
}

/// Store an element into a managed heap allocation.
#[inline(always)]
pub(crate) fn store_element_managed(
    state: &mut ThreadedState<'_, '_>,
    handle: ManagedReference,
    pointee: mir::LocalNodeId<mir::Type>,
    index: u64,
    array_length: u64,
    value: Value,
) -> Result<(), Error> {
    let bounds_checks = state.bounds_checks;
    let null_checks = state.null_checks;
    let tree = &state.interpreter.isolate.image.tree;

    // track pointer stores
    if state.collect_stats {
        stat_inc!(state.interpreter.engine.statistics, stores);
    }

    let is_value_array = {
        let heap = state.heap_ref();
        if !heap.is_managed_allocated(handle) {
            return Err(Error::InvalidManagedReference);
        }
        has_packed_values(heap, handle)
    };

    if is_value_array {
        check_array_index(state, index, array_length)?;

        if null_checks && handle.is_null() {
            return Err(Error::NullPointerDereference);
        }

        let heap = state.heap();
        let allocation_len = packed_value_count(heap, handle)?;
        let value_index = handle.byte_offset() / Value::BYTE_LEN + index as usize;
        return if heap.set_packed_value(handle, value_index, value) {
            Ok(())
        } else {
            Err(Error::InvalidArrayAccess {
                index,
                length: allocation_len as u64,
            })
        };
    }

    check_array_index(state, index, array_length)?;

    if null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let (element_type, element_stride) = managed_element_info(tree, pointee)?;
    let element_type = managed_projection_type(tree, element_type);
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length,
        })?;
    let bytes = encode_raw_value(tree, element_type, value)?;
    let heap = state.heap();
    let allocation_len = heap
        .managed_byte_len(handle)
        .ok_or(Error::InvalidManagedReference)?;
    let start = element_offset;
    let end = start
        .checked_add(bytes.len())
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: allocation_len as u64,
        })?;

    if bounds_checks && end > allocation_len {
        return Err(Error::InvalidArrayAccess {
            index,
            length: allocation_len as u64,
        });
    }

    for (offset, byte) in bytes.into_iter().enumerate() {
        if !heap.set_managed_byte(handle, start + offset, byte) {
            return Err(Error::InvalidArrayAccess {
                index,
                length: allocation_len as u64,
            });
        }
    }

    Ok(())
}

/// Load an element from a raw heap allocation.
#[inline(always)]
pub(crate) fn load_element_raw(
    state: &mut ThreadedState<'_, '_>,
    pointer: RawPointer,
    pointee: mir::LocalNodeId<mir::Type>,
    index: u64,
    array_length: u64,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.interpreter.engine.statistics, loads);
    }

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let (element_type, element_stride) =
        raw_element_info(&state.interpreter.isolate.image.tree, pointee)?;
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length,
        })?;
    let byte_offset =
        pointer
            .byte_offset()
            .checked_add(element_offset)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: array_length,
            })?;
    let pointer = Value::raw_pointer(RawPointer::with_byte_offset(
        pointer.id(),
        byte_offset as u32,
    ));
    load_from_raw_pointer_typed(state, pointer, element_type)
}

/// Store an element into a raw heap allocation.
#[inline(always)]
pub(crate) fn store_element_raw(
    state: &mut ThreadedState<'_, '_>,
    pointer: RawPointer,
    pointee: mir::LocalNodeId<mir::Type>,
    index: u64,
    array_length: u64,
    value: Value,
) -> Result<(), Error> {
    // track pointer stores
    if state.collect_stats {
        stat_inc!(state.interpreter.engine.statistics, stores);
    }

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let (element_type, element_stride) =
        raw_element_info(&state.interpreter.isolate.image.tree, pointee)?;
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length,
        })?;
    let byte_offset =
        pointer
            .byte_offset()
            .checked_add(element_offset)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: array_length,
            })?;
    let pointer = Value::raw_pointer(RawPointer::with_byte_offset(
        pointer.id(),
        byte_offset as u32,
    ));
    store_to_raw_pointer_typed(state, pointer, element_type, value)
}

/// Load an element from a stack allocation.
#[inline(always)]
pub(crate) fn load_element_stack(
    state: &mut ThreadedState<'_, '_>,
    pointer: StackPointer,
    index: u64,
    array_length: u64,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.interpreter.engine.statistics, loads);
    }

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // look up the stack buffer
    let frame = state.frame_by_index(pointer.frame_idx)?;
    let cell = frame
        .stack_buffer(pointer.slot)
        .ok_or(Error::InvalidManagedReference)?;
    let cell_len = cell.len();

    // compute the absolute slot offset
    let index_usize = if state.bounds_checks {
        usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell_len as u64,
        })?
    } else {
        index as usize
    };
    let slot_index = if state.bounds_checks {
        pointer
            .slot_offset
            .checked_add(index_usize)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: cell_len as u64,
            })?
    } else {
        pointer.slot_offset.wrapping_add(index_usize)
    };

    // validate bounds when array length is unknown
    if state.bounds_checks && array_length == UNKNOWN_ARRAY_LENGTH && slot_index >= cell_len {
        return Err(Error::InvalidArrayAccess {
            index,
            length: cell_len as u64,
        });
    }

    // treat empty slot 0 as void
    if cell.is_empty() && slot_index == 0 {
        return Ok(Value::VOID);
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell_len, "stack element out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        let value = unsafe { *cell.get_unchecked(slot_index) };
        return Ok(value);
    }

    // read the slot when in bounds
    if let Some(value) = cell.get(slot_index).copied() {
        return Ok(value);
    }

    Err(Error::InvalidArrayAccess {
        index,
        length: cell.len() as u64,
    })
}

/// Store an element into a stack allocation.
#[inline(always)]
pub(crate) fn store_element_stack(
    state: &mut ThreadedState<'_, '_>,
    pointer: StackPointer,
    index: u64,
    array_length: u64,
    value: Value,
) -> Result<(), Error> {
    // cache bounds checks setting
    let bounds_checks = state.bounds_checks;

    // track pointer stores
    if state.collect_stats {
        stat_inc!(state.interpreter.engine.statistics, stores);
    }

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // look up the stack buffer
    let frame = state.frame_by_index_mut(pointer.frame_idx)?;
    let cell = frame
        .stack_buffer_mut(pointer.slot)
        .ok_or(Error::InvalidManagedReference)?;
    let cell_len = cell.len();

    // compute the absolute slot offset
    let index_usize = if bounds_checks {
        usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell_len as u64,
        })?
    } else {
        index as usize
    };
    let slot_index = if bounds_checks {
        pointer
            .slot_offset
            .checked_add(index_usize)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: cell_len as u64,
            })?
    } else {
        pointer.slot_offset.wrapping_add(index_usize)
    };

    // validate bounds when array length is unknown
    if bounds_checks && array_length == UNKNOWN_ARRAY_LENGTH && slot_index >= cell_len {
        return Err(Error::InvalidArrayAccess {
            index,
            length: cell_len as u64,
        });
    }

    // fast path without bounds checks
    if !bounds_checks {
        debug_assert!(slot_index < cell_len, "stack element out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        unsafe {
            *cell.get_unchecked_mut(slot_index) = value;
        }
        return Ok(());
    }

    // write the slot when in bounds
    if let Some(slot) = cell.get_mut(slot_index) {
        *slot = value;
        return Ok(());
    }

    Err(Error::InvalidArrayAccess {
        index,
        length: cell.len() as u64,
    })
}

/// Load an element from a global allocation.
#[inline(always)]
pub(crate) fn load_element_global(
    state: &mut ThreadedState<'_, '_>,
    pointer: GlobalPointer,
    index: u64,
    array_length: u64,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.interpreter.engine.statistics, loads);
    }

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // load the global value
    let value = state
        .interpreter
        .isolate
        .globals
        .get(pointer.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: pointer.id })?;

    // require aggregate payload
    if value.tag() != ValueTag::Aggregate {
        return Err(Error::InvalidArrayAccess { index, length: 0 });
    }

    let handle = value.as_managed_reference().unwrap();

    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed allocation
    let heap = state.heap_ref();
    let cell_len = packed_value_count(heap, handle)?;

    // compute the absolute slot offset
    let index_usize = if state.bounds_checks {
        usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell_len as u64,
        })?
    } else {
        index as usize
    };
    let slot_index = if state.bounds_checks {
        pointer
            .slot_offset
            .checked_add(index_usize)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: cell_len as u64,
            })?
    } else {
        pointer.slot_offset.wrapping_add(index_usize)
    };

    // validate bounds when array length is unknown
    if state.bounds_checks && array_length == UNKNOWN_ARRAY_LENGTH && slot_index >= cell_len {
        return Err(Error::InvalidArrayAccess {
            index,
            length: cell_len as u64,
        });
    }

    // treat empty slot 0 as void
    if cell_len == 0 && slot_index == 0 {
        return Ok(Value::VOID);
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell_len, "global element out of bounds");
        let value = load_packed_value(heap, handle, slot_index)?;
        return Ok(value);
    }

    // read the slot when in bounds
    if let Some(value) = heap.packed_value_at(handle, slot_index) {
        return Ok(value);
    }

    Err(Error::InvalidArrayAccess {
        index,
        length: cell_len as u64,
    })
}

/// Store an element into a global allocation.
#[inline(always)]
pub(crate) fn store_element_global(
    state: &mut ThreadedState<'_, '_>,
    pointer: GlobalPointer,
    index: u64,
    array_length: u64,
    value: Value,
) -> Result<(), Error> {
    let bounds_checks = state.bounds_checks;
    let null_checks = state.null_checks;

    // track pointer stores
    if state.collect_stats {
        stat_inc!(state.interpreter.engine.statistics, stores);
    }

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // load the global value
    let current = state
        .interpreter
        .isolate
        .globals
        .get(pointer.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: pointer.id })?;

    // require aggregate payload
    if current.tag() != ValueTag::Aggregate {
        return Err(Error::InvalidArrayAccess { index, length: 0 });
    }

    let handle = current.as_managed_reference().unwrap();

    // reject null handles when enabled
    if null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    {
        // look up the managed heap allocation
        let heap = state.heap();
        let allocation_len = packed_value_count(heap, handle)?;

        // compute the absolute slot offset
        let index_usize = if bounds_checks {
            usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
                index,
                length: allocation_len as u64,
            })?
        } else {
            index as usize
        };
        let slot_index = if bounds_checks {
            pointer
                .slot_offset
                .checked_add(index_usize)
                .ok_or(Error::InvalidArrayAccess {
                    index,
                    length: allocation_len as u64,
                })?
        } else {
            pointer.slot_offset.wrapping_add(index_usize)
        };

        // validate bounds when array length is unknown
        if bounds_checks && array_length == UNKNOWN_ARRAY_LENGTH && slot_index >= allocation_len {
            return Err(Error::InvalidArrayAccess {
                index,
                length: allocation_len as u64,
            });
        }

        // fast path without bounds checks
        if !bounds_checks {
            debug_assert!(slot_index < allocation_len, "global element out of bounds");
            // #Safety: bounds checks are disabled and slot is trusted
            store_packed_value(heap, handle, slot_index, value)?;
        } else if heap.set_packed_value(handle, slot_index, value) {
            // write the slot when in bounds
        } else {
            return Err(Error::InvalidArrayAccess {
                index,
                length: allocation_len as u64,
            });
        }
    }

    state.interpreter.isolate.globals.set(pointer.id, current);
    Ok(())
}

/// Get a field from an aggregate value.
#[inline(always)]
pub(crate) fn get_field(
    state: &mut ThreadedState<'_, '_>,
    agg: Value,
    index: u32,
) -> Result<Value, Error> {
    // resolve aggregate value
    match agg.tag() {
        ValueTag::String => {
            let handle = agg.as_managed_reference().unwrap();
            load_string_field(state.heap_ref(), handle, index)
        }
        ValueTag::Aggregate => {
            let handle = agg.as_managed_reference().unwrap();
            get_heap_field(state, handle, index)
        }
        _ => Err(Error::TypeMismatch {
            expected: "aggregate".to_string(),
            actual: format!("{agg:?}"),
        }),
    }
}

/// Set a field on an aggregate value.
#[inline(always)]
pub(crate) fn set_field(
    state: &mut ThreadedState<'_, '_>,
    agg: Value,
    index: u32,
    val: Value,
) -> Result<Value, Error> {
    // resolve aggregate value
    match agg.tag() {
        ValueTag::String => {
            let handle = agg.as_managed_reference().unwrap();
            store_string_field(state.heap(), handle, index, val)?;
            Ok(agg)
        }
        ValueTag::Aggregate => {
            let handle = agg.as_managed_reference().unwrap();
            set_heap_field(state, handle, index, val)?;
            Ok(agg)
        }
        _ => Err(Error::TypeMismatch {
            expected: "aggregate".to_string(),
            actual: format!("{agg:?}"),
        }),
    }
}

/// Get an element from an array value.
#[inline(always)]
pub(crate) fn get_element(
    state: &mut ThreadedState<'_, '_>,
    arr: Value,
    index: u64,
) -> Result<Value, Error> {
    // resolve array value
    match arr.tag() {
        ValueTag::Aggregate => {
            let handle = arr.as_managed_reference().unwrap();
            get_heap_element(state, handle, index)
        }
        _ => Err(Error::TypeMismatch {
            expected: "array".to_string(),
            actual: format!("{arr:?}"),
        }),
    }
}

/// Set an element on an array value.
#[inline(always)]
pub(crate) fn set_element(
    state: &mut ThreadedState<'_, '_>,
    arr: Value,
    index: u64,
    val: Value,
) -> Result<Value, Error> {
    // resolve array value
    match arr.tag() {
        ValueTag::Aggregate => {
            let handle = arr.as_managed_reference().unwrap();
            set_heap_element(state, handle, index, val)?;
            Ok(arr)
        }
        _ => Err(Error::TypeMismatch {
            expected: "array".to_string(),
            actual: format!("{arr:?}"),
        }),
    }
}

/// Load a slot from a managed heap allocation.
#[inline(always)]
fn load_heap_slot(
    state: &mut ThreadedState<'_, '_>,
    handle: ManagedReference,
    slot_index: usize,
) -> Result<Value, Error> {
    let bounds_checks = state.bounds_checks;
    let null_checks = state.null_checks;

    // reject null handles when enabled
    if null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed allocation
    let heap = state.heap_ref();
    let cell_len = packed_value_count(heap, handle)?;

    // treat empty slot 0 as void
    if cell_len == 0 && slot_index == 0 {
        return Ok(Value::VOID);
    }

    // fast path without bounds checks
    if !bounds_checks {
        debug_assert!(slot_index < cell_len, "heap slot out of bounds");
        let value = load_packed_value(heap, handle, slot_index)?;
        return Ok(value);
    }

    // read the slot when in bounds
    if let Some(value) = heap.packed_value_at(handle, slot_index) {
        return Ok(value);
    }

    Err(Error::InvalidFieldAccess {
        index: slot_index as u32,
        field_count: cell_len,
    })
}

/// Store a slot into a managed heap allocation.
#[inline(always)]
fn store_heap_slot(
    state: &mut ThreadedState<'_, '_>,
    handle: ManagedReference,
    slot_index: usize,
    value: Value,
) -> Result<(), Error> {
    let bounds_checks = state.bounds_checks;
    let null_checks = state.null_checks;

    // reject null handles when enabled
    if null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed heap allocation
    let heap = state.heap();
    let mut allocation_len = packed_value_count(heap, handle)?;

    // resize slots as needed when bounds checks are enabled
    if bounds_checks && allocation_len <= slot_index {
        if !heap
            .resize_packed_values(handle, slot_index + 1)
            .map_err(Error::from)?
        {
            return Err(Error::InvalidManagedReference);
        }

        allocation_len = packed_value_count(heap, handle)?;
    }

    // fast path without bounds checks
    if !bounds_checks {
        debug_assert!(slot_index < allocation_len, "heap slot out of bounds");
        store_packed_value(heap, handle, slot_index, value)?;
        return Ok(());
    }

    // write slot when in bounds
    if heap.set_packed_value(handle, slot_index, value) {
        return Ok(());
    }

    Err(Error::InvalidFieldAccess {
        index: slot_index as u32,
        field_count: allocation_len,
    })
}

/// Load from a global pointer, including slot offsets.
#[inline(always)]
fn load_global_slot(
    state: &mut ThreadedState<'_, '_>,
    global: GlobalPointer,
) -> Result<Value, Error> {
    // read the global value
    let value = state
        .interpreter
        .isolate
        .globals
        .get(global.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: global.id })?;

    // return the global value directly
    if global.slot_offset == 0 {
        return Ok(value);
    }

    // read from the aggregate stored in the global
    if !matches!(value.tag(), ValueTag::Aggregate | ValueTag::String) {
        return Err(Error::InvalidFieldAccess {
            index: global.slot_offset as u32,
            field_count: 0,
        });
    }

    let handle = value.as_managed_reference().unwrap();
    if value.tag() == ValueTag::String {
        return load_string_field(state.heap_ref(), handle, global.slot_offset as u32);
    }

    load_heap_slot(state, handle, global.slot_offset)
}

/// Store through a global pointer, including slot offsets.
#[inline(always)]
fn store_global_slot(
    state: &mut ThreadedState<'_, '_>,
    global: GlobalPointer,
    value: Value,
) -> Result<(), Error> {
    // update the global value directly
    if global.slot_offset == 0 {
        state.interpreter.isolate.globals.set(global.id, value);
        return Ok(());
    }

    // update a slot on the aggregate stored in the global
    let current = state
        .interpreter
        .isolate
        .globals
        .get(global.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: global.id })?;
    if !matches!(current.tag(), ValueTag::Aggregate | ValueTag::String) {
        return Err(Error::InvalidFieldAccess {
            index: global.slot_offset as u32,
            field_count: 0,
        });
    }

    let handle = current.as_managed_reference().unwrap();
    if current.tag() == ValueTag::String {
        store_string_field(state.heap(), handle, global.slot_offset as u32, value)?;
        state.interpreter.isolate.globals.set(global.id, current);
        return Ok(());
    }

    store_heap_slot(state, handle, global.slot_offset, value)?;
    state.interpreter.isolate.globals.set(global.id, current);
    Ok(())
}

/// Get a field from a heap aggregate.
#[inline(always)]
fn get_heap_field(
    state: &mut ThreadedState<'_, '_>,
    handle: ManagedReference,
    index: u32,
) -> Result<Value, Error> {
    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed allocation
    let heap = state.heap_ref();
    let cell_len = packed_value_count(heap, handle)?;

    // fast path for small known aggregates
    let inline_slot_index = handle.byte_offset() / Value::BYTE_LEN + index as usize;
    if inline_slot_index < cell_len && cell_len <= 2 {
        return load_packed_value(heap, handle, inline_slot_index);
    }

    // resolve the target slot
    let slot_index = if state.bounds_checks {
        handle
            .byte_offset()
            .checked_add(index as usize)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: cell_len,
            })?
    } else {
        handle.byte_offset() / Value::BYTE_LEN + index as usize
    };

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell_len, "heap field out of bounds");
        let value = load_packed_value(heap, handle, slot_index)?;
        return Ok(value);
    }

    // read the slot when in bounds
    let value = heap
        .packed_value_at(handle, slot_index)
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: cell_len,
        })?;

    Ok(value)
}

/// Set a field on a heap aggregate.
#[inline(always)]
fn set_heap_field(
    state: &mut ThreadedState<'_, '_>,
    handle: ManagedReference,
    index: u32,
    value: Value,
) -> Result<(), Error> {
    let bounds_checks = state.bounds_checks;
    let null_checks = state.null_checks;

    // reject null handles when enabled
    if null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed allocation
    let heap = state.heap();
    let cell_len = packed_value_count(heap, handle)?;

    // fast path for small known aggregates
    let inline_slot_index = handle.byte_offset() / Value::BYTE_LEN + index as usize;
    if inline_slot_index < cell_len && cell_len <= 2 {
        store_packed_value(heap, handle, inline_slot_index, value)?;
        return Ok(());
    }

    // resolve the target slot
    let slot_index = if bounds_checks {
        handle
            .byte_offset()
            .checked_add(index as usize)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: cell_len,
            })?
    } else {
        handle.byte_offset() / Value::BYTE_LEN + index as usize
    };

    // fast path without bounds checks
    if !bounds_checks {
        debug_assert!(slot_index < cell_len, "heap field out of bounds");
        store_packed_value(heap, handle, slot_index, value)?;
        return Ok(());
    }

    // write slot when in bounds
    if heap.set_packed_value(handle, slot_index, value) {
        return Ok(());
    }

    Err(Error::InvalidFieldAccess {
        index,
        field_count: cell_len,
    })
}

/// Get an element from a heap array aggregate.
#[inline(always)]
fn get_heap_element(
    state: &mut ThreadedState<'_, '_>,
    handle: ManagedReference,
    index: u64,
) -> Result<Value, Error> {
    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed allocation
    let heap = state.heap_ref();
    let cell_len = packed_value_count(heap, handle)?;

    // resolve the target slot
    let index_usize = if state.bounds_checks {
        usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell_len as u64,
        })?
    } else {
        index as usize
    };
    let slot_index = if state.bounds_checks {
        handle
            .byte_offset()
            .checked_add(index_usize)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: cell_len as u64,
            })?
    } else {
        handle.byte_offset() / Value::BYTE_LEN + index_usize
    };

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell_len, "heap element out of bounds");
        let value = load_packed_value(heap, handle, slot_index)?;
        return Ok(value);
    }

    // read the slot when in bounds
    let value = heap
        .packed_value_at(handle, slot_index)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: cell_len as u64,
        })?;

    Ok(value)
}

/// Set an element on a heap array aggregate.
#[inline(always)]
fn set_heap_element(
    state: &mut ThreadedState<'_, '_>,
    handle: ManagedReference,
    index: u64,
    value: Value,
) -> Result<(), Error> {
    let bounds_checks = state.bounds_checks;
    let null_checks = state.null_checks;

    // reject null handles when enabled
    if null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed heap allocation
    let heap = state.heap();
    let allocation_len = packed_value_count(heap, handle)?;

    // resolve the target slot
    let index_usize = if bounds_checks {
        usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: allocation_len as u64,
        })?
    } else {
        index as usize
    };
    let slot_index = if bounds_checks {
        handle
            .byte_offset()
            .checked_add(index_usize)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: allocation_len as u64,
            })?
    } else {
        handle.byte_offset() / Value::BYTE_LEN + index_usize
    };

    // fast path without bounds checks
    if !bounds_checks {
        debug_assert!(slot_index < allocation_len, "heap element out of bounds");
        store_packed_value(heap, handle, slot_index, value)?;
        return Ok(());
    }

    // write slot when in bounds
    if heap.set_packed_value(handle, slot_index, value) {
        return Ok(());
    }

    Err(Error::InvalidArrayAccess {
        index,
        length: allocation_len as u64,
    })
}

/// Resolve a field slot for a managed heap pointer.
#[inline(always)]
fn resolve_heap_field_slot(
    state: &mut ThreadedState<'_, '_>,
    handle: ManagedReference,
    index: u32,
    field_count: u32,
) -> Result<u32, Error> {
    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed allocation
    let heap = state.heap_ref();
    let cell_len = packed_value_count(heap, handle)?;

    // select field count for diagnostics
    let field_count_for_error = if field_count == UNKNOWN_FIELD_COUNT {
        cell_len
    } else {
        field_count as usize
    };

    // compute the absolute slot offset
    let slot_index = if state.bounds_checks {
        handle
            .byte_offset()
            .checked_add(index as usize)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: field_count_for_error,
            })?
    } else {
        handle.byte_offset() / Value::BYTE_LEN + index as usize
    };

    // validate bounds when field count is unknown
    if state.bounds_checks
        && field_count == UNKNOWN_FIELD_COUNT
        && cell_len != 0
        && slot_index >= cell_len
    {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: cell_len,
        });
    }

    // narrow slot index for pointer encoding
    if state.bounds_checks {
        return u32::try_from(slot_index).map_err(|_| Error::InvalidFieldAccess {
            index,
            field_count: field_count_for_error,
        });
    }

    debug_assert!(slot_index <= u32::MAX as usize, "heap slot index overflow");
    Ok(slot_index as u32)
}

/// Resolve a field slot for a stack pointer.
#[inline(always)]
fn resolve_stack_field_slot(
    state: &mut ThreadedState<'_, '_>,
    pointer: StackPointer,
    index: u32,
    field_count: u32,
) -> Result<usize, Error> {
    // look up the stack buffer
    let frame = state.frame_by_index(pointer.frame_idx)?;
    let cell = frame
        .stack_buffer(pointer.slot)
        .ok_or(Error::InvalidManagedReference)?;
    let cell_len = cell.len();

    // compute the absolute slot offset
    let slot_index = if state.bounds_checks {
        pointer
            .slot_offset
            .checked_add(index as usize)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: cell_len,
            })?
    } else {
        pointer.slot_offset.wrapping_add(index as usize)
    };

    // validate bounds when field count is unknown
    if state.bounds_checks
        && field_count == UNKNOWN_FIELD_COUNT
        && cell_len != 0
        && slot_index >= cell_len
    {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: cell_len,
        });
    }

    Ok(slot_index)
}

/// Resolve a field slot for a global pointer.
#[inline(always)]
fn resolve_global_field_slot(
    state: &mut ThreadedState<'_, '_>,
    global: GlobalPointer,
    index: u32,
    field_count: u32,
) -> Result<usize, Error> {
    // load the global value
    let value = state
        .interpreter
        .isolate
        .globals
        .get(global.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: global.id })?;

    // require aggregate payload
    if !matches!(value.tag(), ValueTag::Aggregate | ValueTag::String) {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: 0,
        });
    }

    let handle = value.as_managed_reference().unwrap();

    // look up the managed allocation
    let heap = state.heap_ref();
    let cell_len = packed_value_count(heap, handle)?;

    // compute the absolute slot offset
    let slot_index = if state.bounds_checks {
        global
            .slot_offset
            .checked_add(index as usize)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: cell_len,
            })?
    } else {
        global.slot_offset.wrapping_add(index as usize)
    };

    // validate bounds when field count is unknown
    if state.bounds_checks
        && field_count == UNKNOWN_FIELD_COUNT
        && cell_len != 0
        && slot_index >= cell_len
    {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: cell_len,
        });
    }

    Ok(slot_index)
}

/// Resolve an element slot for a managed heap pointer.
#[inline(always)]
fn resolve_heap_element_slot(
    state: &mut ThreadedState<'_, '_>,
    handle: ManagedReference,
    index: u64,
    array_length: u64,
) -> Result<u32, Error> {
    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed allocation
    let heap = state.heap_ref();
    let cell_len = packed_value_count(heap, handle)?;

    // compute the absolute slot offset
    let index_usize = if state.bounds_checks {
        usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell_len as u64,
        })?
    } else {
        index as usize
    };
    let slot_index = if state.bounds_checks {
        handle
            .byte_offset()
            .checked_add(index_usize)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: cell_len as u64,
            })?
    } else {
        handle.byte_offset() / Value::BYTE_LEN + index_usize
    };

    // validate bounds when array length is unknown
    if state.bounds_checks && array_length == UNKNOWN_ARRAY_LENGTH && slot_index >= cell_len {
        return Err(Error::InvalidArrayAccess {
            index,
            length: cell_len as u64,
        });
    }

    // narrow slot index for pointer encoding
    if state.bounds_checks {
        return u32::try_from(slot_index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell_len as u64,
        });
    }

    debug_assert!(
        slot_index <= u32::MAX as usize,
        "heap element slot index overflow"
    );
    Ok(slot_index as u32)
}

/// Resolve an element slot for a stack pointer.
#[inline(always)]
fn resolve_stack_element_slot(
    state: &mut ThreadedState<'_, '_>,
    pointer: StackPointer,
    index: u64,
    array_length: u64,
) -> Result<usize, Error> {
    // look up the stack buffer
    let frame = state.frame_by_index(pointer.frame_idx)?;
    let cell = frame
        .stack_buffer(pointer.slot)
        .ok_or(Error::InvalidManagedReference)?;

    // compute the absolute slot offset
    let index_usize = if state.bounds_checks {
        usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell.len() as u64,
        })?
    } else {
        index as usize
    };
    let slot_index = if state.bounds_checks {
        pointer
            .slot_offset
            .checked_add(index_usize)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: cell.len() as u64,
            })?
    } else {
        pointer.slot_offset.wrapping_add(index_usize)
    };

    // validate bounds when array length is unknown
    if state.bounds_checks && array_length == UNKNOWN_ARRAY_LENGTH && slot_index >= cell.len() {
        return Err(Error::InvalidArrayAccess {
            index,
            length: cell.len() as u64,
        });
    }

    Ok(slot_index)
}

/// Resolve an element slot for a global pointer.
#[inline(always)]
fn resolve_global_element_slot(
    state: &mut ThreadedState<'_, '_>,
    global: GlobalPointer,
    index: u64,
    array_length: u64,
) -> Result<usize, Error> {
    // load the global value
    let value = state
        .interpreter
        .isolate
        .globals
        .get(global.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: global.id })?;

    // require aggregate payload
    if value.tag() != ValueTag::Aggregate {
        return Err(Error::InvalidArrayAccess { index, length: 0 });
    }

    let handle = value.as_managed_reference().unwrap();

    // look up the managed allocation
    let heap = state.heap_ref();
    let cell_len = heap
        .packed_value_count(handle)
        .ok_or(Error::InvalidManagedReference)?;

    // compute the absolute slot offset
    let index_usize = if state.bounds_checks {
        usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell_len as u64,
        })?
    } else {
        index as usize
    };
    let slot_index = if state.bounds_checks {
        global
            .slot_offset
            .checked_add(index_usize)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: cell_len as u64,
            })?
    } else {
        global.slot_offset.wrapping_add(index_usize)
    };

    // validate bounds when array length is unknown
    if state.bounds_checks && array_length == UNKNOWN_ARRAY_LENGTH && slot_index >= cell_len {
        return Err(Error::InvalidArrayAccess {
            index,
            length: cell_len as u64,
        });
    }

    Ok(slot_index)
}

/// Load a slot from a stack allocation.
#[inline(always)]
fn load_stack_slot(
    state: &mut ThreadedState<'_, '_>,
    sp: StackPointer,
    slot_index: usize,
) -> Result<Value, Error> {
    // resolve the stack buffer
    let frame = state.frame_by_index(sp.frame_idx)?;
    let cell = frame
        .stack_buffer(sp.slot)
        .ok_or(Error::InvalidManagedReference)?;

    // treat empty slot 0 as void
    if cell.is_empty() && slot_index == 0 {
        return Ok(Value::VOID);
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell.len(), "stack slot out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        let value = unsafe { *cell.get_unchecked(slot_index) };
        return Ok(value);
    }

    // read the slot when in bounds
    if let Some(value) = cell.get(slot_index).copied() {
        return Ok(value);
    }

    Err(Error::InvalidFieldAccess {
        index: slot_index as u32,
        field_count: cell.len(),
    })
}

/// Load a local slot from a frame.
#[inline(always)]
fn load_local_slot(
    state: &mut ThreadedState<'_, '_>,
    pointer: LocalPointer,
    slot_offset: usize,
) -> Result<Value, Error> {
    // reject non zero offsets
    if slot_offset != 0 {
        return Err(Error::InvalidFieldAccess {
            index: slot_offset as u32,
            field_count: 1,
        });
    }

    // resolve the target frame
    let frame = state.frame_by_index(pointer.frame_idx)?;

    // resolve the local id
    let local = mir::LocalNodeId::new(pointer.local as u32);

    // read the local slot
    frame.get_local_or_error(&state.interpreter.engine.local_stack, local)
}

/// Store a slot into a stack allocation.
#[inline(always)]
fn store_stack_slot(
    state: &mut ThreadedState<'_, '_>,
    sp: StackPointer,
    slot_index: usize,
    value: Value,
) -> Result<(), Error> {
    // cache bounds checks setting
    let bounds_checks = state.bounds_checks;

    // resolve the stack buffer
    let frame = state.frame_by_index_mut(sp.frame_idx)?;
    let cell = frame
        .stack_buffer_mut(sp.slot)
        .ok_or(Error::InvalidManagedReference)?;

    // resize slots as needed when bounds checks are enabled
    if bounds_checks && cell.len() <= slot_index {
        cell.resize(slot_index + 1, Value::VOID);
    }

    // fast path without bounds checks
    if !bounds_checks {
        debug_assert!(slot_index < cell.len(), "stack slot out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        unsafe {
            *cell.get_unchecked_mut(slot_index) = value;
        }
        return Ok(());
    }

    // write slot when in bounds
    if let Some(slot) = cell.get_mut(slot_index) {
        *slot = value;
        return Ok(());
    }

    Err(Error::InvalidFieldAccess {
        index: slot_index as u32,
        field_count: cell.len(),
    })
}

/// Store a local slot into a frame.
#[inline(always)]
fn store_local_slot(
    state: &mut ThreadedState<'_, '_>,
    pointer: LocalPointer,
    slot_offset: usize,
    value: Value,
) -> Result<(), Error> {
    // reject non zero offsets
    if slot_offset != 0 {
        return Err(Error::InvalidFieldAccess {
            index: slot_offset as u32,
            field_count: 1,
        });
    }

    // resolve the target frame
    let (local_base, local_count) = {
        let frame = state.frame_by_index(pointer.frame_idx)?;
        (frame.local_base, frame.local_count)
    };

    // resolve the local id
    let local = mir::LocalNodeId::new(pointer.local as u32);
    let local_index = local.id as usize;

    // validate local bounds
    if local_index >= local_count {
        return Err(Error::UndefinedLocal { local });
    }

    // write the local slot
    let slot = local_base + local_index;
    let locals = &mut state.interpreter.engine.local_stack;
    let Some(target) = locals.get_mut(slot) else {
        return Err(Error::UndefinedLocal { local });
    };
    *target = value;

    Ok(())
}
