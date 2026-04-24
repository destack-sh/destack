use crate::diagnostic::Error;
use crate::module::{ElementAccess, FieldAccess, TypedAccess};
use crate::{
    FramePointer, GlobalPointer, HeapReference, RawPointer, ReferenceMeta, SharedHeapReference,
    SharedRawPointer, StackPointer, Value, ValueTag,
};
use destack_heap::Payload;
use destack_mir as mir;

use super::storage::*;
use super::{
    frame_pointer_value_with_meta, global_pointer_value_with_meta, stack_pointer_value,
    stack_pointer_value_with_meta,
};
use crate::interpreter::StepState;
use crate::telemetry::stat_inc;

pub(crate) use super::storage::{
    allocate_callable, allocate_heap_value_by_index, allocate_zeroed_heap_value, decode_callable,
    decode_raw_value, encode_raw_value, encode_value_bytes, load_from_raw_pointer_typed,
    raw_type_size, store_to_raw_pointer_typed,
};

const POINTER_BASE_MASK: u64 = 0xFFFF_FFFF;
const POINTER_SLOT_SHIFT: u64 = 32;
const STACK_INDEX_MASK: u64 = 0xFFFF;
const STACK_SLOT_SHIFT: u64 = 16;

/// Build one invalid-pointer-type error for the given value.
#[inline(always)]
pub(super) fn invalid_pointer_type(value: Value) -> Error {
    Error::InvalidPointerType {
        actual: format!("{value:?}"),
    }
}

/// Build one invalid-pointer-type error for the given description.
#[inline(always)]
fn invalid_pointer_description(actual: impl Into<String>) -> Error {
    Error::InvalidPointerType {
        actual: actual.into(),
    }
}

/// Build one non-scalar value load error.
#[inline(always)]
fn non_scalar_load_error(ty: mir::LocalNodeId<mir::Type>) -> Error {
    Error::TypeMismatch {
        expected: "value-representable type".to_string(),
        actual: format!("{ty:?}"),
    }
}

/// Require one heap reference value.
#[inline(always)]
fn heap_reference_from_value(value: Value) -> Result<HeapReference, Error> {
    value
        .as_heap_reference()
        .ok_or_else(|| invalid_pointer_type(value))
}

/// Require one shared heap reference value.
#[inline(always)]
fn shared_heap_reference_from_value(value: Value) -> Result<SharedHeapReference, Error> {
    value
        .as_shared_heap_reference()
        .ok_or_else(|| invalid_pointer_type(value))
}

/// Require one stack pointer value.
#[inline(always)]
fn stack_pointer_from_value(value: Value) -> Result<StackPointer, Error> {
    value
        .as_stack_pointer()
        .ok_or_else(|| invalid_pointer_type(value))
}

/// Require one frame pointer value.
#[inline(always)]
fn frame_pointer_from_value(value: Value) -> Result<FramePointer, Error> {
    value
        .as_frame_pointer()
        .ok_or_else(|| invalid_pointer_type(value))
}

/// Require one global pointer value.
#[inline(always)]
fn global_pointer_from_value(value: Value) -> Result<GlobalPointer, Error> {
    value
        .as_global_pointer()
        .ok_or_else(|| invalid_pointer_type(value))
}

/// Return the effective field count for one error path.
#[inline(always)]
fn field_count_for_error(field_count: Option<u32>, actual_count: usize) -> usize {
    field_count.map_or(actual_count, |field_count| field_count as usize)
}

/// Return the effective array length for one error path.
#[inline(always)]
fn array_length_for_error(array_length: Option<u64>, actual_length: u64) -> u64 {
    array_length.unwrap_or(actual_length)
}

/// Return one lowered element access for an already known indexed type.
#[inline(always)]
pub(crate) fn element_access_for_type(
    state: &StepState<'_, '_>,
    indexed_type: mir::LocalNodeId<mir::Type>,
    index: u64,
) -> Result<(ElementAccess, Option<u64>), Error> {
    let layout = state.layout(indexed_type)?;
    let element_count = layout.element_count().unwrap_or(0) as u64;
    let element = layout.element().ok_or(Error::InvalidArrayAccess {
        index,
        length: element_count,
    })?;

    if index >= element_count {
        return Err(Error::InvalidArrayAccess {
            index,
            length: element_count,
        });
    }

    let access = ElementAccess {
        value_type: element.ty,
        byte_stride: element.stride,
        byte_len: element.byte_len,
        is_scalar: state.layout(element.ty)?.is_scalar(),
    };

    Ok((access, Some(element_count)))
}

/// Record one shared heap write barrier from one exact stored value.
#[inline(always)]
fn publish_shared_store(
    state: &mut StepState<'_, '_>,
    destination: SharedHeapReference,
    start: usize,
    bytes: &[u8],
    value: Value,
) -> Result<(), Error> {
    // exact shared reference stores can publish directly
    if let Some(reference) = value.as_shared_heap_reference() {
        state
            .shared()
            .publish_edge(reference)
            .map_err(Error::from)?;

        return Ok(());
    }

    // fall back to the generic byte-range barrier for packed values
    state
        .shared()
        .write_barrier_bytes(destination, start, bytes)
        .map_err(Error::from)
}

/// Remap one invalid heap reference into one field access error.
#[inline(always)]
fn map_invalid_field_reference(error: Error, index: u32, field_count: usize) -> Error {
    match error {
        Error::InvalidHeapReference => Error::InvalidFieldAccess { index, field_count },
        other => other,
    }
}

/// Remap one invalid heap reference into one array access error.
#[inline(always)]
fn map_invalid_array_reference(error: Error, index: u64, length: u64) -> Error {
    match error {
        Error::InvalidHeapReference => Error::InvalidArrayAccess { index, length },
        other => other,
    }
}

/// Decode one raw bit pattern into a pointer-shaped VM value.
pub(crate) fn decode_pointer_bits(raw: u64, target_type: &mir::Type) -> Result<Value, Error> {
    match target_type {
        mir::Type::FunctionPointer { .. } => {
            Ok(Value::function_pointer(mir::LocalNodeId::new(raw as u32)))
        }
        mir::Type::Reference {
            kind,
            address_space,
            mutability,
            is_nullable,
            ..
        } => {
            let meta = ReferenceMeta::new(*kind, address_space.clone(), *mutability, *is_nullable);
            let value = match (*kind, address_space.clone()) {
                (
                    mir::ReferenceKind::Managed | mir::ReferenceKind::Owned,
                    mir::AddressSpace::Shared,
                ) => Value::shared_heap_reference_with_meta(
                    SharedHeapReference::from_bits(raw as usize),
                    meta,
                ),
                (mir::ReferenceKind::Managed | mir::ReferenceKind::Owned, _) => {
                    Value::heap_reference_with_meta(HeapReference::from_bits(raw as usize), meta)
                }
                (mir::ReferenceKind::Borrowed, mir::AddressSpace::Shared) => {
                    Value::shared_heap_reference_with_meta(
                        SharedHeapReference::from_bits(raw as usize),
                        meta,
                    )
                }
                (_, mir::AddressSpace::Stack) => {
                    let frame_index = (raw & STACK_INDEX_MASK) as usize;
                    let slot = ((raw >> STACK_SLOT_SHIFT) & STACK_INDEX_MASK) as usize;
                    let byte_offset = ((raw >> POINTER_SLOT_SHIFT) & POINTER_BASE_MASK) as usize;
                    let pointer = StackPointer::with_offset(frame_index, slot, byte_offset);

                    stack_pointer_value_with_meta(pointer, meta)?
                }
                (_, mir::AddressSpace::Frame) => {
                    let frame_index = (raw & STACK_INDEX_MASK) as usize;
                    let slot = ((raw >> STACK_SLOT_SHIFT) & STACK_INDEX_MASK) as usize;
                    let byte_offset = ((raw >> POINTER_SLOT_SHIFT) & POINTER_BASE_MASK) as usize;
                    let pointer = FramePointer::with_offset(frame_index, slot, byte_offset);

                    frame_pointer_value_with_meta(pointer, meta)?
                }
                (_, mir::AddressSpace::Static) => {
                    let global = mir::LocalNodeId::new((raw & POINTER_BASE_MASK) as u32);
                    let byte_offset = ((raw >> POINTER_SLOT_SHIFT) & POINTER_BASE_MASK) as usize;

                    global_pointer_value_with_meta(global, byte_offset, meta)?
                }
                (mir::ReferenceKind::Borrowed, _) => {
                    Value::heap_reference_with_meta(HeapReference::from_bits(raw as usize), meta)
                }
                (_, mir::AddressSpace::Shared) => Value::shared_raw_pointer_with_meta(
                    SharedRawPointer::from_bits(raw as usize),
                    meta,
                ),
                _ => Value::raw_pointer_with_meta(RawPointer::from_bits(raw as usize), meta),
            };

            Ok(value)
        }
        _ => Ok(Value::raw_pointer(RawPointer::from_bits(raw as usize))),
    }
}

/// Load a value from a pointer with one optional typed access plan.
#[inline(always)]
pub(crate) fn load_from_pointer_with_access(
    state: &mut StepState<'_, '_>,
    ptr: Value,
    access: Option<TypedAccess>,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.engine.statistics, loads);
    }

    // resolve pointer kind and load
    match ptr.tag() {
        ValueTag::HeapReference => {
            // use the compiled typed access when the pointee is known
            if let Some(access) = access {
                return load_from_heap_reference_typed(state, ptr, access);
            }

            Err(invalid_pointer_description(
                "heap reference without pointee type",
            ))
        }
        ValueTag::SharedHeapReference => {
            // use the compiled typed access when the pointee is known
            if let Some(access) = access {
                return load_from_shared_heap_reference_typed(state, ptr, access);
            }

            Err(invalid_pointer_description(
                "shared heap reference without pointee type",
            ))
        }
        ValueTag::RawPointer => {
            // require the raw pointee type before decoding raw memory
            let Some(access) = access else {
                return Err(invalid_pointer_description(
                    "raw pointer without pointee type",
                ));
            };

            load_from_raw_pointer_typed(state, ptr, access)
        }
        ValueTag::StackPointer => {
            // require the stack pointee type before decoding stack storage
            let sp = stack_pointer_from_value(ptr)?;
            let Some(access) = access else {
                return Err(invalid_pointer_description(
                    "stack pointer without pointee type",
                ));
            };

            load_from_stack_pointer_typed(state, sp, access)
        }
        ValueTag::FramePointer => {
            // load one frame slot directly
            let pointer = frame_pointer_from_value(ptr)?;
            load_frame_slot(state, pointer, pointer.byte_offset)
        }
        ValueTag::GlobalPointer => {
            // load one global slot directly
            let global = global_pointer_from_value(ptr)?;
            load_global_slot(state, global)
        }

        // reject non pointer values loudly
        _ => Err(invalid_pointer_type(ptr)),
    }
}

/// Store a value to a pointer with one optional typed access plan.
#[inline(always)]
pub(crate) fn store_to_pointer_with_access(
    state: &mut StepState<'_, '_>,
    ptr: Value,
    access: Option<TypedAccess>,
    value: Value,
) -> Result<(), Error> {
    // track pointer stores
    if state.collect_stats {
        stat_inc!(state.engine.statistics, stores);
    }

    // resolve pointer kind and store
    match ptr.tag() {
        ValueTag::HeapReference => {
            // use the compiled typed access when the pointee is known
            if let Some(access) = access {
                return store_to_heap_reference_typed(state, ptr, access, value);
            }

            Err(invalid_pointer_description(
                "heap reference without pointee type",
            ))
        }
        ValueTag::SharedHeapReference => {
            // use the compiled typed access when the pointee is known
            if let Some(access) = access {
                return store_to_shared_heap_reference_typed(state, ptr, access, value);
            }

            Err(invalid_pointer_description(
                "shared heap reference without pointee type",
            ))
        }
        ValueTag::RawPointer => {
            // require the raw pointee type before encoding raw memory
            let Some(access) = access else {
                return Err(invalid_pointer_description(
                    "raw pointer without pointee type",
                ));
            };

            store_to_raw_pointer_typed(state, ptr, access, value)
        }
        ValueTag::StackPointer => {
            // require the stack pointee type before encoding stack storage
            let sp = stack_pointer_from_value(ptr)?;
            let Some(access) = access else {
                return Err(invalid_pointer_description(
                    "stack pointer without pointee type",
                ));
            };

            store_to_stack_pointer_typed(state, sp, access, value)
        }
        ValueTag::FramePointer => {
            // store one frame slot directly
            let pointer = frame_pointer_from_value(ptr)?;
            store_frame_slot(state, pointer, pointer.byte_offset, value)
        }
        ValueTag::GlobalPointer => {
            // reject immutable globals before storing the slot
            let global = global_pointer_from_value(ptr)?;
            let global_def = state.tree().get(global.id);
            if !global_def.is_mutable() {
                return Err(Error::ImmutableGlobalWrite { global: global.id });
            }

            store_global_slot(state, global, value)
        }

        // reject non pointer values loudly
        _ => Err(invalid_pointer_type(ptr)),
    }
}

/// Load a value from a heap reference.
#[inline(always)]
pub(crate) fn load_from_heap_reference_typed(
    state: &mut StepState<'_, '_>,
    ptr: Value,
    access: TypedAccess,
) -> Result<Value, Error> {
    // require one heap reference value
    if ptr.tag() != ValueTag::HeapReference {
        return Err(invalid_pointer_type(ptr));
    }

    // reject null pointers before reading the allocation
    let handle = heap_reference_from_value(ptr)?;
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // require one live heap allocation
    let heap = state.heap();
    if !heap.is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    if !access.is_scalar {
        return Ok(ptr);
    }

    // decode the storage bytes
    let owned_bytes = state
        .read_heap_bytes(handle, 0, access.byte_len)
        .map_err(Error::from)?;

    decode_raw_value(state.tree(), access.value_type, &owned_bytes)
}

/// Load a value from a shared heap reference.
#[inline(always)]
pub(crate) fn load_from_shared_heap_reference_typed(
    state: &mut StepState<'_, '_>,
    ptr: Value,
    access: TypedAccess,
) -> Result<Value, Error> {
    // require one shared heap reference value
    if ptr.tag() != ValueTag::SharedHeapReference {
        return Err(invalid_pointer_type(ptr));
    }

    // reject null pointers before reading the allocation
    let handle = shared_heap_reference_from_value(ptr)?;
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // require one live shared heap allocation
    if !state.shared().is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    if !access.is_scalar {
        return Ok(ptr);
    }

    // decode the storage bytes
    let owned_bytes = state
        .read_shared_heap_bytes(handle, 0, access.byte_len)
        .map_err(Error::from)?;

    decode_raw_value(state.tree(), access.value_type, &owned_bytes)
}

/// Load a value from a stack pointer.
#[inline(always)]
pub(crate) fn load_from_stack_pointer_typed(
    state: &mut StepState<'_, '_>,
    pointer: StackPointer,
    access: TypedAccess,
) -> Result<Value, Error> {
    let frame = state.frame_by_index(pointer.frame_idx)?;
    let allocation = frame
        .stack_allocation(pointer.slot)
        .ok_or(Error::InvalidHeapReference)?;
    let start = pointer.byte_offset;
    let end = start
        .checked_add(access.byte_len)
        .ok_or(Error::InvalidFieldAccess {
            index: start as u32,
            field_count: allocation.len(),
        })?;
    let byte_range = allocation
        .bytes()
        .get(start..end)
        .ok_or(Error::InvalidFieldAccess {
            index: start as u32,
            field_count: allocation.len(),
        })?;
    if !access.is_scalar {
        return stack_pointer_value(pointer);
    }

    decode_raw_value(state.tree(), access.value_type, byte_range)
}

/// Load a value from a frame pointer.
#[inline(always)]
pub(crate) fn load_from_frame_pointer(
    state: &mut StepState<'_, '_>,
    ptr: Value,
) -> Result<Value, Error> {
    // validate pointer tag
    if ptr.tag() != ValueTag::FramePointer {
        return Err(invalid_pointer_type(ptr));
    }

    // resolve pointer
    let pointer = frame_pointer_from_value(ptr)?;
    load_frame_slot(state, pointer, pointer.byte_offset)
}

/// Load a value from a global pointer.
#[inline(always)]
pub(crate) fn load_from_global_pointer(
    state: &mut StepState<'_, '_>,
    ptr: Value,
) -> Result<Value, Error> {
    // validate pointer tag
    if ptr.tag() != ValueTag::GlobalPointer {
        return Err(invalid_pointer_type(ptr));
    }

    // resolve pointer
    let global = global_pointer_from_value(ptr)?;
    load_global_slot(state, global)
}

/// Store a value through a heap reference.
#[inline(always)]
pub(crate) fn store_to_heap_reference_typed(
    state: &mut StepState<'_, '_>,
    ptr: Value,
    access: TypedAccess,
    val: Value,
) -> Result<(), Error> {
    if ptr.tag() != ValueTag::HeapReference {
        return Err(invalid_pointer_type(ptr));
    }

    let handle = heap_reference_from_value(ptr)?;
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    {
        let heap = state.heap();
        if !heap.is_heap_live(handle) {
            return Err(Error::InvalidHeapReference);
        }
    }

    let bytes = encode_value_bytes(state, access.value_type, val)?;
    let byte_len = state.heap().heap_byte_len(handle)?;
    let start = 0usize;
    let end = bytes.len();

    if end > byte_len {
        return Err(Error::InvalidFieldAccess {
            index: start as u32,
            field_count: byte_len,
        });
    }

    state
        .write_heap_bytes(handle, start, &bytes)
        .map_err(Error::from)?;

    Ok(())
}

/// Store a value through a shared heap reference.
#[inline(always)]
pub(crate) fn store_to_shared_heap_reference_typed(
    state: &mut StepState<'_, '_>,
    ptr: Value,
    access: TypedAccess,
    val: Value,
) -> Result<(), Error> {
    if ptr.tag() != ValueTag::SharedHeapReference {
        return Err(invalid_pointer_type(ptr));
    }

    let handle = shared_heap_reference_from_value(ptr)?;
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    {
        if !state.shared().is_heap_live(handle) {
            return Err(Error::InvalidHeapReference);
        }
    }

    let bytes = encode_value_bytes(state, access.value_type, val)?;
    let byte_len = state
        .shared_ref()
        .heap_byte_len(handle)
        .map_err(Error::from)?;
    let start = 0usize;
    let end = bytes.len();

    if end > byte_len {
        return Err(Error::InvalidFieldAccess {
            index: start as u32,
            field_count: byte_len,
        });
    }

    state
        .shared()
        .write_heap_bytes(handle, start, &bytes)
        .map_err(Error::from)?;
    publish_shared_store(state, handle, start, &bytes, val)?;

    Ok(())
}

/// Store a value through a stack pointer.
#[inline(always)]
pub(crate) fn store_to_stack_pointer_typed(
    state: &mut StepState<'_, '_>,
    pointer: StackPointer,
    access: TypedAccess,
    value: Value,
) -> Result<(), Error> {
    // encode the typed payload
    let bytes = encode_value_bytes(state, access.value_type, value)?;

    let frame = state.frame_by_index_mut(pointer.frame_idx)?;
    let allocation = frame
        .stack_allocation_mut(pointer.slot)
        .ok_or(Error::InvalidHeapReference)?;
    let start = pointer.byte_offset;
    let end = start
        .checked_add(bytes.len())
        .ok_or(Error::InvalidFieldAccess {
            index: start as u32,
            field_count: allocation.len(),
        })?;

    if end > allocation.len() {
        return Err(Error::InvalidFieldAccess {
            index: start as u32,
            field_count: allocation.len(),
        });
    }

    allocation.bytes_mut()[start..end].copy_from_slice(&bytes);

    Ok(())
}

/// Store a value through a frame pointer.
#[inline(always)]
pub(crate) fn store_to_frame_pointer(
    state: &mut StepState<'_, '_>,
    ptr: Value,
    val: Value,
) -> Result<(), Error> {
    // validate pointer tag
    if ptr.tag() != ValueTag::FramePointer {
        return Err(invalid_pointer_type(ptr));
    }

    // resolve pointer
    let pointer = frame_pointer_from_value(ptr)?;
    store_frame_slot(state, pointer, pointer.byte_offset, val)
}

/// Store a value through a global pointer.
#[inline(always)]
pub(crate) fn store_to_global_pointer(
    state: &mut StepState<'_, '_>,
    ptr: Value,
    val: Value,
) -> Result<(), Error> {
    // validate pointer tag
    if ptr.tag() != ValueTag::GlobalPointer {
        return Err(invalid_pointer_type(ptr));
    }

    // resolve pointer
    let global = global_pointer_from_value(ptr)?;
    let global_def = state.tree().get(global.id);
    if !global_def.is_mutable() {
        return Err(Error::ImmutableGlobalWrite { global: global.id });
    }
    store_global_slot(state, global, val)
}

/// Get the address of a field from an composite or pointer.
#[inline(always)]
pub(crate) fn field_addr(
    state: &mut StepState<'_, '_>,
    composite: Value,
    index: u32,
    field_count: Option<u32>,
) -> Result<Value, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    // resolve the source and compute the field pointer
    match composite.tag() {
        ValueTag::HeapReference => Err(invalid_pointer_description(
            "heap reference requires typed field access",
        )),
        ValueTag::SharedHeapReference => Err(invalid_pointer_description(
            "shared heap reference requires typed field access",
        )),
        ValueTag::RawPointer => Err(invalid_pointer_description(
            "raw pointer requires typed field access",
        )),
        ValueTag::StackPointer => Err(invalid_pointer_description(
            "stack pointer requires typed field access",
        )),
        ValueTag::FramePointer => {
            let pointer = frame_pointer_from_value(composite)?;
            field_addr_local(state, pointer, index, field_count)
        }
        ValueTag::GlobalPointer => Err(invalid_pointer_description(
            "global pointer requires typed field access",
        )),
        _ => Err(Error::TypeMismatch {
            expected: "composite or pointer".to_string(),
            actual: format!("{composite:?}"),
        }),
    }
}

/// Get the address of a field from a heap reference.
#[inline(always)]
pub(crate) fn field_addr_heap(
    state: &mut StepState<'_, '_>,
    handle: HeapReference,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
) -> Result<Value, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    let heap = state.heap();
    if !heap.is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let field_count_for_error = field_count_for_error(field_count, index as usize + 1);
    let handle = handle
        .add_bytes(field.byte_offset)
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: field_count_for_error,
        })?;

    Ok(Value::heap_reference(handle))
}

/// Get the address of a field from a shared heap reference.
#[inline(always)]
pub(crate) fn field_addr_shared_heap(
    state: &mut StepState<'_, '_>,
    handle: SharedHeapReference,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
) -> Result<Value, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    if !state.shared().is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let field_count_for_error = field_count_for_error(field_count, index as usize + 1);
    let handle = handle
        .add_bytes(field.byte_offset)
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: field_count_for_error,
        })?;

    Ok(Value::shared_heap_reference(handle))
}

/// Get the address of a field from a raw pointer.
#[inline(always)]
pub(crate) fn field_addr_raw(
    state: &mut StepState<'_, '_>,
    pointer: RawPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
) -> Result<Value, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let field_count_for_error = field_count_for_error(field_count, index as usize + 1);
    let pointer = pointer
        .add_bytes(field.byte_offset)
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: field_count_for_error,
        })?;

    Ok(Value::raw_pointer(pointer))
}

/// Get the address of a field from a stack pointer.
#[inline(always)]
pub(crate) fn field_addr_stack(
    state: &mut StepState<'_, '_>,
    pointer: StackPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
) -> Result<Value, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    // validate the field byte range inside the stack allocation
    let frame = state.frame_by_index(pointer.frame_idx)?;
    let allocation = frame
        .stack_allocation(pointer.slot)
        .ok_or(Error::InvalidHeapReference)?;
    let slot_index =
        pointer
            .byte_offset
            .checked_add(field.byte_offset)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: allocation.len(),
            })?;
    let byte_end = slot_index
        .checked_add(field.byte_len)
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: allocation.len(),
        })?;

    if byte_end > allocation.len() {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: allocation.len(),
        });
    }

    stack_pointer_value(StackPointer::with_offset(
        pointer.frame_idx,
        pointer.slot,
        slot_index,
    ))
}

/// Get the address of a field from a frame pointer.
#[inline(always)]
pub(crate) fn field_addr_local(
    state: &mut StepState<'_, '_>,
    pointer: FramePointer,
    index: u32,
    field_count: Option<u32>,
) -> Result<Value, Error> {
    // load the local value
    let value = load_frame_slot(state, pointer, pointer.byte_offset)?;

    // resolve the field address from the value
    field_addr(state, value, index, field_count)
}

/// Get the address of a field from a global pointer.
#[inline(always)]
pub(crate) fn field_addr_global(
    state: &mut StepState<'_, '_>,
    pointer: GlobalPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
) -> Result<Value, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    if pointer.byte_offset != 0 {
        return Err(Error::InvalidFieldAccess {
            index: pointer.byte_offset as u32,
            field_count: 0,
        });
    }

    let value = state
        .globals
        .get(pointer.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: pointer.id })?;

    if let Some(handle) = value.as_heap_reference() {
        return field_addr_heap(state, handle, field, index, field_count);
    }

    if let Some(handle) = value.as_shared_heap_reference() {
        return field_addr_shared_heap(state, handle, field, index, field_count);
    }

    if let Some(pointer) = value.as_raw_pointer() {
        return field_addr_raw(state, pointer, field, index, field_count);
    }

    Err(Error::InvalidFieldAccess {
        index,
        field_count: 0,
    })
}

/// Get the address of an element from an array or pointer.
#[inline(always)]
pub(crate) fn element_addr(
    state: &mut StepState<'_, '_>,
    array: Value,
    index: u64,
    array_length: Option<u64>,
) -> Result<Value, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    // resolve the source and compute the element pointer
    match array.tag() {
        ValueTag::HeapReference => Err(invalid_pointer_description(
            "heap reference requires typed element access",
        )),
        ValueTag::SharedHeapReference => Err(invalid_pointer_description(
            "shared heap reference requires typed element access",
        )),
        ValueTag::RawPointer => Err(invalid_pointer_description(
            "raw pointer requires typed element access",
        )),
        ValueTag::StackPointer => Err(invalid_pointer_description(
            "stack pointer requires typed element access",
        )),
        ValueTag::FramePointer => {
            let pointer = frame_pointer_from_value(array)?;
            element_addr_local(state, pointer, index, array_length)
        }
        ValueTag::GlobalPointer => Err(invalid_pointer_description(
            "global pointer requires typed element access",
        )),
        _ => Err(Error::TypeMismatch {
            expected: "array or pointer".to_string(),
            actual: format!("{array:?}"),
        }),
    }
}

/// Get the address of an element from a heap reference.
#[inline(always)]
pub(crate) fn element_addr_heap(
    state: &mut StepState<'_, '_>,
    handle: HeapReference,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
) -> Result<Value, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    let heap = state.heap();
    if !heap.is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let handle = handle
        .add_bytes(element_offset)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;

    Ok(Value::heap_reference(handle))
}

/// Get the address of an element from a shared heap reference.
#[inline(always)]
pub(crate) fn element_addr_shared_heap(
    state: &mut StepState<'_, '_>,
    handle: SharedHeapReference,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
) -> Result<Value, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    if !state.shared().is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let handle = handle
        .add_bytes(element_offset)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;

    Ok(Value::shared_heap_reference(handle))
}

/// Get the address of an element from a raw pointer.
#[inline(always)]
pub(crate) fn element_addr_raw(
    state: &mut StepState<'_, '_>,
    pointer: RawPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
) -> Result<Value, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // resolve byte offset
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let pointer = pointer
        .add_bytes(element_offset)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;

    Ok(Value::raw_pointer(pointer))
}

/// Get the address of an element from a stack pointer.
#[inline(always)]
pub(crate) fn element_addr_stack(
    state: &mut StepState<'_, '_>,
    pointer: StackPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
) -> Result<Value, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    // validate the element byte range inside the stack allocation
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let frame = state.frame_by_index(pointer.frame_idx)?;
    let allocation = frame
        .stack_allocation(pointer.slot)
        .ok_or(Error::InvalidHeapReference)?;
    let slot_index =
        pointer
            .byte_offset
            .checked_add(element_offset)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: allocation.len() as u64,
            })?;
    let byte_end = slot_index
        .checked_add(element.byte_len)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: allocation.len() as u64,
        })?;

    if byte_end > allocation.len() {
        return Err(Error::InvalidArrayAccess {
            index,
            length: allocation.len() as u64,
        });
    }

    stack_pointer_value(StackPointer::with_offset(
        pointer.frame_idx,
        pointer.slot,
        slot_index,
    ))
}

/// Get the address of an element from a frame pointer.
#[inline(always)]
pub(crate) fn element_addr_local(
    state: &mut StepState<'_, '_>,
    pointer: FramePointer,
    index: u64,
    array_length: Option<u64>,
) -> Result<Value, Error> {
    // load the local value
    let value = load_frame_slot(state, pointer, pointer.byte_offset)?;

    // resolve the element address from the value
    element_addr(state, value, index, array_length)
}

/// Get the address of an element from a global pointer.
#[inline(always)]
pub(crate) fn element_addr_global(
    state: &mut StepState<'_, '_>,
    pointer: GlobalPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
) -> Result<Value, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    if pointer.byte_offset != 0 {
        return Err(Error::InvalidArrayAccess { index, length: 0 });
    }

    let value = state
        .globals
        .get(pointer.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: pointer.id })?;

    if let Some(handle) = value.as_heap_reference() {
        return element_addr_heap(state, handle, element, index, array_length);
    }

    if let Some(handle) = value.as_shared_heap_reference() {
        return element_addr_shared_heap(state, handle, element, index, array_length);
    }

    if let Some(pointer) = value.as_raw_pointer() {
        return element_addr_raw(state, pointer, element, index, array_length);
    }

    Err(Error::InvalidArrayAccess { index, length: 0 })
}

/// Load a field from a heap allocation.
#[inline(always)]
pub(crate) fn load_field_heap(
    state: &mut StepState<'_, '_>,
    handle: HeapReference,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.engine.statistics, loads);
    }

    // require one live heap allocation
    let heap = state.heap();
    if !heap.is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    // validate the requested field before decoding byte storage
    check_field_index(state, index, field_count)?;

    // reject null handles before reading the allocation bytes
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    if !field.is_scalar {
        return Err(non_scalar_load_error(field.value_type));
    }

    // resolve the field bytes inside the heap storage
    let owned_bytes = state
        .read_heap_bytes(handle, field.byte_offset, field.byte_len)
        .map_err(Error::from)
        .map_err(|error| {
            map_invalid_field_reference(error, index, field_count_for_error(field_count, 0))
        })?;

    decode_raw_value(state.tree(), field.value_type, &owned_bytes)
}

/// Load a field from a shared heap allocation.
#[inline(always)]
pub(crate) fn load_field_shared_heap(
    state: &mut StepState<'_, '_>,
    handle: SharedHeapReference,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.engine.statistics, loads);
    }

    if !state.shared().is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    check_field_index(state, index, field_count)?;

    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    if !field.is_scalar {
        return Err(non_scalar_load_error(field.value_type));
    }

    let owned_bytes = state
        .read_shared_heap_bytes(handle, field.byte_offset, field.byte_len)
        .map_err(Error::from)
        .map_err(|error| {
            map_invalid_field_reference(error, index, field_count_for_error(field_count, 0))
        })?;

    decode_raw_value(state.tree(), field.value_type, &owned_bytes)
}

/// Store a field into a heap allocation.
#[inline(always)]
pub(crate) fn store_field_heap(
    state: &mut StepState<'_, '_>,
    handle: HeapReference,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
    value: Value,
) -> Result<(), Error> {
    let bounds_checks = state.bounds_checks;
    let null_checks = state.null_checks;

    // track pointer stores
    if state.collect_stats {
        stat_inc!(state.engine.statistics, stores);
    }

    // require one live heap allocation
    let heap = state.heap();
    if !heap.is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    // validate the requested field before encoding byte storage
    check_field_index(state, index, field_count)?;

    // reject null handles before writing the allocation bytes
    if null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // encode the field payload into the managed byte storage
    let bytes = encode_value_bytes(state, field.value_type, value)?;
    let byte_len = state.heap().heap_byte_len(handle)?;
    let start = field.byte_offset;
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

    // write the encoded field bytes
    state
        .write_heap_bytes(handle, start, &bytes)
        .map_err(Error::from)?;

    Ok(())
}

/// Store a field into a shared heap allocation.
#[inline(always)]
pub(crate) fn store_field_shared_heap(
    state: &mut StepState<'_, '_>,
    handle: SharedHeapReference,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
    value: Value,
) -> Result<(), Error> {
    let bounds_checks = state.bounds_checks;
    let null_checks = state.null_checks;

    if state.collect_stats {
        stat_inc!(state.engine.statistics, stores);
    }

    if !state.shared().is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    check_field_index(state, index, field_count)?;

    if null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let bytes = encode_value_bytes(state, field.value_type, value)?;
    let byte_len = state
        .shared_ref()
        .heap_byte_len(handle)
        .map_err(Error::from)?;
    let start = field.byte_offset;
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

    state
        .shared()
        .write_heap_bytes(handle, start, &bytes)
        .map_err(Error::from)?;
    publish_shared_store(state, handle, start, &bytes, value)?;

    Ok(())
}

/// Load a field from a raw heap allocation.
#[inline(always)]
pub(crate) fn load_field_raw(
    state: &mut StepState<'_, '_>,
    pointer: RawPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.engine.statistics, loads);
    }

    // validate field index when known
    check_field_index(state, index, field_count)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let pointer = pointer
        .add_bytes(field.byte_offset)
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: field_count_for_error(field_count, index as usize + 1),
        })?;
    let pointer = Value::raw_pointer(pointer);
    let access = TypedAccess::from(field);

    load_from_raw_pointer_typed(state, pointer, access)
}

/// Store a field into a raw heap allocation.
#[inline(always)]
pub(crate) fn store_field_raw(
    state: &mut StepState<'_, '_>,
    pointer: RawPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
    value: Value,
) -> Result<(), Error> {
    // track pointer stores
    if state.collect_stats {
        stat_inc!(state.engine.statistics, stores);
    }

    // validate field index when known
    check_field_index(state, index, field_count)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let pointer = pointer
        .add_bytes(field.byte_offset)
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: field_count_for_error(field_count, index as usize + 1),
        })?;
    let pointer = Value::raw_pointer(pointer);
    let access = TypedAccess::from(field);

    store_to_raw_pointer_typed(state, pointer, access, value)
}

/// Load a field from a stack allocation.
#[inline(always)]
pub(crate) fn load_field_stack(
    state: &mut StepState<'_, '_>,
    pointer: StackPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.engine.statistics, loads);
    }

    // validate field index when known
    check_field_index(state, index, field_count)?;

    if !field.is_scalar {
        return Err(non_scalar_load_error(field.value_type));
    }

    let frame = state.frame_by_index(pointer.frame_idx)?;
    let allocation = frame
        .stack_allocation(pointer.slot)
        .ok_or(Error::InvalidHeapReference)?;
    let start =
        pointer
            .byte_offset
            .checked_add(field.byte_offset)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: allocation.len(),
            })?;
    let end = start
        .checked_add(field.byte_len)
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: allocation.len(),
        })?;
    let byte_range = allocation
        .bytes()
        .get(start..end)
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: allocation.len(),
        })?;

    decode_raw_value(state.tree(), field.value_type, byte_range)
}

/// Store a field into a stack allocation.
#[inline(always)]
pub(crate) fn store_field_stack(
    state: &mut StepState<'_, '_>,
    pointer: StackPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
    value: Value,
) -> Result<(), Error> {
    // track pointer stores
    if state.collect_stats {
        stat_inc!(state.engine.statistics, stores);
    }

    // validate field index when known
    check_field_index(state, index, field_count)?;

    // encode the typed field payload
    let bytes = encode_value_bytes(state, field.value_type, value)?;

    // write the field bytes into the stack allocation
    let frame = state.frame_by_index_mut(pointer.frame_idx)?;
    let allocation = frame
        .stack_allocation_mut(pointer.slot)
        .ok_or(Error::InvalidHeapReference)?;
    let start =
        pointer
            .byte_offset
            .checked_add(field.byte_offset)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: allocation.len(),
            })?;
    let end = start
        .checked_add(bytes.len())
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: allocation.len(),
        })?;

    if end > allocation.len() {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: allocation.len(),
        });
    }

    allocation.bytes_mut()[start..end].copy_from_slice(&bytes);

    Ok(())
}

/// Load a field from a global allocation.
#[inline(always)]
pub(crate) fn load_field_global(
    state: &mut StepState<'_, '_>,
    pointer: GlobalPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.engine.statistics, loads);
    }

    // validate field index when known
    check_field_index(state, index, field_count)?;

    if pointer.byte_offset != 0 {
        return Err(Error::InvalidFieldAccess {
            index: pointer.byte_offset as u32,
            field_count: 0,
        });
    }

    // load the global value
    let value = state
        .globals
        .get(pointer.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: pointer.id })?;

    if let Some(handle) = value.as_heap_reference() {
        return load_field_heap(state, handle, field, index, field_count);
    }

    if let Some(handle) = value.as_shared_heap_reference() {
        return load_field_shared_heap(state, handle, field, index, field_count);
    }

    Err(Error::InvalidFieldAccess {
        index,
        field_count: 0,
    })
}

/// Store a field into a global allocation.
#[inline(always)]
pub(crate) fn store_field_global(
    state: &mut StepState<'_, '_>,
    pointer: GlobalPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
    value: Value,
) -> Result<(), Error> {
    // track pointer stores
    if state.collect_stats {
        stat_inc!(state.engine.statistics, stores);
    }

    // validate field index when known
    check_field_index(state, index, field_count)?;

    if pointer.byte_offset != 0 {
        return Err(Error::InvalidFieldAccess {
            index: pointer.byte_offset as u32,
            field_count: 0,
        });
    }

    // load the global value
    let current = state
        .globals
        .get(pointer.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: pointer.id })?;

    if let Some(handle) = current.as_heap_reference() {
        store_field_heap(state, handle, field, index, field_count, value)?;
        state.globals.set(pointer.id, current);
        return Ok(());
    }

    if let Some(handle) = current.as_shared_heap_reference() {
        store_field_shared_heap(state, handle, field, index, field_count, value)?;
        state.globals.set(pointer.id, current);
        return Ok(());
    }

    Err(Error::InvalidFieldAccess {
        index,
        field_count: 0,
    })
}

/// Load an element from a heap allocation.
#[inline(always)]
pub(crate) fn load_element_heap(
    state: &mut StepState<'_, '_>,
    handle: HeapReference,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.engine.statistics, loads);
    }

    // require one live heap allocation
    let heap = state.heap();
    if !heap.is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    // validate the requested element before decoding byte storage
    check_array_index(state, index, array_length)?;

    // reject null handles before reading the allocation bytes
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    if !element.is_scalar {
        return Err(non_scalar_load_error(element.value_type));
    }

    // resolve the element bytes inside the heap storage
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let owned_bytes = state
        .read_heap_bytes(handle, element_offset, element.byte_len)
        .map_err(Error::from)
        .map_err(|error| {
            map_invalid_array_reference(error, index, array_length_for_error(array_length, 0))
        })?;

    decode_raw_value(state.tree(), element.value_type, &owned_bytes)
}

/// Load an element from a shared heap allocation.
#[inline(always)]
pub(crate) fn load_element_shared_heap(
    state: &mut StepState<'_, '_>,
    handle: SharedHeapReference,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
) -> Result<Value, Error> {
    if state.collect_stats {
        stat_inc!(state.engine.statistics, loads);
    }

    if !state.shared().is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    check_array_index(state, index, array_length)?;

    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    if !element.is_scalar {
        return Err(non_scalar_load_error(element.value_type));
    }

    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let owned_bytes = state
        .read_shared_heap_bytes(handle, element_offset, element.byte_len)
        .map_err(Error::from)
        .map_err(|error| {
            map_invalid_array_reference(error, index, array_length_for_error(array_length, 0))
        })?;

    decode_raw_value(state.tree(), element.value_type, &owned_bytes)
}

/// Store an element into a heap allocation.
#[inline(always)]
pub(crate) fn store_element_heap(
    state: &mut StepState<'_, '_>,
    handle: HeapReference,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
    value: Value,
) -> Result<(), Error> {
    let bounds_checks = state.bounds_checks;
    let null_checks = state.null_checks;

    // track pointer stores
    if state.collect_stats {
        stat_inc!(state.engine.statistics, stores);
    }

    // require one live heap allocation
    let heap = state.heap();
    if !heap.is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    // validate the requested element before encoding byte storage
    check_array_index(state, index, array_length)?;

    // reject null handles before writing the allocation bytes
    if null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // encode the element payload into the managed byte storage
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let bytes = encode_value_bytes(state, element.value_type, value)?;
    let allocation_len = state.heap().heap_byte_len(handle)?;
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

    // write the encoded element bytes
    state
        .write_heap_bytes(handle, start, &bytes)
        .map_err(Error::from)?;

    Ok(())
}

/// Store an element into a shared heap allocation.
#[inline(always)]
pub(crate) fn store_element_shared_heap(
    state: &mut StepState<'_, '_>,
    handle: SharedHeapReference,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
    value: Value,
) -> Result<(), Error> {
    let bounds_checks = state.bounds_checks;
    let null_checks = state.null_checks;

    if state.collect_stats {
        stat_inc!(state.engine.statistics, stores);
    }

    if !state.shared().is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    check_array_index(state, index, array_length)?;

    if null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let bytes = encode_value_bytes(state, element.value_type, value)?;
    let allocation_len = state
        .shared_ref()
        .heap_byte_len(handle)
        .map_err(Error::from)?;
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

    state
        .shared()
        .write_heap_bytes(handle, start, &bytes)
        .map_err(Error::from)?;
    publish_shared_store(state, handle, start, &bytes, value)?;

    Ok(())
}

/// Load an element from a raw heap allocation.
#[inline(always)]
pub(crate) fn load_element_raw(
    state: &mut StepState<'_, '_>,
    pointer: RawPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.engine.statistics, loads);
    }

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let pointer = pointer
        .add_bytes(element_offset)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let pointer = Value::raw_pointer(pointer);
    let access = TypedAccess::from(element);

    load_from_raw_pointer_typed(state, pointer, access)
}

/// Store an element into a raw heap allocation.
#[inline(always)]
pub(crate) fn store_element_raw(
    state: &mut StepState<'_, '_>,
    pointer: RawPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
    value: Value,
) -> Result<(), Error> {
    // track pointer stores
    if state.collect_stats {
        stat_inc!(state.engine.statistics, stores);
    }

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let pointer = pointer
        .add_bytes(element_offset)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let pointer = Value::raw_pointer(pointer);
    let access = TypedAccess::from(element);

    store_to_raw_pointer_typed(state, pointer, access, value)
}

/// Load an element from a stack allocation.
#[inline(always)]
pub(crate) fn load_element_stack(
    state: &mut StepState<'_, '_>,
    pointer: StackPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.engine.statistics, loads);
    }

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // resolve the typed element bytes
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    if !element.is_scalar {
        return Err(non_scalar_load_error(element.value_type));
    }

    let frame = state.frame_by_index(pointer.frame_idx)?;
    let allocation = frame
        .stack_allocation(pointer.slot)
        .ok_or(Error::InvalidHeapReference)?;
    let start =
        pointer
            .byte_offset
            .checked_add(element_offset)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: allocation.len() as u64,
            })?;
    let end = start
        .checked_add(element.byte_len)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: allocation.len() as u64,
        })?;
    let byte_range = allocation
        .bytes()
        .get(start..end)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: allocation.len() as u64,
        })?;

    decode_raw_value(state.tree(), element.value_type, byte_range)
}

/// Store an element into a stack allocation.
#[inline(always)]
pub(crate) fn store_element_stack(
    state: &mut StepState<'_, '_>,
    pointer: StackPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
    value: Value,
) -> Result<(), Error> {
    // track pointer stores
    if state.collect_stats {
        stat_inc!(state.engine.statistics, stores);
    }

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // compute the element offset before taking mutable borrows
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;

    // encode the typed element payload
    let bytes = encode_value_bytes(state, element.value_type, value)?;

    // write the element bytes into the stack allocation
    let frame = state.frame_by_index_mut(pointer.frame_idx)?;
    let allocation = frame
        .stack_allocation_mut(pointer.slot)
        .ok_or(Error::InvalidHeapReference)?;
    let start =
        pointer
            .byte_offset
            .checked_add(element_offset)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: allocation.len() as u64,
            })?;
    let end = start
        .checked_add(bytes.len())
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: allocation.len() as u64,
        })?;

    if end > allocation.len() {
        return Err(Error::InvalidArrayAccess {
            index,
            length: allocation.len() as u64,
        });
    }

    allocation.bytes_mut()[start..end].copy_from_slice(&bytes);

    Ok(())
}

/// Load an element from a global allocation.
#[inline(always)]
pub(crate) fn load_element_global(
    state: &mut StepState<'_, '_>,
    pointer: GlobalPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.engine.statistics, loads);
    }

    // validate array index when known
    check_array_index(state, index, array_length)?;

    if pointer.byte_offset != 0 {
        return Err(Error::InvalidArrayAccess { index, length: 0 });
    }

    // load the global value
    let value = state
        .globals
        .get(pointer.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: pointer.id })?;

    if let Some(handle) = value.as_heap_reference() {
        return load_element_heap(state, handle, element, index, array_length);
    }

    if let Some(handle) = value.as_shared_heap_reference() {
        return load_element_shared_heap(state, handle, element, index, array_length);
    }

    Err(Error::InvalidArrayAccess { index, length: 0 })
}

/// Store an element into a global allocation.
#[inline(always)]
pub(crate) fn store_element_global(
    state: &mut StepState<'_, '_>,
    pointer: GlobalPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
    value: Value,
) -> Result<(), Error> {
    // track pointer stores
    if state.collect_stats {
        stat_inc!(state.engine.statistics, stores);
    }

    // validate array index when known
    check_array_index(state, index, array_length)?;

    if pointer.byte_offset != 0 {
        return Err(Error::InvalidArrayAccess { index, length: 0 });
    }

    // load the global value
    let current = state
        .globals
        .get(pointer.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: pointer.id })?;

    if let Some(handle) = current.as_heap_reference() {
        store_element_heap(state, handle, element, index, array_length, value)?;
        state.globals.set(pointer.id, current);
        return Ok(());
    }

    if let Some(handle) = current.as_shared_heap_reference() {
        store_element_shared_heap(state, handle, element, index, array_length, value)?;
        state.globals.set(pointer.id, current);
        return Ok(());
    }

    Err(Error::InvalidArrayAccess { index, length: 0 })
}

/// Get a field from an composite value.
#[inline(always)]
pub(crate) fn get_field(
    state: &mut StepState<'_, '_>,
    agg: Value,
    index: u32,
    field_count: Option<u32>,
    field: Option<FieldAccess>,
) -> Result<Value, Error> {
    let Some(field) = field else {
        return Err(Error::TypeMismatch {
            expected: "typed field access".to_string(),
            actual: format!("{agg:?}"),
        });
    };

    // resolve composite value
    match agg.tag() {
        ValueTag::HeapReference => {
            let handle = heap_reference_from_value(agg)?;
            load_field_heap(state, handle, field, index, field_count)
        }
        ValueTag::SharedHeapReference => {
            let handle = shared_heap_reference_from_value(agg)?;
            load_field_shared_heap(state, handle, field, index, field_count)
        }
        ValueTag::StackPointer => {
            let pointer = stack_pointer_from_value(agg)?;
            load_field_stack(state, pointer, field, index, field_count)
        }
        _ => Err(Error::TypeMismatch {
            expected: "composite".to_string(),
            actual: format!("{agg:?}"),
        }),
    }
}

/// Set a field on an composite value.
#[inline(always)]
pub(crate) fn set_field(
    state: &mut StepState<'_, '_>,
    agg: Value,
    index: u32,
    val: Value,
    aggregate_type: mir::LocalNodeId<mir::Type>,
    field_count: Option<u32>,
    field: Option<FieldAccess>,
) -> Result<Value, Error> {
    let Some(field) = field else {
        return Err(Error::TypeMismatch {
            expected: "typed field access".to_string(),
            actual: format!("{agg:?}"),
        });
    };

    let copied = duplicate_composite_value_to_heap(state, agg, aggregate_type, "composite")?;
    let handle = heap_reference_from_value(copied)?;

    store_field_heap(state, handle, field, index, field_count, val)?;

    Ok(copied)
}

/// Get an element from an array value.
#[inline(always)]
pub(crate) fn get_element(
    state: &mut StepState<'_, '_>,
    arr: Value,
    index: u64,
    array_length: Option<u64>,
    element: Option<ElementAccess>,
) -> Result<Value, Error> {
    let Some(element) = element else {
        return Err(Error::TypeMismatch {
            expected: "typed element access".to_string(),
            actual: format!("{arr:?}"),
        });
    };

    // resolve array value
    match arr.tag() {
        ValueTag::HeapReference => {
            let handle = heap_reference_from_value(arr)?;
            load_element_heap(state, handle, element, index, array_length)
        }
        ValueTag::SharedHeapReference => {
            let handle = shared_heap_reference_from_value(arr)?;
            load_element_shared_heap(state, handle, element, index, array_length)
        }
        ValueTag::StackPointer => {
            let pointer = stack_pointer_from_value(arr)?;
            load_element_stack(state, pointer, element, index, array_length)
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
    state: &mut StepState<'_, '_>,
    arr: Value,
    index: u64,
    val: Value,
    aggregate_type: mir::LocalNodeId<mir::Type>,
    array_length: Option<u64>,
    element: Option<ElementAccess>,
) -> Result<Value, Error> {
    let Some(element) = element else {
        return Err(Error::TypeMismatch {
            expected: "typed element access".to_string(),
            actual: format!("{arr:?}"),
        });
    };

    let copied = duplicate_composite_value_to_heap(state, arr, aggregate_type, "array")?;
    let handle = heap_reference_from_value(copied)?;

    store_element_heap(state, handle, element, index, array_length, val)?;

    Ok(copied)
}

/// Duplicate one whole composite or array value into a fresh heap object.
fn duplicate_composite_value_to_heap(
    state: &mut StepState<'_, '_>,
    value: Value,
    aggregate_type: mir::LocalNodeId<mir::Type>,
    expected: &'static str,
) -> Result<Value, Error> {
    let bytes = match value.tag() {
        ValueTag::HeapReference => {
            let handle = heap_reference_from_value(value)?;

            state.heap().read_heap_bytes(handle)?
        }
        ValueTag::SharedHeapReference => {
            let handle = shared_heap_reference_from_value(value)?;

            state
                .shared()
                .read_heap_bytes(handle)
                .map_err(Error::from)?
        }
        _ => {
            return Err(Error::TypeMismatch {
                expected: expected.to_string(),
                actual: format!("{value:?}"),
            });
        }
    };

    let layout_id = state
        .module
        .layout_id_for_type(aggregate_type)
        .ok_or(Error::InvalidInstruction)?;
    let reference = state
        .heap_mut()
        .allocate(layout_id, Payload::Bytes(&bytes))
        .map_err(Error::from)?;

    Ok(Value::heap_reference(reference))
}

/// Load from a global pointer, including slot offsets.
#[inline(always)]
fn load_global_slot(state: &mut StepState<'_, '_>, global: GlobalPointer) -> Result<Value, Error> {
    // read the global value
    let value = state
        .globals
        .get(global.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: global.id })?;

    // return the global value directly
    if global.byte_offset == 0 {
        return Ok(value);
    }

    Err(invalid_pointer_description(
        "global pointer offset requires typed access",
    ))
}

/// Store through a global pointer, including slot offsets.
#[inline(always)]
fn store_global_slot(
    state: &mut StepState<'_, '_>,
    global: GlobalPointer,
    value: Value,
) -> Result<(), Error> {
    // update the global value directly
    if global.byte_offset == 0 {
        state.globals.set(global.id, value);
        return Ok(());
    }

    Err(invalid_pointer_description(
        "global pointer offset requires typed access",
    ))
}

/// Load a local slot from a frame.
#[inline(always)]
fn load_frame_slot(
    state: &mut StepState<'_, '_>,
    pointer: FramePointer,
    byte_offset: usize,
) -> Result<Value, Error> {
    // reject non zero offsets
    if byte_offset != 0 {
        return Err(Error::InvalidFieldAccess {
            index: byte_offset as u32,
            field_count: 1,
        });
    }

    // resolve the target frame
    let frame = state.frame_by_index(pointer.frame_idx)?;

    // resolve the local id
    let local = mir::LocalNodeId::new(pointer.slot as u32);

    // read the local slot
    frame.get_local_or_error(local)
}

/// Store a local slot into a frame.
#[inline(always)]
fn store_frame_slot(
    state: &mut StepState<'_, '_>,
    pointer: FramePointer,
    byte_offset: usize,
    value: Value,
) -> Result<(), Error> {
    // reject non zero offsets
    if byte_offset != 0 {
        return Err(Error::InvalidFieldAccess {
            index: byte_offset as u32,
            field_count: 1,
        });
    }

    // resolve the target frame
    // resolve the local id
    let local = mir::LocalNodeId::new(pointer.slot as u32);
    let frame = state.frame_by_index_mut(pointer.frame_idx)?;
    frame.set_local(local, value);

    Ok(())
}
