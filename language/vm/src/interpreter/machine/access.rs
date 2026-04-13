use crate::diagnostic::Error;
use crate::executable::{
    ElementAccess, FieldAccess, TypedAccess, UNKNOWN_ARRAY_LENGTH, UNKNOWN_FIELD_COUNT,
};
use destack_heap::{
    GlobalPointer, LocalPointer, ManagedReference, RawPointer, ReferenceMeta, SharedPointer,
    StackPointer, Value, ValueTag,
};
use destack_mir as mir;

use super::super::state::{StackAllocation, StepState};
use super::storage::*;
use crate::telemetry::stat_inc;

pub(crate) use super::storage::{
    allocate_function_value, allocate_stack_storage_value_by_index, allocate_zeroed_stack_storage,
    clone_typed_storage_bytes_from_value, decode_function_value, decode_raw_value,
    encode_raw_value, encode_storage_value, load_from_raw_pointer_typed, managed_storage_type,
    materialize_value_from_storage, raw_type_size, store_to_raw_pointer_typed,
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

/// Require one managed reference value.
#[inline(always)]
fn managed_reference_from_value(value: Value) -> Result<ManagedReference, Error> {
    value
        .as_managed_reference()
        .ok_or_else(|| invalid_pointer_type(value))
}

/// Require one stack pointer value.
#[inline(always)]
fn stack_pointer_from_value(value: Value) -> Result<StackPointer, Error> {
    value
        .as_stack_pointer()
        .ok_or_else(|| invalid_pointer_type(value))
}

/// Require one local pointer value.
#[inline(always)]
fn local_pointer_from_value(value: Value) -> Result<LocalPointer, Error> {
    value
        .as_local_pointer()
        .ok_or_else(|| invalid_pointer_type(value))
}

/// Require one global pointer value.
#[inline(always)]
fn global_pointer_from_value(value: Value) -> Result<GlobalPointer, Error> {
    value
        .as_global_pointer()
        .ok_or_else(|| invalid_pointer_type(value))
}

/// Return the semantic component count for one managed composite.
#[inline(always)]
fn managed_component_count(
    state: &StepState<'_, '_>,
    handle: ManagedReference,
) -> Result<usize, Error> {
    let composite_type = managed_storage_type(state, handle)?;

    state.storage_component_count(composite_type)
}

/// Decode one raw bit pattern into a pointer-shaped VM value.
pub(crate) fn decode_pointer_bits(raw: u64, target_type: &mir::Type) -> Value {
    match target_type {
        mir::Type::FunctionPointer { .. } => {
            Value::function_pointer(mir::LocalNodeId::new(raw as u32))
        }
        mir::Type::Reference {
            kind,
            address_space,
            mutability,
            is_nullable,
            ..
        } => {
            let meta = ReferenceMeta::new(*kind, *address_space, *mutability, *is_nullable);
            match (*kind, *address_space) {
                (mir::ReferenceKind::Managed, _) => {
                    Value::managed_reference_with_meta(ManagedReference::from_bits(raw), meta)
                }
                (_, mir::AddressSpace::Shared) => {
                    Value::shared_pointer_with_meta(SharedPointer::from_bits(raw), meta)
                }
                (_, mir::AddressSpace::Stack) => {
                    let frame_index = (raw & STACK_INDEX_MASK) as usize;
                    let slot = ((raw >> STACK_SLOT_SHIFT) & STACK_INDEX_MASK) as usize;
                    let slot_offset = ((raw >> POINTER_SLOT_SHIFT) & POINTER_BASE_MASK) as usize;
                    let pointer = StackPointer::with_offset(frame_index, slot, slot_offset);

                    Value::stack_pointer_with_meta(pointer, meta)
                }
                (_, mir::AddressSpace::Local) => {
                    let frame_index = (raw & STACK_INDEX_MASK) as usize;
                    let local = ((raw >> STACK_SLOT_SHIFT) & STACK_INDEX_MASK) as usize;
                    let slot_offset = ((raw >> POINTER_SLOT_SHIFT) & POINTER_BASE_MASK) as usize;
                    let pointer = LocalPointer::with_offset(frame_index, local, slot_offset);

                    Value::local_pointer_with_meta(pointer, meta)
                }
                (_, mir::AddressSpace::Global | mir::AddressSpace::Constant) => {
                    let global = mir::LocalNodeId::new((raw & POINTER_BASE_MASK) as u32);
                    let slot_offset = ((raw >> POINTER_SLOT_SHIFT) & POINTER_BASE_MASK) as usize;

                    Value::global_pointer_with_meta(global, slot_offset, meta)
                }
                _ => Value::raw_pointer_with_meta(RawPointer::from_bits(raw), meta),
            }
        }
        _ => Value::raw_pointer(RawPointer::from_bits(raw)),
    }
}

/// Load a value from a pointer with one optional typed access descriptor.
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
        ValueTag::ManagedReference => {
            // use the compiled typed access when the pointee is known
            if let Some(access) = access {
                return load_from_managed_reference_typed(state, ptr, access);
            }

            Err(invalid_pointer_description(
                "managed reference without pointee type",
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
        ValueTag::LocalPointer => {
            // load one local slot directly
            let lp = local_pointer_from_value(ptr)?;
            load_local_slot(state, lp, lp.slot_offset)
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

/// Store a value to a pointer with one optional typed access descriptor.
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
        ValueTag::ManagedReference => {
            // use the compiled typed access when the pointee is known
            if let Some(access) = access {
                return store_to_managed_reference_typed(state, ptr, access, value);
            }

            Err(invalid_pointer_description(
                "managed reference without pointee type",
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
        ValueTag::LocalPointer => {
            // store one local slot directly
            let lp = local_pointer_from_value(ptr)?;
            store_local_slot(state, lp, lp.slot_offset, value)
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

/// Load a typed value from a managed reference.
#[inline(always)]
pub(crate) fn load_from_managed_reference_typed(
    state: &mut StepState<'_, '_>,
    ptr: Value,
    access: TypedAccess,
) -> Result<Value, Error> {
    // require one managed reference value
    if ptr.tag() != ValueTag::ManagedReference {
        return Err(invalid_pointer_type(ptr));
    }

    // reject null pointers before reading the allocation
    let handle = managed_reference_from_value(ptr)?;
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // require one live managed allocation
    let heap = state.heap();
    if !heap.is_managed_allocated(handle) {
        return Err(Error::InvalidManagedReference);
    }

    // decode the typed storage bytes
    let owned_window = {
        let bytes = heap
            .managed_bytes(handle)
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

/// Load a typed value from a stack pointer.
#[inline(always)]
pub(crate) fn load_from_stack_pointer_typed(
    state: &mut StepState<'_, '_>,
    pointer: StackPointer,
    access: TypedAccess,
) -> Result<Value, Error> {
    // resolve the typed byte window
    let owned_window = {
        let frame = state.frame_by_index(pointer.frame_idx)?;
        let allocation = frame
            .stack_allocation(pointer.slot)
            .ok_or(Error::InvalidManagedReference)?;
        let start = pointer.slot_offset;
        let end = start
            .checked_add(access.byte_len)
            .ok_or(Error::InvalidFieldAccess {
                index: start as u32,
                field_count: allocation.len(),
            })?;
        let window = allocation
            .bytes()
            .get(start..end)
            .ok_or(Error::InvalidFieldAccess {
                index: start as u32,
                field_count: allocation.len(),
            })?;

        if access.is_scalar {
            return decode_raw_value(state.tree(), access.value_type, window);
        }

        window.to_vec()
    };

    materialize_value_from_storage(state, access.value_type, &owned_window)
}

/// Load a value from a local pointer.
#[inline(always)]
pub(crate) fn load_from_local_pointer(
    state: &mut StepState<'_, '_>,
    ptr: Value,
) -> Result<Value, Error> {
    // validate pointer tag
    if ptr.tag() != ValueTag::LocalPointer {
        return Err(invalid_pointer_type(ptr));
    }

    // resolve pointer
    let lp = local_pointer_from_value(ptr)?;
    load_local_slot(state, lp, lp.slot_offset)
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

/// Store a typed value through a managed reference.
#[inline(always)]
pub(crate) fn store_to_managed_reference_typed(
    state: &mut StepState<'_, '_>,
    ptr: Value,
    access: TypedAccess,
    val: Value,
) -> Result<(), Error> {
    if ptr.tag() != ValueTag::ManagedReference {
        return Err(invalid_pointer_type(ptr));
    }

    let handle = managed_reference_from_value(ptr)?;
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    {
        let heap = state.heap();
        if !heap.is_managed_allocated(handle) {
            return Err(Error::InvalidManagedReference);
        }
    }

    if !access.is_scalar
        && let Some(source_bytes) =
            clone_typed_storage_bytes_from_value(state, val, access.value_type)?
    {
        if source_bytes.len() != access.byte_len {
            return Err(Error::InvalidManagedReference);
        }

        let byte_len = state
            .heap()
            .managed_byte_len(handle)
            .ok_or(Error::InvalidManagedReference)?;
        let start = 0usize;
        let end = source_bytes.len();

        if end > byte_len {
            return Err(Error::InvalidFieldAccess {
                index: start as u32,
                field_count: byte_len,
            });
        }

        if !state
            .heap_mut()
            .set_managed_bytes(handle, start, source_bytes.as_ref())
        {
            return Err(Error::InvalidFieldAccess {
                index: start as u32,
                field_count: byte_len,
            });
        }

        return Ok(());
    }

    let bytes = encode_storage_value(state, access.value_type, val)?;
    let byte_len = state
        .heap()
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

    if !state.heap_mut().set_managed_bytes(handle, start, &bytes) {
        return Err(Error::InvalidFieldAccess {
            index: start as u32,
            field_count: byte_len,
        });
    }

    Ok(())
}

/// Store a typed value through a stack pointer.
#[inline(always)]
pub(crate) fn store_to_stack_pointer_typed(
    state: &mut StepState<'_, '_>,
    pointer: StackPointer,
    access: TypedAccess,
    value: Value,
) -> Result<(), Error> {
    if !access.is_scalar
        && let Some(source_bytes) =
            clone_typed_storage_bytes_from_value(state, value, access.value_type)?
    {
        if source_bytes.len() != access.byte_len {
            return Err(Error::InvalidManagedReference);
        }

        let frame = state.frame_by_index_mut(pointer.frame_idx)?;
        let allocation = frame
            .stack_allocation_mut(pointer.slot)
            .ok_or(Error::InvalidManagedReference)?;
        let start = pointer.slot_offset;
        let end = start
            .checked_add(source_bytes.len())
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

        allocation.bytes_mut()[start..end].copy_from_slice(source_bytes.as_ref());
        return Ok(());
    }

    // encode the typed payload
    let bytes = encode_storage_value(state, access.value_type, value)?;

    let frame = state.frame_by_index_mut(pointer.frame_idx)?;
    let allocation = frame
        .stack_allocation_mut(pointer.slot)
        .ok_or(Error::InvalidManagedReference)?;
    let start = pointer.slot_offset;
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

/// Store a value through a local pointer.
#[inline(always)]
pub(crate) fn store_to_local_pointer(
    state: &mut StepState<'_, '_>,
    ptr: Value,
    val: Value,
) -> Result<(), Error> {
    // validate pointer tag
    if ptr.tag() != ValueTag::LocalPointer {
        return Err(invalid_pointer_type(ptr));
    }

    // resolve pointer
    let lp = local_pointer_from_value(ptr)?;
    store_local_slot(state, lp, lp.slot_offset, val)
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
    field_count: u32,
) -> Result<Value, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    // resolve the source and compute the field pointer
    match composite.tag() {
        ValueTag::ManagedReference => {
            let handle = managed_reference_from_value(composite)?;
            let composite_type = managed_storage_type(state, handle)?;
            let field = state.storage_component_layout(composite_type, index)?;
            let byte_offset = handle.byte_offset().checked_add(field.offset).ok_or(
                Error::InvalidFieldAccess {
                    index,
                    field_count: field_count as usize,
                },
            )?;

            Ok(Value::managed_reference(
                ManagedReference::with_byte_offset(handle.id(), byte_offset as u32),
            ))
        }
        ValueTag::RawPointer => Err(invalid_pointer_description(
            "raw pointer requires typed field access",
        )),
        ValueTag::StackPointer => Err(invalid_pointer_description(
            "stack pointer requires typed field access",
        )),
        ValueTag::LocalPointer => {
            let pointer = local_pointer_from_value(composite)?;
            field_addr_local(state, pointer, index, field_count)
        }
        ValueTag::GlobalPointer => {
            let global = global_pointer_from_value(composite)?;
            let slot_index = resolve_global_field_slot(state, global, index, field_count)?;
            Ok(Value::global_pointer_with_offset(global.id, slot_index))
        }
        _ => Err(Error::TypeMismatch {
            expected: "composite or pointer".to_string(),
            actual: format!("{composite:?}"),
        }),
    }
}

/// Get the address of a field from a managed reference.
#[inline(always)]
pub(crate) fn field_addr_managed(
    state: &mut StepState<'_, '_>,
    handle: ManagedReference,
    field: FieldAccess,
    index: u32,
    field_count: u32,
) -> Result<Value, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    let heap = state.heap();
    if !heap.is_managed_allocated(handle) {
        return Err(Error::InvalidManagedReference);
    }

    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let byte_offset =
        handle
            .byte_offset()
            .checked_add(field.byte_offset)
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
    state: &mut StepState<'_, '_>,
    pointer: RawPointer,
    field: FieldAccess,
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
    let byte_offset =
        pointer
            .byte_offset()
            .checked_add(field.byte_offset)
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
    state: &mut StepState<'_, '_>,
    pointer: StackPointer,
    field: FieldAccess,
    index: u32,
    field_count: u32,
) -> Result<Value, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    // validate the field window inside the stack allocation
    let frame = state.frame_by_index(pointer.frame_idx)?;
    let allocation = frame
        .stack_allocation(pointer.slot)
        .ok_or(Error::InvalidManagedReference)?;
    let slot_index =
        pointer
            .slot_offset
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

    Ok(Value::stack_pointer(StackPointer::with_offset(
        pointer.frame_idx,
        pointer.slot,
        slot_index,
    )))
}

/// Get the address of a field from a local pointer.
#[inline(always)]
pub(crate) fn field_addr_local(
    state: &mut StepState<'_, '_>,
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
    state: &mut StepState<'_, '_>,
    pointer: GlobalPointer,
    index: u32,
    field_count: u32,
) -> Result<Value, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    // ensure the global exists
    state
        .globals
        .get(pointer.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: pointer.id })?;

    // resolve composite slot offset
    let slot_index = resolve_global_field_slot(state, pointer, index, field_count)?;
    Ok(Value::global_pointer_with_offset(pointer.id, slot_index))
}

/// Get the address of an element from an array or pointer.
#[inline(always)]
pub(crate) fn element_addr(
    state: &mut StepState<'_, '_>,
    array: Value,
    index: u64,
    array_length: u64,
) -> Result<Value, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    // resolve the source and compute the element pointer
    match array.tag() {
        ValueTag::ManagedReference => {
            let handle = managed_reference_from_value(array)?;
            let slot_index = resolve_heap_element_slot(state, handle, index, array_length)?;
            let byte_offset = slot_index
                .checked_mul(Value::BYTE_LEN as u32)
                .ok_or(Error::InvalidManagedReference)?;
            let handle = ManagedReference::with_byte_offset(handle.id(), byte_offset);
            Ok(Value::managed_reference(handle))
        }
        ValueTag::RawPointer => Err(invalid_pointer_description(
            "raw pointer requires typed element access",
        )),
        ValueTag::StackPointer => Err(invalid_pointer_description(
            "stack pointer requires typed element access",
        )),
        ValueTag::LocalPointer => {
            let pointer = local_pointer_from_value(array)?;
            element_addr_local(state, pointer, index, array_length)
        }
        ValueTag::GlobalPointer => {
            let global = global_pointer_from_value(array)?;
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
    state: &mut StepState<'_, '_>,
    handle: ManagedReference,
    element: ElementAccess,
    index: u64,
    array_length: u64,
) -> Result<Value, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    let heap = state.heap();
    if !heap.is_managed_allocated(handle) {
        return Err(Error::InvalidManagedReference);
    }

    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
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
    state: &mut StepState<'_, '_>,
    pointer: RawPointer,
    element: ElementAccess,
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
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
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
    state: &mut StepState<'_, '_>,
    pointer: StackPointer,
    element: ElementAccess,
    index: u64,
    array_length: u64,
) -> Result<Value, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    // validate the element window inside the stack allocation
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length,
        })?;
    let frame = state.frame_by_index(pointer.frame_idx)?;
    let allocation = frame
        .stack_allocation(pointer.slot)
        .ok_or(Error::InvalidManagedReference)?;
    let slot_index =
        pointer
            .slot_offset
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

    Ok(Value::stack_pointer(StackPointer::with_offset(
        pointer.frame_idx,
        pointer.slot,
        slot_index,
    )))
}

/// Get the address of an element from a local pointer.
#[inline(always)]
pub(crate) fn element_addr_local(
    state: &mut StepState<'_, '_>,
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
    state: &mut StepState<'_, '_>,
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
    state: &mut StepState<'_, '_>,
    handle: ManagedReference,
    field: FieldAccess,
    index: u32,
    field_count: u32,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.engine.statistics, loads);
    }

    // require one live managed allocation
    let heap = state.heap();
    if !heap.is_managed_allocated(handle) {
        return Err(Error::InvalidManagedReference);
    }

    // validate the requested field before decoding byte storage
    check_field_index(state, index, field_count)?;

    // reject null handles before reading the allocation bytes
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // resolve the field window inside the managed byte storage
    let owned_window = {
        let bytes = heap
            .managed_bytes(handle)
            .ok_or(Error::InvalidManagedReference)?;
        let start = field.byte_offset;
        let end = start
            .checked_add(field.byte_len)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: bytes.len(),
            })?;
        let window = bytes.get(start..end).ok_or(Error::InvalidFieldAccess {
            index,
            field_count: bytes.len(),
        })?;

        if field.is_scalar {
            return decode_raw_value(state.tree(), field.value_type, window);
        }

        window.to_vec()
    };

    // decode the typed field payload
    materialize_value_from_storage(state, field.value_type, &owned_window)
}

/// Store a field into a managed heap allocation.
#[inline(always)]
pub(crate) fn store_field_managed(
    state: &mut StepState<'_, '_>,
    handle: ManagedReference,
    field: FieldAccess,
    index: u32,
    field_count: u32,
    value: Value,
) -> Result<(), Error> {
    let bounds_checks = state.bounds_checks;
    let null_checks = state.null_checks;

    // track pointer stores
    if state.collect_stats {
        stat_inc!(state.engine.statistics, stores);
    }

    // require one live managed allocation
    let heap = state.heap();
    if !heap.is_managed_allocated(handle) {
        return Err(Error::InvalidManagedReference);
    }

    // validate the requested field before encoding byte storage
    check_field_index(state, index, field_count)?;

    // reject null handles before writing the allocation bytes
    if null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // encode the field payload into the managed byte storage
    let bytes = encode_storage_value(state, field.value_type, value)?;
    let byte_len = state
        .heap()
        .managed_byte_len(handle)
        .ok_or(Error::InvalidManagedReference)?;
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
    if !state.heap_mut().set_managed_bytes(handle, start, &bytes) {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: byte_len,
        });
    }

    Ok(())
}

/// Load a field from a raw heap allocation.
#[inline(always)]
pub(crate) fn load_field_raw(
    state: &mut StepState<'_, '_>,
    pointer: RawPointer,
    field: FieldAccess,
    index: u32,
    field_count: u32,
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

    let byte_offset =
        pointer
            .byte_offset()
            .checked_add(field.byte_offset)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: field_count as usize,
            })?;
    let pointer = Value::raw_pointer(RawPointer::with_byte_offset(
        pointer.id(),
        byte_offset as u32,
    ));
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
    field_count: u32,
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

    let byte_offset =
        pointer
            .byte_offset()
            .checked_add(field.byte_offset)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: field_count as usize,
            })?;
    let pointer = Value::raw_pointer(RawPointer::with_byte_offset(
        pointer.id(),
        byte_offset as u32,
    ));
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
    field_count: u32,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.engine.statistics, loads);
    }

    // validate field index when known
    check_field_index(state, index, field_count)?;

    // resolve the typed field window
    let owned_window = {
        let frame = state.frame_by_index(pointer.frame_idx)?;
        let allocation = frame
            .stack_allocation(pointer.slot)
            .ok_or(Error::InvalidManagedReference)?;
        let start = pointer.slot_offset.checked_add(field.byte_offset).ok_or(
            Error::InvalidFieldAccess {
                index,
                field_count: allocation.len(),
            },
        )?;
        let end = start
            .checked_add(field.byte_len)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: allocation.len(),
            })?;
        let window = allocation
            .bytes()
            .get(start..end)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: allocation.len(),
            })?;

        if field.is_scalar {
            return decode_raw_value(state.tree(), field.value_type, window);
        }

        window.to_vec()
    };

    materialize_value_from_storage(state, field.value_type, &owned_window)
}

/// Store a field into a stack allocation.
#[inline(always)]
pub(crate) fn store_field_stack(
    state: &mut StepState<'_, '_>,
    pointer: StackPointer,
    field: FieldAccess,
    index: u32,
    field_count: u32,
    value: Value,
) -> Result<(), Error> {
    // track pointer stores
    if state.collect_stats {
        stat_inc!(state.engine.statistics, stores);
    }

    // validate field index when known
    check_field_index(state, index, field_count)?;

    // encode the typed field payload
    let bytes = encode_storage_value(state, field.value_type, value)?;

    // write the field bytes into the stack allocation
    let frame = state.frame_by_index_mut(pointer.frame_idx)?;
    let allocation = frame
        .stack_allocation_mut(pointer.slot)
        .ok_or(Error::InvalidManagedReference)?;
    let start =
        pointer
            .slot_offset
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
    index: u32,
    field_count: u32,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.engine.statistics, loads);
    }

    // validate field index when known
    check_field_index(state, index, field_count)?;

    // load the global value
    let value = state
        .globals
        .get(pointer.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: pointer.id })?;

    // require composite payload
    if value.tag() != ValueTag::ManagedReference {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: 0,
        });
    }

    let handle = managed_reference_from_value(value)?;

    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed allocation
    let cell_len = managed_component_count(state, handle)?;

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

    load_heap_slot(state, handle, slot_index)
}

/// Store a field into a global allocation.
#[inline(always)]
pub(crate) fn store_field_global(
    state: &mut StepState<'_, '_>,
    pointer: GlobalPointer,
    index: u32,
    field_count: u32,
    value: Value,
) -> Result<(), Error> {
    let bounds_checks = state.bounds_checks;
    let null_checks = state.null_checks;

    // track pointer stores
    if state.collect_stats {
        stat_inc!(state.engine.statistics, stores);
    }

    // validate field index when known
    check_field_index(state, index, field_count)?;

    // load the global value
    let current = state
        .globals
        .get(pointer.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: pointer.id })?;

    // require composite payload
    if current.tag() != ValueTag::ManagedReference {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: 0,
        });
    }

    let handle = managed_reference_from_value(current)?;

    // reject null handles when enabled
    if null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    {
        // look up the managed heap allocation
        let allocation_len = managed_component_count(state, handle)?;

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

        store_heap_slot(state, handle, slot_index, value)?;
    }

    state.globals.set(pointer.id, current);
    Ok(())
}

/// Load an element from a managed heap allocation.
#[inline(always)]
pub(crate) fn load_element_managed(
    state: &mut StepState<'_, '_>,
    handle: ManagedReference,
    element: ElementAccess,
    index: u64,
    array_length: u64,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.engine.statistics, loads);
    }

    // require one live managed allocation
    let heap = state.heap();
    if !heap.is_managed_allocated(handle) {
        return Err(Error::InvalidManagedReference);
    }

    // validate the requested element before decoding byte storage
    check_array_index(state, index, array_length)?;

    // reject null handles before reading the allocation bytes
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // resolve the element window inside the managed byte storage
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length,
        })?;
    let owned_window = {
        let bytes = heap
            .managed_bytes(handle)
            .ok_or(Error::InvalidManagedReference)?;
        let start = element_offset;
        let end = start
            .checked_add(element.byte_len)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: bytes.len() as u64,
            })?;
        let window = bytes.get(start..end).ok_or(Error::InvalidArrayAccess {
            index,
            length: bytes.len() as u64,
        })?;

        if element.is_scalar {
            return decode_raw_value(state.tree(), element.value_type, window);
        }

        window.to_vec()
    };

    // decode the typed element payload
    materialize_value_from_storage(state, element.value_type, &owned_window)
}

/// Store an element into a managed heap allocation.
#[inline(always)]
pub(crate) fn store_element_managed(
    state: &mut StepState<'_, '_>,
    handle: ManagedReference,
    element: ElementAccess,
    index: u64,
    array_length: u64,
    value: Value,
) -> Result<(), Error> {
    let bounds_checks = state.bounds_checks;
    let null_checks = state.null_checks;

    // track pointer stores
    if state.collect_stats {
        stat_inc!(state.engine.statistics, stores);
    }

    // require one live managed allocation
    let heap = state.heap();
    if !heap.is_managed_allocated(handle) {
        return Err(Error::InvalidManagedReference);
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
            length: array_length,
        })?;
    let bytes = encode_storage_value(state, element.value_type, value)?;
    let allocation_len = state
        .heap()
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

    // write the encoded element bytes
    if !state.heap_mut().set_managed_bytes(handle, start, &bytes) {
        return Err(Error::InvalidArrayAccess {
            index,
            length: allocation_len as u64,
        });
    }

    Ok(())
}

/// Load an element from a raw heap allocation.
#[inline(always)]
pub(crate) fn load_element_raw(
    state: &mut StepState<'_, '_>,
    pointer: RawPointer,
    element: ElementAccess,
    index: u64,
    array_length: u64,
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
    array_length: u64,
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
    array_length: u64,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.engine.statistics, loads);
    }

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // resolve the typed element window
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length,
        })?;
    let owned_window =
        {
            let frame = state.frame_by_index(pointer.frame_idx)?;
            let allocation = frame
                .stack_allocation(pointer.slot)
                .ok_or(Error::InvalidManagedReference)?;
            let start = pointer.slot_offset.checked_add(element_offset).ok_or(
                Error::InvalidArrayAccess {
                    index,
                    length: allocation.len() as u64,
                },
            )?;
            let end = start
                .checked_add(element.byte_len)
                .ok_or(Error::InvalidArrayAccess {
                    index,
                    length: allocation.len() as u64,
                })?;
            let window = allocation
                .bytes()
                .get(start..end)
                .ok_or(Error::InvalidArrayAccess {
                    index,
                    length: allocation.len() as u64,
                })?;

            if element.is_scalar {
                return decode_raw_value(state.tree(), element.value_type, window);
            }

            window.to_vec()
        };

    materialize_value_from_storage(state, element.value_type, &owned_window)
}

/// Store an element into a stack allocation.
#[inline(always)]
pub(crate) fn store_element_stack(
    state: &mut StepState<'_, '_>,
    pointer: StackPointer,
    element: ElementAccess,
    index: u64,
    array_length: u64,
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
            length: array_length,
        })?;

    // encode the typed element payload
    let bytes = encode_storage_value(state, element.value_type, value)?;

    // write the element bytes into the stack allocation
    let frame = state.frame_by_index_mut(pointer.frame_idx)?;
    let allocation = frame
        .stack_allocation_mut(pointer.slot)
        .ok_or(Error::InvalidManagedReference)?;
    let start =
        pointer
            .slot_offset
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
    index: u64,
    array_length: u64,
) -> Result<Value, Error> {
    // track pointer loads
    if state.collect_stats {
        stat_inc!(state.engine.statistics, loads);
    }

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // load the global value
    let value = state
        .globals
        .get(pointer.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: pointer.id })?;

    // require composite payload
    if value.tag() != ValueTag::ManagedReference {
        return Err(Error::InvalidArrayAccess { index, length: 0 });
    }

    let handle = managed_reference_from_value(value)?;

    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let slot_index = resolve_global_element_slot(state, pointer, index, array_length)?;
    load_heap_slot(state, handle, slot_index)
}

/// Store an element into a global allocation.
#[inline(always)]
pub(crate) fn store_element_global(
    state: &mut StepState<'_, '_>,
    pointer: GlobalPointer,
    index: u64,
    array_length: u64,
    value: Value,
) -> Result<(), Error> {
    let null_checks = state.null_checks;

    // track pointer stores
    if state.collect_stats {
        stat_inc!(state.engine.statistics, stores);
    }

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // load the global value
    let current = state
        .globals
        .get(pointer.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: pointer.id })?;

    // require composite payload
    if current.tag() != ValueTag::ManagedReference {
        return Err(Error::InvalidArrayAccess { index, length: 0 });
    }

    let handle = managed_reference_from_value(current)?;

    // reject null handles when enabled
    if null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let slot_index = resolve_global_element_slot(state, pointer, index, array_length)?;
    store_heap_slot(state, handle, slot_index, value)?;

    state.globals.set(pointer.id, current);
    Ok(())
}

/// Get a field from an composite value.
#[inline(always)]
pub(crate) fn get_field(
    state: &mut StepState<'_, '_>,
    agg: Value,
    index: u32,
) -> Result<Value, Error> {
    // resolve composite value
    match agg.tag() {
        ValueTag::ManagedReference => {
            let handle = managed_reference_from_value(agg)?;
            get_heap_field(state, handle, index)
        }
        ValueTag::StackPointer => {
            let pointer = stack_pointer_from_value(agg)?;
            get_stack_field(state, pointer, index)
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
) -> Result<Value, Error> {
    // copy the value into fresh stack storage first
    let copied = duplicate_composite_value_to_stack(state, agg, "composite")?;
    let pointer = stack_pointer_from_value(copied)?;

    // then mutate the fresh value
    set_stack_field(state, pointer, index, val)?;

    Ok(copied)
}

/// Store one field into one addressable composite place.
#[inline(always)]
pub(crate) fn set_field_in_place(
    state: &mut StepState<'_, '_>,
    agg: Value,
    index: u32,
    val: Value,
) -> Result<(), Error> {
    match agg.tag() {
        ValueTag::ManagedReference => {
            let handle = managed_reference_from_value(agg)?;
            store_heap_slot(state, handle, index as usize, val)
        }
        ValueTag::StackPointer => {
            let pointer = stack_pointer_from_value(agg)?;
            set_stack_field(state, pointer, index, val)
        }
        _ => Err(Error::TypeMismatch {
            expected: "composite place".to_string(),
            actual: format!("{agg:?}"),
        }),
    }
}

/// Get an element from an array value.
#[inline(always)]
pub(crate) fn get_element(
    state: &mut StepState<'_, '_>,
    arr: Value,
    index: u64,
) -> Result<Value, Error> {
    // resolve array value
    match arr.tag() {
        ValueTag::ManagedReference => {
            let handle = managed_reference_from_value(arr)?;
            get_heap_element(state, handle, index)
        }
        ValueTag::StackPointer => {
            let pointer = stack_pointer_from_value(arr)?;
            get_stack_element(state, pointer, index)
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
) -> Result<Value, Error> {
    // copy the value into fresh stack storage first
    let copied = duplicate_composite_value_to_stack(state, arr, "array")?;
    let pointer = stack_pointer_from_value(copied)?;

    // then mutate the fresh value
    set_stack_element(state, pointer, index, val)?;

    Ok(copied)
}

/// Duplicate one whole composite or array value into fresh stack storage.
fn duplicate_composite_value_to_stack(
    state: &mut StepState<'_, '_>,
    value: Value,
    expected: &'static str,
) -> Result<Value, Error> {
    let (storage_type, bytes) = match value.tag() {
        ValueTag::ManagedReference => {
            let handle = managed_reference_from_value(value)?;
            let storage_type = managed_storage_type(state, handle)?;
            let bytes = state
                .heap()
                .managed_bytes(handle)
                .ok_or(Error::InvalidManagedReference)?
                .into_owned();

            (storage_type, bytes)
        }
        ValueTag::StackPointer => {
            let pointer = stack_pointer_from_value(value)?;
            if pointer.slot_offset != 0 {
                return Err(Error::InvalidManagedReference);
            }

            let (storage_type, bytes) = {
                let frame = state.frame_by_index(pointer.frame_idx)?;
                let allocation = frame
                    .stack_allocation(pointer.slot)
                    .ok_or(Error::InvalidManagedReference)?;

                (allocation.storage_type(), allocation.clone_bytes())
            };

            (storage_type, bytes)
        }
        _ => {
            return Err(Error::TypeMismatch {
                expected: expected.to_string(),
                actual: format!("{value:?}"),
            });
        }
    };

    // keep updated value semantics non-aliased
    let allocation = StackAllocation::from_bytes(bytes, storage_type);
    let frame_index = state.frame_index;
    let slot = state
        .current_frame_mut()
        .allocate_stack_allocation(allocation);

    Ok(Value::stack_pointer(StackPointer::new(frame_index, slot)))
}

/// Load a slot from a managed heap allocation.
#[inline(always)]
fn load_heap_slot(
    state: &mut StepState<'_, '_>,
    handle: ManagedReference,
    slot_index: usize,
) -> Result<Value, Error> {
    let null_checks = state.null_checks;

    // reject null handles when enabled
    if null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // decode the typed component payload
    let heap = state.heap();
    let aggregate_type = managed_storage_type(state, handle)?;
    let component = state.storage_component_layout(
        aggregate_type,
        u32::try_from(slot_index).map_err(|_| Error::InvalidFieldAccess {
            index: slot_index as u32,
            field_count: 0,
        })?,
    )?;
    let owned_window = {
        let bytes = heap
            .managed_bytes(handle)
            .ok_or(Error::InvalidManagedReference)?;
        let start = component.offset;
        let end = start
            .checked_add(component.byte_len)
            .ok_or(Error::InvalidFieldAccess {
                index: slot_index as u32,
                field_count: bytes.len(),
            })?;
        let window = bytes.get(start..end).ok_or(Error::InvalidFieldAccess {
            index: slot_index as u32,
            field_count: bytes.len(),
        })?;

        if state.layout(component.ty)?.is_scalar() {
            return decode_raw_value(state.tree(), component.ty, window);
        }

        window.to_vec()
    };

    materialize_value_from_storage(state, component.ty, &owned_window)
}

/// Store a slot into a managed heap allocation.
#[inline(always)]
fn store_heap_slot(
    state: &mut StepState<'_, '_>,
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

    // encode the typed component payload
    let aggregate_type = managed_storage_type(state, handle)?;
    let component = state.storage_component_layout(
        aggregate_type,
        u32::try_from(slot_index).map_err(|_| Error::InvalidFieldAccess {
            index: slot_index as u32,
            field_count: 0,
        })?,
    )?;
    let bytes = encode_storage_value(state, component.ty, value)?;
    let allocation_len = state
        .heap()
        .managed_byte_len(handle)
        .ok_or(Error::InvalidManagedReference)?;
    let start = component.offset;
    let end = start
        .checked_add(bytes.len())
        .ok_or(Error::InvalidFieldAccess {
            index: slot_index as u32,
            field_count: allocation_len,
        })?;

    if bounds_checks && end > allocation_len {
        return Err(Error::InvalidFieldAccess {
            index: slot_index as u32,
            field_count: allocation_len,
        });
    }

    // write the encoded component bytes
    if !state.heap_mut().set_managed_bytes(handle, start, &bytes) {
        return Err(Error::InvalidFieldAccess {
            index: slot_index as u32,
            field_count: allocation_len,
        });
    }

    Ok(())
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
    if global.slot_offset == 0 {
        return Ok(value);
    }

    // read from the composite stored in the global
    if value.tag() != ValueTag::ManagedReference {
        return Err(Error::InvalidFieldAccess {
            index: global.slot_offset as u32,
            field_count: 0,
        });
    }

    let handle = managed_reference_from_value(value)?;
    load_heap_slot(state, handle, global.slot_offset)
}

/// Store through a global pointer, including slot offsets.
#[inline(always)]
fn store_global_slot(
    state: &mut StepState<'_, '_>,
    global: GlobalPointer,
    value: Value,
) -> Result<(), Error> {
    // update the global value directly
    if global.slot_offset == 0 {
        state.globals.set(global.id, value);
        return Ok(());
    }

    // update a slot on the composite stored in the global
    let current = state
        .globals
        .get(global.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: global.id })?;
    if current.tag() != ValueTag::ManagedReference {
        return Err(Error::InvalidFieldAccess {
            index: global.slot_offset as u32,
            field_count: 0,
        });
    }

    let handle = managed_reference_from_value(current)?;
    store_heap_slot(state, handle, global.slot_offset, value)?;
    state.globals.set(global.id, current);
    Ok(())
}

/// Get a field from a heap composite.
#[inline(always)]
fn get_heap_field(
    state: &mut StepState<'_, '_>,
    handle: ManagedReference,
    index: u32,
) -> Result<Value, Error> {
    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // decode the typed field payload
    let heap = state.heap();
    let aggregate_type = managed_storage_type(state, handle)?;
    let field = state.storage_component_layout(aggregate_type, index)?;
    let owned_window = {
        let bytes = heap
            .managed_bytes(handle)
            .ok_or(Error::InvalidManagedReference)?;
        let start = field.offset;
        let end = start
            .checked_add(field.byte_len)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: bytes.len(),
            })?;
        let window = bytes.get(start..end).ok_or(Error::InvalidFieldAccess {
            index,
            field_count: bytes.len(),
        })?;

        if state.layout(field.ty)?.is_scalar() {
            return decode_raw_value(state.tree(), field.ty, window);
        }

        window.to_vec()
    };

    materialize_value_from_storage(state, field.ty, &owned_window)
}

/// Get a field from a stack-backed composite.
#[inline(always)]
fn get_stack_field(
    state: &mut StepState<'_, '_>,
    pointer: StackPointer,
    index: u32,
) -> Result<Value, Error> {
    let field = {
        let frame = state.frame_by_index(pointer.frame_idx)?;
        let allocation = frame
            .stack_allocation(pointer.slot)
            .ok_or(Error::InvalidManagedReference)?;
        let composite_type = allocation.storage_type();

        state.storage_component_layout(composite_type, index)?
    };
    let field_count = {
        let frame = state.frame_by_index(pointer.frame_idx)?;
        let allocation = frame
            .stack_allocation(pointer.slot)
            .ok_or(Error::InvalidManagedReference)?;
        let composite_type = allocation.storage_type();

        state.storage_component_count(composite_type)? as u32
    };
    let access = FieldAccess {
        value_type: field.ty,
        byte_offset: field.offset,
        byte_len: field.byte_len,
        is_scalar: state.layout(field.ty)?.is_scalar(),
    };

    load_field_stack(state, pointer, access, index, field_count)
}

/// Set a field on a stack-backed composite.
#[inline(always)]
fn set_stack_field(
    state: &mut StepState<'_, '_>,
    pointer: StackPointer,
    index: u32,
    value: Value,
) -> Result<(), Error> {
    let field = {
        let frame = state.frame_by_index(pointer.frame_idx)?;
        let allocation = frame
            .stack_allocation(pointer.slot)
            .ok_or(Error::InvalidManagedReference)?;
        let composite_type = allocation.storage_type();

        state.storage_component_layout(composite_type, index)?
    };
    let field_count = {
        let frame = state.frame_by_index(pointer.frame_idx)?;
        let allocation = frame
            .stack_allocation(pointer.slot)
            .ok_or(Error::InvalidManagedReference)?;
        let composite_type = allocation.storage_type();

        state.storage_component_count(composite_type)? as u32
    };
    let access = FieldAccess {
        value_type: field.ty,
        byte_offset: field.offset,
        byte_len: field.byte_len,
        is_scalar: state.layout(field.ty)?.is_scalar(),
    };

    store_field_stack(state, pointer, access, index, field_count, value)
}

/// Get an element from a heap array composite.
#[inline(always)]
fn get_heap_element(
    state: &mut StepState<'_, '_>,
    handle: ManagedReference,
    index: u64,
) -> Result<Value, Error> {
    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // decode the typed element payload
    let heap = state.heap();
    let aggregate_type = managed_storage_type(state, handle)?;
    let layout = state.layout(aggregate_type)?;
    let element = layout.element().ok_or(Error::TypeMismatch {
        expected: "array type".to_string(),
        actual: format!("{aggregate_type:?}"),
    })?;
    let element_count = layout.component_count().ok_or(Error::TypeMismatch {
        expected: "array storage length".to_string(),
        actual: format!("{aggregate_type:?}"),
    })?;
    let index_usize = usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
        index,
        length: element_count as u64,
    })?;
    if state.bounds_checks && index_usize >= element_count {
        return Err(Error::InvalidArrayAccess {
            index,
            length: element_count as u64,
        });
    }
    let start = index_usize
        .checked_mul(element.stride)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: element_count as u64,
        })?;
    let owned_window = {
        let bytes = heap
            .managed_bytes(handle)
            .ok_or(Error::InvalidManagedReference)?;
        let end = start
            .checked_add(element.byte_len)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: bytes.len() as u64,
            })?;
        let window = bytes.get(start..end).ok_or(Error::InvalidArrayAccess {
            index,
            length: bytes.len() as u64,
        })?;

        if state.layout(element.ty)?.is_scalar() {
            return decode_raw_value(state.tree(), element.ty, window);
        }

        window.to_vec()
    };

    materialize_value_from_storage(state, element.ty, &owned_window)
}

/// Get an element from a stack-backed composite.
#[inline(always)]
fn get_stack_element(
    state: &mut StepState<'_, '_>,
    pointer: StackPointer,
    index: u64,
) -> Result<Value, Error> {
    let (element, element_count) = {
        let frame = state.frame_by_index(pointer.frame_idx)?;
        let allocation = frame
            .stack_allocation(pointer.slot)
            .ok_or(Error::InvalidManagedReference)?;
        let composite_type = allocation.storage_type();
        let layout = state.layout(composite_type)?;
        let element = layout.element().ok_or(Error::TypeMismatch {
            expected: "array type".to_string(),
            actual: format!("{composite_type:?}"),
        })?;
        let element_count = layout.component_count().ok_or(Error::TypeMismatch {
            expected: "array storage length".to_string(),
            actual: format!("{composite_type:?}"),
        })?;

        (element, element_count)
    };
    let access = ElementAccess {
        value_type: element.ty,
        byte_stride: element.stride,
        byte_len: element.byte_len,
        is_scalar: state.layout(element.ty)?.is_scalar(),
    };

    load_element_stack(state, pointer, access, index, element_count as u64)
}

/// Set an element on a stack-backed composite.
#[inline(always)]
fn set_stack_element(
    state: &mut StepState<'_, '_>,
    pointer: StackPointer,
    index: u64,
    value: Value,
) -> Result<(), Error> {
    let (element, element_count) = {
        let frame = state.frame_by_index(pointer.frame_idx)?;
        let allocation = frame
            .stack_allocation(pointer.slot)
            .ok_or(Error::InvalidManagedReference)?;
        let composite_type = allocation.storage_type();
        let layout = state.layout(composite_type)?;
        let element = layout.element().ok_or(Error::TypeMismatch {
            expected: "array type".to_string(),
            actual: format!("{composite_type:?}"),
        })?;
        let element_count = layout.component_count().ok_or(Error::TypeMismatch {
            expected: "array storage length".to_string(),
            actual: format!("{composite_type:?}"),
        })?;

        (element, element_count)
    };
    let access = ElementAccess {
        value_type: element.ty,
        byte_stride: element.stride,
        byte_len: element.byte_len,
        is_scalar: state.layout(element.ty)?.is_scalar(),
    };

    store_element_stack(state, pointer, access, index, element_count as u64, value)
}

/// Resolve a field slot for a global pointer.
#[inline(always)]
fn resolve_global_field_slot(
    state: &mut StepState<'_, '_>,
    global: GlobalPointer,
    index: u32,
    field_count: u32,
) -> Result<usize, Error> {
    // load the global value
    let value = state
        .globals
        .get(global.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: global.id })?;

    // require composite payload
    if value.tag() != ValueTag::ManagedReference {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: 0,
        });
    }

    let handle = managed_reference_from_value(value)?;

    // look up the managed allocation
    let cell_len = managed_component_count(state, handle)?;

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
    state: &mut StepState<'_, '_>,
    handle: ManagedReference,
    index: u64,
    array_length: u64,
) -> Result<u32, Error> {
    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed allocation
    let cell_len = managed_component_count(state, handle)?;

    // compute the absolute slot offset
    let index_usize = if state.bounds_checks {
        usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell_len as u64,
        })?
    } else {
        index as usize
    };
    let slot_index =
        handle
            .slot_offset()
            .checked_add(index_usize)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: cell_len as u64,
            })?;

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

/// Resolve an element slot for a global pointer.
#[inline(always)]
fn resolve_global_element_slot(
    state: &mut StepState<'_, '_>,
    global: GlobalPointer,
    index: u64,
    array_length: u64,
) -> Result<usize, Error> {
    // load the global value
    let value = state
        .globals
        .get(global.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: global.id })?;

    // require composite payload
    if value.tag() != ValueTag::ManagedReference {
        return Err(Error::InvalidArrayAccess { index, length: 0 });
    }

    let handle = managed_reference_from_value(value)?;

    // look up the managed allocation
    let cell_len = managed_component_count(state, handle)?;

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

/// Load a local slot from a frame.
#[inline(always)]
fn load_local_slot(
    state: &mut StepState<'_, '_>,
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
    frame.get_local_or_error(&state.engine.local_stack, local)
}

/// Store a local slot into a frame.
#[inline(always)]
fn store_local_slot(
    state: &mut StepState<'_, '_>,
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
    let locals = &mut state.engine.local_stack;
    let Some(target) = locals.get_mut(slot) else {
        return Err(Error::UndefinedLocal { local });
    };
    *target = value;

    Ok(())
}
