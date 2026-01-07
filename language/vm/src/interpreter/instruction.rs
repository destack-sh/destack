use crate::diagnostic::Error;
use crate::memory::{GlobalPointer, HeapHandle, RawPointer, StackPointer, Value, ValueTag};

use super::threaded::{ThreadedState, UNKNOWN_ARRAY_LENGTH, UNKNOWN_FIELD_COUNT};

/// Load a value from a pointer.
#[inline(always)]
pub(super) fn load_from_pointer(state: &mut ThreadedState<'_>, ptr: Value) -> Result<Value, Error> {
    // track pointer loads
    state.interpreter.statistics.loads += 1;

    // resolve pointer kind and load
    match ptr.tag() {
        ValueTag::ManagedReference => {
            let handle = ptr.as_heap_handle().unwrap();
            load_heap_slot(state, handle, handle.slot_index())
        }
        ValueTag::RawPointer => {
            let raw_ptr = ptr.as_raw_pointer().unwrap();
            load_raw_slot(state, raw_ptr, raw_ptr.slot_index())
        }
        ValueTag::StackPointer => {
            let sp = ptr.as_stack_pointer().unwrap();
            load_stack_slot(state, sp, sp.slot_offset)
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

/// Store a value to a pointer.
#[inline(always)]
pub(super) fn store_to_pointer(
    state: &mut ThreadedState<'_>,
    ptr: Value,
    val: Value,
) -> Result<(), Error> {
    // track pointer stores
    state.interpreter.statistics.stores += 1;

    // resolve pointer kind and store
    match ptr.tag() {
        ValueTag::ManagedReference => {
            let handle = ptr.as_heap_handle().unwrap();
            store_heap_slot(state, handle, handle.slot_index(), val)
        }
        ValueTag::RawPointer => {
            let raw_ptr = ptr.as_raw_pointer().unwrap();
            store_raw_slot(state, raw_ptr, raw_ptr.slot_index(), val)
        }
        ValueTag::StackPointer => {
            let sp = ptr.as_stack_pointer().unwrap();
            store_stack_slot(state, sp, sp.slot_offset, val)
        }
        ValueTag::GlobalPointer => {
            let global = ptr.as_global_pointer().unwrap();
            let global_def = state.interpreter.tree.get(global.id);
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
pub(super) fn load_from_managed_reference(
    state: &mut ThreadedState<'_>,
    ptr: Value,
) -> Result<Value, Error> {
    // validate pointer tag
    if ptr.tag() != ValueTag::ManagedReference {
        return Err(Error::InvalidPointerType {
            actual: format!("{ptr:?}"),
        });
    }

    // resolve handle
    let handle = ptr.as_heap_handle().unwrap();
    load_heap_slot(state, handle, handle.slot_index())
}

/// Load a value from a raw pointer.
#[inline(always)]
pub(super) fn load_from_raw_pointer(
    state: &mut ThreadedState<'_>,
    ptr: Value,
) -> Result<Value, Error> {
    // validate pointer tag
    if ptr.tag() != ValueTag::RawPointer {
        return Err(Error::InvalidPointerType {
            actual: format!("{ptr:?}"),
        });
    }

    // resolve pointer
    let raw_ptr = ptr.as_raw_pointer().unwrap();
    load_raw_slot(state, raw_ptr, raw_ptr.slot_index())
}

/// Load a value from a stack pointer.
#[inline(always)]
pub(super) fn load_from_stack_pointer(
    state: &mut ThreadedState<'_>,
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

/// Load a value from a global pointer.
#[inline(always)]
pub(super) fn load_from_global_pointer(
    state: &mut ThreadedState<'_>,
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

/// Store a value through a managed reference.
#[inline(always)]
pub(super) fn store_to_managed_reference(
    state: &mut ThreadedState<'_>,
    ptr: Value,
    val: Value,
) -> Result<(), Error> {
    // validate pointer tag
    if ptr.tag() != ValueTag::ManagedReference {
        return Err(Error::InvalidPointerType {
            actual: format!("{ptr:?}"),
        });
    }

    // resolve handle
    let handle = ptr.as_heap_handle().unwrap();
    store_heap_slot(state, handle, handle.slot_index(), val)
}

/// Store a value through a raw pointer.
#[inline(always)]
pub(super) fn store_to_raw_pointer(
    state: &mut ThreadedState<'_>,
    ptr: Value,
    val: Value,
) -> Result<(), Error> {
    // validate pointer tag
    if ptr.tag() != ValueTag::RawPointer {
        return Err(Error::InvalidPointerType {
            actual: format!("{ptr:?}"),
        });
    }

    // resolve pointer
    let raw_ptr = ptr.as_raw_pointer().unwrap();
    store_raw_slot(state, raw_ptr, raw_ptr.slot_index(), val)
}

/// Store a value through a stack pointer.
#[inline(always)]
pub(super) fn store_to_stack_pointer(
    state: &mut ThreadedState<'_>,
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

/// Store a value through a global pointer.
#[inline(always)]
pub(super) fn store_to_global_pointer(
    state: &mut ThreadedState<'_>,
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
    let global_def = state.interpreter.tree.get(global.id);
    if !global_def.is_mutable() {
        return Err(Error::ImmutableGlobalWrite { global: global.id });
    }
    store_global_slot(state, global, val)
}

/// Validate a field index against a known field count.
#[inline(always)]
fn check_field_index(state: &ThreadedState<'_>, index: u32, field_count: u32) -> Result<(), Error> {
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
    state: &ThreadedState<'_>,
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

/// Get the address of a field from an aggregate or pointer.
#[inline(always)]
pub(super) fn field_addr(
    state: &mut ThreadedState<'_>,
    aggregate: Value,
    index: u32,
    field_count: u32,
) -> Result<Value, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    // resolve the source and compute the field pointer
    match aggregate.tag() {
        ValueTag::Aggregate | ValueTag::ManagedReference => {
            let handle = aggregate.as_heap_handle().unwrap();
            let slot_index = resolve_heap_field_slot(state, handle, index, field_count)?;
            let handle = HeapHandle::with_slot(handle.id(), slot_index);
            Ok(Value::managed_reference(handle))
        }
        ValueTag::RawPointer => {
            let raw_ptr = aggregate.as_raw_pointer().unwrap();
            let slot_index = resolve_raw_field_slot(state, raw_ptr, index, field_count)?;
            Ok(Value::raw_pointer(RawPointer::with_slot(
                raw_ptr.id(),
                slot_index,
            )))
        }
        ValueTag::StackPointer => {
            let sp = aggregate.as_stack_pointer().unwrap();
            let slot_index = resolve_stack_field_slot(state, sp, index, field_count)?;
            Ok(Value::stack_pointer(StackPointer::with_offset(
                sp.frame_idx,
                sp.slot,
                slot_index,
            )))
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
pub(super) fn field_addr_managed(
    state: &mut ThreadedState<'_>,
    handle: HeapHandle,
    index: u32,
    field_count: u32,
) -> Result<Value, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    // resolve slot offset
    let slot_index = resolve_heap_field_slot(state, handle, index, field_count)?;
    Ok(Value::managed_reference(HeapHandle::with_slot(
        handle.id(),
        slot_index,
    )))
}

/// Get the address of a field from a raw pointer.
#[inline(always)]
pub(super) fn field_addr_raw(
    state: &mut ThreadedState<'_>,
    pointer: RawPointer,
    index: u32,
    field_count: u32,
) -> Result<Value, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    // resolve slot offset
    let slot_index = resolve_raw_field_slot(state, pointer, index, field_count)?;
    Ok(Value::raw_pointer(RawPointer::with_slot(
        pointer.id(),
        slot_index,
    )))
}

/// Get the address of a field from a stack pointer.
#[inline(always)]
pub(super) fn field_addr_stack(
    state: &mut ThreadedState<'_>,
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

/// Get the address of a field from a global pointer.
#[inline(always)]
pub(super) fn field_addr_global(
    state: &mut ThreadedState<'_>,
    pointer: GlobalPointer,
    index: u32,
    field_count: u32,
) -> Result<Value, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    // resolve slot offset
    let slot_index = resolve_global_field_slot(state, pointer, index, field_count)?;
    Ok(Value::global_pointer_with_offset(pointer.id, slot_index))
}

/// Get the address of an element from an array or pointer.
#[inline(always)]
pub(super) fn element_addr(
    state: &mut ThreadedState<'_>,
    array: Value,
    index: u64,
    array_length: u64,
) -> Result<Value, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    // resolve the source and compute the element pointer
    match array.tag() {
        ValueTag::Aggregate | ValueTag::ManagedReference => {
            let handle = array.as_heap_handle().unwrap();
            let slot_index = resolve_heap_element_slot(state, handle, index, array_length)?;
            let handle = HeapHandle::with_slot(handle.id(), slot_index);
            Ok(Value::managed_reference(handle))
        }
        ValueTag::RawPointer => {
            let raw_ptr = array.as_raw_pointer().unwrap();
            let slot_index = resolve_raw_element_slot(state, raw_ptr, index, array_length)?;
            Ok(Value::raw_pointer(RawPointer::with_slot(
                raw_ptr.id(),
                slot_index,
            )))
        }
        ValueTag::StackPointer => {
            let sp = array.as_stack_pointer().unwrap();
            let slot_index = resolve_stack_element_slot(state, sp, index, array_length)?;
            Ok(Value::stack_pointer(StackPointer::with_offset(
                sp.frame_idx,
                sp.slot,
                slot_index,
            )))
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
pub(super) fn element_addr_managed(
    state: &mut ThreadedState<'_>,
    handle: HeapHandle,
    index: u64,
    array_length: u64,
) -> Result<Value, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    // resolve slot offset
    let slot_index = resolve_heap_element_slot(state, handle, index, array_length)?;
    Ok(Value::managed_reference(HeapHandle::with_slot(
        handle.id(),
        slot_index,
    )))
}

/// Get the address of an element from a raw pointer.
#[inline(always)]
pub(super) fn element_addr_raw(
    state: &mut ThreadedState<'_>,
    pointer: RawPointer,
    index: u64,
    array_length: u64,
) -> Result<Value, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    // resolve slot offset
    let slot_index = resolve_raw_element_slot(state, pointer, index, array_length)?;
    Ok(Value::raw_pointer(RawPointer::with_slot(
        pointer.id(),
        slot_index,
    )))
}

/// Get the address of an element from a stack pointer.
#[inline(always)]
pub(super) fn element_addr_stack(
    state: &mut ThreadedState<'_>,
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

/// Get the address of an element from a global pointer.
#[inline(always)]
pub(super) fn element_addr_global(
    state: &mut ThreadedState<'_>,
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
pub(super) fn load_field_managed(
    state: &mut ThreadedState<'_>,
    handle: HeapHandle,
    index: u32,
    field_count: u32,
) -> Result<Value, Error> {
    // track pointer loads
    state.interpreter.statistics.loads += 1;

    // fast path: skip all validation when bounds and null checks are disabled
    if !state.bounds_checks && !state.null_checks {
        // compute slot index
        let slot_index = handle.slot_index().wrapping_add(index as usize);

        // #Safety: checks are disabled, caller ensures validity
        let cell = unsafe { state.interpreter.managed_heap.get_unchecked(handle) };
        debug_assert!(slot_index < cell.slots.len(), "heap field out of bounds");
        let value = unsafe { *cell.slots.get_unchecked(slot_index) };
        return Ok(value);
    }

    // validate field index when known
    check_field_index(state, index, field_count)?;

    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed heap cell
    let cell = state
        .interpreter
        .managed_heap
        .get(handle)
        .ok_or(Error::InvalidHeapHandle)?;

    // select field count for diagnostics
    let field_count_for_error = if field_count == UNKNOWN_FIELD_COUNT {
        cell.slots.len()
    } else {
        field_count as usize
    };

    // compute the absolute slot offset
    let slot_index = if state.bounds_checks {
        handle
            .slot_index()
            .checked_add(index as usize)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: field_count_for_error,
            })?
    } else {
        handle.slot_index().wrapping_add(index as usize)
    };

    // validate bounds when field count is unknown
    if state.bounds_checks
        && field_count == UNKNOWN_FIELD_COUNT
        && !cell.slots.is_empty()
        && slot_index >= cell.slots.len()
    {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: cell.slots.len(),
        });
    }

    // treat empty slot 0 as void
    if cell.slots.is_empty() && slot_index == 0 {
        return Ok(Value::VOID);
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "heap field out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        let value = unsafe { *cell.slots.get_unchecked(slot_index) };
        return Ok(value);
    }

    // read the slot when in bounds
    if let Some(value) = cell.slots.get(slot_index).copied() {
        return Ok(value);
    }

    Err(Error::InvalidFieldAccess {
        index,
        field_count: cell.slots.len(),
    })
}

/// Store a field into a managed heap allocation.
#[inline(always)]
pub(super) fn store_field_managed(
    state: &mut ThreadedState<'_>,
    handle: HeapHandle,
    index: u32,
    field_count: u32,
    value: Value,
) -> Result<(), Error> {
    // track pointer stores
    state.interpreter.statistics.stores += 1;

    // fast path: skip all validation when bounds and null checks are disabled
    if !state.bounds_checks && !state.null_checks {
        // compute slot index
        let slot_index = handle.slot_index().wrapping_add(index as usize);

        // #Safety: checks are disabled, caller ensures validity
        let cell = unsafe { state.interpreter.managed_heap.get_unchecked_mut(handle) };
        debug_assert!(slot_index < cell.slots.len(), "heap field out of bounds");
        unsafe { *cell.slots.get_unchecked_mut(slot_index) = value };
        return Ok(());
    }

    // validate field index when known
    check_field_index(state, index, field_count)?;

    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed heap cell
    let cell = state
        .interpreter
        .managed_heap
        .get_mut(handle)
        .ok_or(Error::InvalidHeapHandle)?;

    // select field count for diagnostics
    let field_count_for_error = if field_count == UNKNOWN_FIELD_COUNT {
        cell.slots.len()
    } else {
        field_count as usize
    };

    // compute the absolute slot offset
    let slot_index = if state.bounds_checks {
        handle
            .slot_index()
            .checked_add(index as usize)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: field_count_for_error,
            })?
    } else {
        handle.slot_index().wrapping_add(index as usize)
    };

    // validate bounds when field count is unknown
    if state.bounds_checks
        && field_count == UNKNOWN_FIELD_COUNT
        && !cell.slots.is_empty()
        && slot_index >= cell.slots.len()
    {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: cell.slots.len(),
        });
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "heap field out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        unsafe {
            *cell.slots.get_unchecked_mut(slot_index) = value;
        }
        return Ok(());
    }

    // write the slot when in bounds
    if let Some(slot) = cell.slots.get_mut(slot_index) {
        *slot = value;
        return Ok(());
    }

    Err(Error::InvalidFieldAccess {
        index,
        field_count: cell.slots.len(),
    })
}

/// Load a field from a raw heap allocation.
#[inline(always)]
pub(super) fn load_field_raw(
    state: &mut ThreadedState<'_>,
    pointer: RawPointer,
    index: u32,
    field_count: u32,
) -> Result<Value, Error> {
    // track pointer loads
    state.interpreter.statistics.loads += 1;

    // validate field index when known
    check_field_index(state, index, field_count)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the raw heap cell
    let cell = state
        .interpreter
        .raw_heap
        .get(pointer)
        .ok_or(Error::InvalidHeapHandle)?;

    // select field count for diagnostics
    let field_count_for_error = if field_count == UNKNOWN_FIELD_COUNT {
        cell.slots.len()
    } else {
        field_count as usize
    };

    // compute the absolute slot offset
    let slot_index = if state.bounds_checks {
        pointer
            .slot_index()
            .checked_add(index as usize)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: field_count_for_error,
            })?
    } else {
        pointer.slot_index().wrapping_add(index as usize)
    };

    // validate bounds when field count is unknown
    if state.bounds_checks
        && field_count == UNKNOWN_FIELD_COUNT
        && !cell.slots.is_empty()
        && slot_index >= cell.slots.len()
    {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: cell.slots.len(),
        });
    }

    // treat empty slot 0 as void
    if cell.slots.is_empty() && slot_index == 0 {
        return Ok(Value::VOID);
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "raw field out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        let value = unsafe { *cell.slots.get_unchecked(slot_index) };
        return Ok(value);
    }

    // read the slot when in bounds
    if let Some(value) = cell.slots.get(slot_index).copied() {
        return Ok(value);
    }

    Err(Error::InvalidFieldAccess {
        index,
        field_count: cell.slots.len(),
    })
}

/// Store a field into a raw heap allocation.
#[inline(always)]
pub(super) fn store_field_raw(
    state: &mut ThreadedState<'_>,
    pointer: RawPointer,
    index: u32,
    field_count: u32,
    value: Value,
) -> Result<(), Error> {
    // track pointer stores
    state.interpreter.statistics.stores += 1;

    // validate field index when known
    check_field_index(state, index, field_count)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the raw heap cell
    let cell = state
        .interpreter
        .raw_heap
        .get_mut(pointer)
        .ok_or(Error::InvalidHeapHandle)?;

    // select field count for diagnostics
    let field_count_for_error = if field_count == UNKNOWN_FIELD_COUNT {
        cell.slots.len()
    } else {
        field_count as usize
    };

    // compute the absolute slot offset
    let slot_index = if state.bounds_checks {
        pointer
            .slot_index()
            .checked_add(index as usize)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: field_count_for_error,
            })?
    } else {
        pointer.slot_index().wrapping_add(index as usize)
    };

    // validate bounds when field count is unknown
    if state.bounds_checks
        && field_count == UNKNOWN_FIELD_COUNT
        && !cell.slots.is_empty()
        && slot_index >= cell.slots.len()
    {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: cell.slots.len(),
        });
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "raw field out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        unsafe {
            *cell.slots.get_unchecked_mut(slot_index) = value;
        }
        return Ok(());
    }

    // write the slot when in bounds
    if let Some(slot) = cell.slots.get_mut(slot_index) {
        *slot = value;
        return Ok(());
    }

    Err(Error::InvalidFieldAccess {
        index,
        field_count: cell.slots.len(),
    })
}

/// Load a field from a stack allocation.
#[inline(always)]
pub(super) fn load_field_stack(
    state: &mut ThreadedState<'_>,
    pointer: StackPointer,
    index: u32,
    field_count: u32,
) -> Result<Value, Error> {
    // track pointer loads
    state.interpreter.statistics.loads += 1;

    // validate field index when known
    check_field_index(state, index, field_count)?;

    // look up the stack cell
    let frame = state.frame_by_index(pointer.frame_idx)?;
    let cell = frame
        .get_stack_cell(pointer.slot)
        .ok_or(Error::InvalidHeapHandle)?;

    // select field count for diagnostics
    let field_count_for_error = if field_count == UNKNOWN_FIELD_COUNT {
        cell.slots.len()
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
        && !cell.slots.is_empty()
        && slot_index >= cell.slots.len()
    {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: cell.slots.len(),
        });
    }

    // treat empty slot 0 as void
    if cell.slots.is_empty() && slot_index == 0 {
        return Ok(Value::VOID);
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "stack field out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        let value = unsafe { *cell.slots.get_unchecked(slot_index) };
        return Ok(value);
    }

    // read the slot when in bounds
    if let Some(value) = cell.slots.get(slot_index).copied() {
        return Ok(value);
    }

    Err(Error::InvalidFieldAccess {
        index,
        field_count: cell.slots.len(),
    })
}

/// Store a field into a stack allocation.
#[inline(always)]
pub(super) fn store_field_stack(
    state: &mut ThreadedState<'_>,
    pointer: StackPointer,
    index: u32,
    field_count: u32,
    value: Value,
) -> Result<(), Error> {
    // cache bounds checks setting
    let bounds_checks = state.bounds_checks;

    // track pointer stores
    state.interpreter.statistics.stores += 1;

    // validate field index when known
    check_field_index(state, index, field_count)?;

    // look up the stack cell
    let frame = state.frame_by_index_mut(pointer.frame_idx)?;
    let cell = frame
        .get_stack_cell_mut(pointer.slot)
        .ok_or(Error::InvalidHeapHandle)?;

    // select field count for diagnostics
    let field_count_for_error = if field_count == UNKNOWN_FIELD_COUNT {
        cell.slots.len()
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
        && !cell.slots.is_empty()
        && slot_index >= cell.slots.len()
    {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: cell.slots.len(),
        });
    }

    // fast path without bounds checks
    if !bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "stack field out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        unsafe {
            *cell.slots.get_unchecked_mut(slot_index) = value;
        }
        return Ok(());
    }

    // write the slot when in bounds
    if let Some(slot) = cell.slots.get_mut(slot_index) {
        *slot = value;
        return Ok(());
    }

    Err(Error::InvalidFieldAccess {
        index,
        field_count: cell.slots.len(),
    })
}

/// Load a field from a global allocation.
#[inline(always)]
pub(super) fn load_field_global(
    state: &mut ThreadedState<'_>,
    pointer: GlobalPointer,
    index: u32,
    field_count: u32,
) -> Result<Value, Error> {
    // track pointer loads
    state.interpreter.statistics.loads += 1;

    // validate field index when known
    check_field_index(state, index, field_count)?;

    // load the global value
    let value = state
        .interpreter
        .globals
        .get(pointer.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: pointer.id })?;

    // require aggregate payload
    if value.tag() != ValueTag::Aggregate {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: 0,
        });
    }

    let handle = value.as_heap_handle().unwrap();

    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed heap cell
    let cell = state
        .interpreter
        .managed_heap
        .get(handle)
        .ok_or(Error::InvalidHeapHandle)?;

    // select field count for diagnostics
    let field_count_for_error = if field_count == UNKNOWN_FIELD_COUNT {
        cell.slots.len()
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
        && !cell.slots.is_empty()
        && slot_index >= cell.slots.len()
    {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: cell.slots.len(),
        });
    }

    // treat empty slot 0 as void
    if cell.slots.is_empty() && slot_index == 0 {
        return Ok(Value::VOID);
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "global field out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        let value = unsafe { *cell.slots.get_unchecked(slot_index) };
        return Ok(value);
    }

    // read the slot when in bounds
    if let Some(value) = cell.slots.get(slot_index).copied() {
        return Ok(value);
    }

    Err(Error::InvalidFieldAccess {
        index,
        field_count: cell.slots.len(),
    })
}

/// Store a field into a global allocation.
#[inline(always)]
pub(super) fn store_field_global(
    state: &mut ThreadedState<'_>,
    pointer: GlobalPointer,
    index: u32,
    field_count: u32,
    value: Value,
) -> Result<(), Error> {
    // track pointer stores
    state.interpreter.statistics.stores += 1;

    // validate field index when known
    check_field_index(state, index, field_count)?;

    // load the global value
    let current = state
        .interpreter
        .globals
        .get(pointer.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: pointer.id })?;

    // require aggregate payload
    if current.tag() != ValueTag::Aggregate {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: 0,
        });
    }

    let handle = current.as_heap_handle().unwrap();

    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed heap cell
    let cell = state
        .interpreter
        .managed_heap
        .get_mut(handle)
        .ok_or(Error::InvalidHeapHandle)?;

    // select field count for diagnostics
    let field_count_for_error = if field_count == UNKNOWN_FIELD_COUNT {
        cell.slots.len()
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
        && !cell.slots.is_empty()
        && slot_index >= cell.slots.len()
    {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: cell.slots.len(),
        });
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "global field out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        unsafe {
            *cell.slots.get_unchecked_mut(slot_index) = value;
        }
        state.interpreter.globals.set(pointer.id, current);
        return Ok(());
    }

    // write the slot when in bounds
    if let Some(slot) = cell.slots.get_mut(slot_index) {
        *slot = value;
        state.interpreter.globals.set(pointer.id, current);
        return Ok(());
    }

    Err(Error::InvalidFieldAccess {
        index,
        field_count: cell.slots.len(),
    })
}

/// Load an element from a managed heap allocation.
#[inline(always)]
pub(super) fn load_element_managed(
    state: &mut ThreadedState<'_>,
    handle: HeapHandle,
    index: u64,
    array_length: u64,
) -> Result<Value, Error> {
    // track pointer loads
    state.interpreter.statistics.loads += 1;

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed heap cell
    let cell = state
        .interpreter
        .managed_heap
        .get(handle)
        .ok_or(Error::InvalidHeapHandle)?;

    // compute the absolute slot offset
    let index_usize = if state.bounds_checks {
        usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
        })?
    } else {
        index as usize
    };
    let slot_index = if state.bounds_checks {
        handle
            .slot_index()
            .checked_add(index_usize)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: cell.slots.len() as u64,
            })?
    } else {
        handle.slot_index().wrapping_add(index_usize)
    };

    // validate bounds when array length is unknown
    if state.bounds_checks && array_length == UNKNOWN_ARRAY_LENGTH && slot_index >= cell.slots.len()
    {
        return Err(Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
        });
    }

    // treat empty slot 0 as void
    if cell.slots.is_empty() && slot_index == 0 {
        return Ok(Value::VOID);
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "heap element out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        let value = unsafe { *cell.slots.get_unchecked(slot_index) };
        return Ok(value);
    }

    // read the slot when in bounds
    if let Some(value) = cell.slots.get(slot_index).copied() {
        return Ok(value);
    }

    Err(Error::InvalidArrayAccess {
        index,
        length: cell.slots.len() as u64,
    })
}

/// Store an element into a managed heap allocation.
#[inline(always)]
pub(super) fn store_element_managed(
    state: &mut ThreadedState<'_>,
    handle: HeapHandle,
    index: u64,
    array_length: u64,
    value: Value,
) -> Result<(), Error> {
    // track pointer stores
    state.interpreter.statistics.stores += 1;

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed heap cell
    let cell = state
        .interpreter
        .managed_heap
        .get_mut(handle)
        .ok_or(Error::InvalidHeapHandle)?;

    // compute the absolute slot offset
    let index_usize = if state.bounds_checks {
        usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
        })?
    } else {
        index as usize
    };
    let slot_index = if state.bounds_checks {
        handle
            .slot_index()
            .checked_add(index_usize)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: cell.slots.len() as u64,
            })?
    } else {
        handle.slot_index().wrapping_add(index_usize)
    };

    // validate bounds when array length is unknown
    if state.bounds_checks && array_length == UNKNOWN_ARRAY_LENGTH && slot_index >= cell.slots.len()
    {
        return Err(Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
        });
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "heap element out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        unsafe {
            *cell.slots.get_unchecked_mut(slot_index) = value;
        }
        return Ok(());
    }

    // write the slot when in bounds
    if let Some(slot) = cell.slots.get_mut(slot_index) {
        *slot = value;
        return Ok(());
    }

    Err(Error::InvalidArrayAccess {
        index,
        length: cell.slots.len() as u64,
    })
}

/// Load an element from a raw heap allocation.
#[inline(always)]
pub(super) fn load_element_raw(
    state: &mut ThreadedState<'_>,
    pointer: RawPointer,
    index: u64,
    array_length: u64,
) -> Result<Value, Error> {
    // track pointer loads
    state.interpreter.statistics.loads += 1;

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the raw heap cell
    let cell = state
        .interpreter
        .raw_heap
        .get(pointer)
        .ok_or(Error::InvalidHeapHandle)?;

    // compute the absolute slot offset
    let index_usize = if state.bounds_checks {
        usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
        })?
    } else {
        index as usize
    };
    let slot_index = if state.bounds_checks {
        pointer
            .slot_index()
            .checked_add(index_usize)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: cell.slots.len() as u64,
            })?
    } else {
        pointer.slot_index().wrapping_add(index_usize)
    };

    // validate bounds when array length is unknown
    if state.bounds_checks && array_length == UNKNOWN_ARRAY_LENGTH && slot_index >= cell.slots.len()
    {
        return Err(Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
        });
    }

    // treat empty slot 0 as void
    if cell.slots.is_empty() && slot_index == 0 {
        return Ok(Value::VOID);
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "raw element out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        let value = unsafe { *cell.slots.get_unchecked(slot_index) };
        return Ok(value);
    }

    // read the slot when in bounds
    if let Some(value) = cell.slots.get(slot_index).copied() {
        return Ok(value);
    }

    Err(Error::InvalidArrayAccess {
        index,
        length: cell.slots.len() as u64,
    })
}

/// Store an element into a raw heap allocation.
#[inline(always)]
pub(super) fn store_element_raw(
    state: &mut ThreadedState<'_>,
    pointer: RawPointer,
    index: u64,
    array_length: u64,
    value: Value,
) -> Result<(), Error> {
    // track pointer stores
    state.interpreter.statistics.stores += 1;

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the raw heap cell
    let cell = state
        .interpreter
        .raw_heap
        .get_mut(pointer)
        .ok_or(Error::InvalidHeapHandle)?;

    // compute the absolute slot offset
    let index_usize = if state.bounds_checks {
        usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
        })?
    } else {
        index as usize
    };
    let slot_index = if state.bounds_checks {
        pointer
            .slot_index()
            .checked_add(index_usize)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: cell.slots.len() as u64,
            })?
    } else {
        pointer.slot_index().wrapping_add(index_usize)
    };

    // validate bounds when array length is unknown
    if state.bounds_checks && array_length == UNKNOWN_ARRAY_LENGTH && slot_index >= cell.slots.len()
    {
        return Err(Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
        });
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "raw element out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        unsafe {
            *cell.slots.get_unchecked_mut(slot_index) = value;
        }
        return Ok(());
    }

    // write the slot when in bounds
    if let Some(slot) = cell.slots.get_mut(slot_index) {
        *slot = value;
        return Ok(());
    }

    Err(Error::InvalidArrayAccess {
        index,
        length: cell.slots.len() as u64,
    })
}

/// Load an element from a stack allocation.
#[inline(always)]
pub(super) fn load_element_stack(
    state: &mut ThreadedState<'_>,
    pointer: StackPointer,
    index: u64,
    array_length: u64,
) -> Result<Value, Error> {
    // track pointer loads
    state.interpreter.statistics.loads += 1;

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // look up the stack cell
    let frame = state.frame_by_index(pointer.frame_idx)?;
    let cell = frame
        .get_stack_cell(pointer.slot)
        .ok_or(Error::InvalidHeapHandle)?;

    // compute the absolute slot offset
    let index_usize = if state.bounds_checks {
        usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
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
                length: cell.slots.len() as u64,
            })?
    } else {
        pointer.slot_offset.wrapping_add(index_usize)
    };

    // validate bounds when array length is unknown
    if state.bounds_checks && array_length == UNKNOWN_ARRAY_LENGTH && slot_index >= cell.slots.len()
    {
        return Err(Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
        });
    }

    // treat empty slot 0 as void
    if cell.slots.is_empty() && slot_index == 0 {
        return Ok(Value::VOID);
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "stack element out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        let value = unsafe { *cell.slots.get_unchecked(slot_index) };
        return Ok(value);
    }

    // read the slot when in bounds
    if let Some(value) = cell.slots.get(slot_index).copied() {
        return Ok(value);
    }

    Err(Error::InvalidArrayAccess {
        index,
        length: cell.slots.len() as u64,
    })
}

/// Store an element into a stack allocation.
#[inline(always)]
pub(super) fn store_element_stack(
    state: &mut ThreadedState<'_>,
    pointer: StackPointer,
    index: u64,
    array_length: u64,
    value: Value,
) -> Result<(), Error> {
    // cache bounds checks setting
    let bounds_checks = state.bounds_checks;

    // track pointer stores
    state.interpreter.statistics.stores += 1;

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // look up the stack cell
    let frame = state.frame_by_index_mut(pointer.frame_idx)?;
    let cell = frame
        .get_stack_cell_mut(pointer.slot)
        .ok_or(Error::InvalidHeapHandle)?;

    // compute the absolute slot offset
    let index_usize = if bounds_checks {
        usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
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
                length: cell.slots.len() as u64,
            })?
    } else {
        pointer.slot_offset.wrapping_add(index_usize)
    };

    // validate bounds when array length is unknown
    if bounds_checks && array_length == UNKNOWN_ARRAY_LENGTH && slot_index >= cell.slots.len() {
        return Err(Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
        });
    }

    // fast path without bounds checks
    if !bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "stack element out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        unsafe {
            *cell.slots.get_unchecked_mut(slot_index) = value;
        }
        return Ok(());
    }

    // write the slot when in bounds
    if let Some(slot) = cell.slots.get_mut(slot_index) {
        *slot = value;
        return Ok(());
    }

    Err(Error::InvalidArrayAccess {
        index,
        length: cell.slots.len() as u64,
    })
}

/// Load an element from a global allocation.
#[inline(always)]
pub(super) fn load_element_global(
    state: &mut ThreadedState<'_>,
    pointer: GlobalPointer,
    index: u64,
    array_length: u64,
) -> Result<Value, Error> {
    // track pointer loads
    state.interpreter.statistics.loads += 1;

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // load the global value
    let value = state
        .interpreter
        .globals
        .get(pointer.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: pointer.id })?;

    // require aggregate payload
    if value.tag() != ValueTag::Aggregate {
        return Err(Error::InvalidArrayAccess { index, length: 0 });
    }

    let handle = value.as_heap_handle().unwrap();

    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed heap cell
    let cell = state
        .interpreter
        .managed_heap
        .get(handle)
        .ok_or(Error::InvalidHeapHandle)?;

    // compute the absolute slot offset
    let index_usize = if state.bounds_checks {
        usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
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
                length: cell.slots.len() as u64,
            })?
    } else {
        pointer.slot_offset.wrapping_add(index_usize)
    };

    // validate bounds when array length is unknown
    if state.bounds_checks && array_length == UNKNOWN_ARRAY_LENGTH && slot_index >= cell.slots.len()
    {
        return Err(Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
        });
    }

    // treat empty slot 0 as void
    if cell.slots.is_empty() && slot_index == 0 {
        return Ok(Value::VOID);
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(
            slot_index < cell.slots.len(),
            "global element out of bounds"
        );
        // #Safety: bounds checks are disabled and slot is trusted
        let value = unsafe { *cell.slots.get_unchecked(slot_index) };
        return Ok(value);
    }

    // read the slot when in bounds
    if let Some(value) = cell.slots.get(slot_index).copied() {
        return Ok(value);
    }

    Err(Error::InvalidArrayAccess {
        index,
        length: cell.slots.len() as u64,
    })
}

/// Store an element into a global allocation.
#[inline(always)]
pub(super) fn store_element_global(
    state: &mut ThreadedState<'_>,
    pointer: GlobalPointer,
    index: u64,
    array_length: u64,
    value: Value,
) -> Result<(), Error> {
    // track pointer stores
    state.interpreter.statistics.stores += 1;

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // load the global value
    let current = state
        .interpreter
        .globals
        .get(pointer.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: pointer.id })?;

    // require aggregate payload
    if current.tag() != ValueTag::Aggregate {
        return Err(Error::InvalidArrayAccess { index, length: 0 });
    }

    let handle = current.as_heap_handle().unwrap();

    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed heap cell
    let cell = state
        .interpreter
        .managed_heap
        .get_mut(handle)
        .ok_or(Error::InvalidHeapHandle)?;

    // compute the absolute slot offset
    let index_usize = if state.bounds_checks {
        usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
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
                length: cell.slots.len() as u64,
            })?
    } else {
        pointer.slot_offset.wrapping_add(index_usize)
    };

    // validate bounds when array length is unknown
    if state.bounds_checks && array_length == UNKNOWN_ARRAY_LENGTH && slot_index >= cell.slots.len()
    {
        return Err(Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
        });
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(
            slot_index < cell.slots.len(),
            "global element out of bounds"
        );
        // #Safety: bounds checks are disabled and slot is trusted
        unsafe {
            *cell.slots.get_unchecked_mut(slot_index) = value;
        }
        state.interpreter.globals.set(pointer.id, current);
        return Ok(());
    }

    // write the slot when in bounds
    if let Some(slot) = cell.slots.get_mut(slot_index) {
        *slot = value;
        state.interpreter.globals.set(pointer.id, current);
        return Ok(());
    }

    Err(Error::InvalidArrayAccess {
        index,
        length: cell.slots.len() as u64,
    })
}

/// Get a field from an aggregate value.
#[inline(always)]
pub(super) fn get_field(
    state: &mut ThreadedState<'_>,
    agg: Value,
    index: u32,
) -> Result<Value, Error> {
    // resolve aggregate value
    match agg.tag() {
        ValueTag::Aggregate => {
            let handle = agg.as_heap_handle().unwrap();
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
pub(super) fn set_field(
    state: &mut ThreadedState<'_>,
    agg: Value,
    index: u32,
    val: Value,
) -> Result<Value, Error> {
    // resolve aggregate value
    match agg.tag() {
        ValueTag::Aggregate => {
            let handle = agg.as_heap_handle().unwrap();
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
pub(super) fn get_element(
    state: &mut ThreadedState<'_>,
    arr: Value,
    index: u64,
) -> Result<Value, Error> {
    // resolve array value
    match arr.tag() {
        ValueTag::Aggregate => {
            let handle = arr.as_heap_handle().unwrap();
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
pub(super) fn set_element(
    state: &mut ThreadedState<'_>,
    arr: Value,
    index: u64,
    val: Value,
) -> Result<Value, Error> {
    // resolve array value
    match arr.tag() {
        ValueTag::Aggregate => {
            let handle = arr.as_heap_handle().unwrap();
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
    state: &mut ThreadedState<'_>,
    handle: HeapHandle,
    slot_index: usize,
) -> Result<Value, Error> {
    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed heap cell
    let cell = state
        .interpreter
        .managed_heap
        .get(handle)
        .ok_or(Error::InvalidHeapHandle)?;

    // treat empty slot 0 as void
    if cell.slots.is_empty() && slot_index == 0 {
        return Ok(Value::VOID);
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "heap slot out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        let value = unsafe { *cell.slots.get_unchecked(slot_index) };
        return Ok(value);
    }

    // read the slot when in bounds
    if let Some(value) = cell.slots.get(slot_index).copied() {
        return Ok(value);
    }

    Err(Error::InvalidFieldAccess {
        index: slot_index as u32,
        field_count: cell.slots.len(),
    })
}

/// Store a slot into a managed heap allocation.
#[inline(always)]
fn store_heap_slot(
    state: &mut ThreadedState<'_>,
    handle: HeapHandle,
    slot_index: usize,
    value: Value,
) -> Result<(), Error> {
    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed heap cell
    let cell = state
        .interpreter
        .managed_heap
        .get_mut(handle)
        .ok_or(Error::InvalidHeapHandle)?;

    // resize slots as needed when bounds checks are enabled
    if state.bounds_checks && cell.slots.len() <= slot_index {
        cell.slots.resize(slot_index + 1, Value::VOID);
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "heap slot out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        unsafe {
            *cell.slots.get_unchecked_mut(slot_index) = value;
        }
        return Ok(());
    }

    // write slot when in bounds
    if let Some(slot) = cell.slots.get_mut(slot_index) {
        *slot = value;
        return Ok(());
    }

    Err(Error::InvalidFieldAccess {
        index: slot_index as u32,
        field_count: cell.slots.len(),
    })
}

/// Load a slot from a raw heap allocation.
#[inline(always)]
fn load_raw_slot(
    state: &mut ThreadedState<'_>,
    pointer: RawPointer,
    slot_index: usize,
) -> Result<Value, Error> {
    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the raw heap cell
    let cell = state
        .interpreter
        .raw_heap
        .get(pointer)
        .ok_or(Error::InvalidHeapHandle)?;

    // treat empty slot 0 as void
    if cell.slots.is_empty() && slot_index == 0 {
        return Ok(Value::VOID);
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "raw slot out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        let value = unsafe { *cell.slots.get_unchecked(slot_index) };
        return Ok(value);
    }

    // read the slot when in bounds
    if let Some(value) = cell.slots.get(slot_index).copied() {
        return Ok(value);
    }

    Err(Error::InvalidFieldAccess {
        index: slot_index as u32,
        field_count: cell.slots.len(),
    })
}

/// Store a slot into a raw heap allocation.
#[inline(always)]
fn store_raw_slot(
    state: &mut ThreadedState<'_>,
    pointer: RawPointer,
    slot_index: usize,
    value: Value,
) -> Result<(), Error> {
    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the raw heap cell
    let cell = state
        .interpreter
        .raw_heap
        .get_mut(pointer)
        .ok_or(Error::InvalidHeapHandle)?;

    // resize slots as needed when bounds checks are enabled
    if state.bounds_checks && cell.slots.len() <= slot_index {
        cell.slots.resize(slot_index + 1, Value::VOID);
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "raw slot out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        unsafe {
            *cell.slots.get_unchecked_mut(slot_index) = value;
        }
        return Ok(());
    }

    // write slot when in bounds
    if let Some(slot) = cell.slots.get_mut(slot_index) {
        *slot = value;
        return Ok(());
    }

    Err(Error::InvalidFieldAccess {
        index: slot_index as u32,
        field_count: cell.slots.len(),
    })
}

/// Load from a global pointer, including slot offsets.
#[inline(always)]
fn load_global_slot(state: &mut ThreadedState<'_>, global: GlobalPointer) -> Result<Value, Error> {
    // read the global value
    let value = state
        .interpreter
        .globals
        .get(global.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: global.id })?;

    // return the global value directly
    if global.slot_offset == 0 {
        return Ok(value);
    }

    // read from the aggregate stored in the global
    if value.tag() != ValueTag::Aggregate {
        return Err(Error::InvalidFieldAccess {
            index: global.slot_offset as u32,
            field_count: 0,
        });
    }

    let handle = value.as_heap_handle().unwrap();

    load_heap_slot(state, handle, global.slot_offset)
}

/// Store through a global pointer, including slot offsets.
#[inline(always)]
fn store_global_slot(
    state: &mut ThreadedState<'_>,
    global: GlobalPointer,
    value: Value,
) -> Result<(), Error> {
    // update the global value directly
    if global.slot_offset == 0 {
        state.interpreter.globals.set(global.id, value);
        return Ok(());
    }

    // update a slot on the aggregate stored in the global
    let current = state
        .interpreter
        .globals
        .get(global.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: global.id })?;
    if current.tag() != ValueTag::Aggregate {
        return Err(Error::InvalidFieldAccess {
            index: global.slot_offset as u32,
            field_count: 0,
        });
    }

    let handle = current.as_heap_handle().unwrap();

    store_heap_slot(state, handle, global.slot_offset, value)?;
    state.interpreter.globals.set(global.id, current);
    Ok(())
}

/// Get a field from a heap aggregate.
#[inline(always)]
fn get_heap_field(
    state: &mut ThreadedState<'_>,
    handle: HeapHandle,
    index: u32,
) -> Result<Value, Error> {
    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed heap cell
    let cell = state
        .interpreter
        .managed_heap
        .get(handle)
        .ok_or(Error::InvalidHeapHandle)?;

    // resolve the target slot
    let slot_index = if state.bounds_checks {
        handle
            .slot_index()
            .checked_add(index as usize)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: cell.slots.len(),
            })?
    } else {
        handle.slot_index().wrapping_add(index as usize)
    };

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "heap field out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        let value = unsafe { *cell.slots.get_unchecked(slot_index) };
        return Ok(value);
    }

    // read the slot when in bounds
    let value = cell
        .slots
        .get(slot_index)
        .copied()
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: cell.slots.len(),
        })?;

    Ok(value)
}

/// Set a field on a heap aggregate.
#[inline(always)]
fn set_heap_field(
    state: &mut ThreadedState<'_>,
    handle: HeapHandle,
    index: u32,
    value: Value,
) -> Result<(), Error> {
    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed heap cell
    let cell = state
        .interpreter
        .managed_heap
        .get_mut(handle)
        .ok_or(Error::InvalidHeapHandle)?;

    // resolve the target slot
    let slot_index = if state.bounds_checks {
        handle
            .slot_index()
            .checked_add(index as usize)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: cell.slots.len(),
            })?
    } else {
        handle.slot_index().wrapping_add(index as usize)
    };

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "heap field out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        unsafe {
            *cell.slots.get_unchecked_mut(slot_index) = value;
        }
        return Ok(());
    }

    // write slot when in bounds
    if let Some(slot) = cell.slots.get_mut(slot_index) {
        *slot = value;
        return Ok(());
    }

    Err(Error::InvalidFieldAccess {
        index,
        field_count: cell.slots.len(),
    })
}

/// Get an element from a heap array aggregate.
#[inline(always)]
fn get_heap_element(
    state: &mut ThreadedState<'_>,
    handle: HeapHandle,
    index: u64,
) -> Result<Value, Error> {
    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed heap cell
    let cell = state
        .interpreter
        .managed_heap
        .get(handle)
        .ok_or(Error::InvalidHeapHandle)?;

    // resolve the target slot
    let index_usize = if state.bounds_checks {
        usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
        })?
    } else {
        index as usize
    };
    let slot_index = if state.bounds_checks {
        handle
            .slot_index()
            .checked_add(index_usize)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: cell.slots.len() as u64,
            })?
    } else {
        handle.slot_index().wrapping_add(index_usize)
    };

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "heap element out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        let value = unsafe { *cell.slots.get_unchecked(slot_index) };
        return Ok(value);
    }

    // read the slot when in bounds
    let value = cell
        .slots
        .get(slot_index)
        .copied()
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
        })?;

    Ok(value)
}

/// Set an element on a heap array aggregate.
#[inline(always)]
fn set_heap_element(
    state: &mut ThreadedState<'_>,
    handle: HeapHandle,
    index: u64,
    value: Value,
) -> Result<(), Error> {
    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed heap cell
    let cell = state
        .interpreter
        .managed_heap
        .get_mut(handle)
        .ok_or(Error::InvalidHeapHandle)?;

    // resolve the target slot
    let index_usize = if state.bounds_checks {
        usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
        })?
    } else {
        index as usize
    };
    let slot_index = if state.bounds_checks {
        handle
            .slot_index()
            .checked_add(index_usize)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: cell.slots.len() as u64,
            })?
    } else {
        handle.slot_index().wrapping_add(index_usize)
    };

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "heap element out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        unsafe {
            *cell.slots.get_unchecked_mut(slot_index) = value;
        }
        return Ok(());
    }

    // write slot when in bounds
    if let Some(slot) = cell.slots.get_mut(slot_index) {
        *slot = value;
        return Ok(());
    }

    Err(Error::InvalidArrayAccess {
        index,
        length: cell.slots.len() as u64,
    })
}

/// Resolve a field slot for a managed heap pointer.
#[inline(always)]
fn resolve_heap_field_slot(
    state: &mut ThreadedState<'_>,
    handle: HeapHandle,
    index: u32,
    field_count: u32,
) -> Result<u32, Error> {
    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed heap cell
    let cell = state
        .interpreter
        .managed_heap
        .get(handle)
        .ok_or(Error::InvalidHeapHandle)?;

    // select field count for diagnostics
    let field_count_for_error = if field_count == UNKNOWN_FIELD_COUNT {
        cell.slots.len()
    } else {
        field_count as usize
    };

    // compute the absolute slot offset
    let slot_index = if state.bounds_checks {
        handle
            .slot_index()
            .checked_add(index as usize)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: field_count_for_error,
            })?
    } else {
        handle.slot_index().wrapping_add(index as usize)
    };

    // validate bounds when field count is unknown
    if state.bounds_checks
        && field_count == UNKNOWN_FIELD_COUNT
        && !cell.slots.is_empty()
        && slot_index >= cell.slots.len()
    {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: cell.slots.len(),
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

/// Resolve a field slot for a raw heap pointer.
#[inline(always)]
fn resolve_raw_field_slot(
    state: &mut ThreadedState<'_>,
    pointer: RawPointer,
    index: u32,
    field_count: u32,
) -> Result<u32, Error> {
    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the raw heap cell
    let cell = state
        .interpreter
        .raw_heap
        .get(pointer)
        .ok_or(Error::InvalidHeapHandle)?;

    // select field count for diagnostics
    let field_count_for_error = if field_count == UNKNOWN_FIELD_COUNT {
        cell.slots.len()
    } else {
        field_count as usize
    };

    // compute the absolute slot offset
    let slot_index = if state.bounds_checks {
        pointer
            .slot_index()
            .checked_add(index as usize)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: field_count_for_error,
            })?
    } else {
        pointer.slot_index().wrapping_add(index as usize)
    };

    // validate bounds when field count is unknown
    if state.bounds_checks
        && field_count == UNKNOWN_FIELD_COUNT
        && !cell.slots.is_empty()
        && slot_index >= cell.slots.len()
    {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: cell.slots.len(),
        });
    }

    // narrow slot index for pointer encoding
    if state.bounds_checks {
        return u32::try_from(slot_index).map_err(|_| Error::InvalidFieldAccess {
            index,
            field_count: field_count_for_error,
        });
    }

    debug_assert!(slot_index <= u32::MAX as usize, "raw slot index overflow");
    Ok(slot_index as u32)
}

/// Resolve a field slot for a stack pointer.
#[inline(always)]
fn resolve_stack_field_slot(
    state: &mut ThreadedState<'_>,
    pointer: StackPointer,
    index: u32,
    field_count: u32,
) -> Result<usize, Error> {
    // look up the stack cell
    let frame = state.frame_by_index(pointer.frame_idx)?;
    let cell = frame
        .get_stack_cell(pointer.slot)
        .ok_or(Error::InvalidHeapHandle)?;

    // compute the absolute slot offset
    let slot_index = if state.bounds_checks {
        pointer
            .slot_offset
            .checked_add(index as usize)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: cell.slots.len(),
            })?
    } else {
        pointer.slot_offset.wrapping_add(index as usize)
    };

    // validate bounds when field count is unknown
    if state.bounds_checks
        && field_count == UNKNOWN_FIELD_COUNT
        && !cell.slots.is_empty()
        && slot_index >= cell.slots.len()
    {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: cell.slots.len(),
        });
    }

    Ok(slot_index)
}

/// Resolve a field slot for a global pointer.
#[inline(always)]
fn resolve_global_field_slot(
    state: &mut ThreadedState<'_>,
    global: GlobalPointer,
    index: u32,
    field_count: u32,
) -> Result<usize, Error> {
    // load the global value
    let value = state
        .interpreter
        .globals
        .get(global.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: global.id })?;

    // require aggregate payload
    if value.tag() != ValueTag::Aggregate {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: 0,
        });
    }

    let handle = value.as_heap_handle().unwrap();

    // look up the managed heap cell
    let cell = state
        .interpreter
        .managed_heap
        .get(handle)
        .ok_or(Error::InvalidHeapHandle)?;

    // compute the absolute slot offset
    let slot_index = if state.bounds_checks {
        global
            .slot_offset
            .checked_add(index as usize)
            .ok_or(Error::InvalidFieldAccess {
                index,
                field_count: cell.slots.len(),
            })?
    } else {
        global.slot_offset.wrapping_add(index as usize)
    };

    // validate bounds when field count is unknown
    if state.bounds_checks
        && field_count == UNKNOWN_FIELD_COUNT
        && !cell.slots.is_empty()
        && slot_index >= cell.slots.len()
    {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: cell.slots.len(),
        });
    }

    Ok(slot_index)
}

/// Resolve an element slot for a managed heap pointer.
#[inline(always)]
fn resolve_heap_element_slot(
    state: &mut ThreadedState<'_>,
    handle: HeapHandle,
    index: u64,
    array_length: u64,
) -> Result<u32, Error> {
    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the managed heap cell
    let cell = state
        .interpreter
        .managed_heap
        .get(handle)
        .ok_or(Error::InvalidHeapHandle)?;

    // compute the absolute slot offset
    let index_usize = if state.bounds_checks {
        usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
        })?
    } else {
        index as usize
    };
    let slot_index = if state.bounds_checks {
        handle
            .slot_index()
            .checked_add(index_usize)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: cell.slots.len() as u64,
            })?
    } else {
        handle.slot_index().wrapping_add(index_usize)
    };

    // validate bounds when array length is unknown
    if state.bounds_checks && array_length == UNKNOWN_ARRAY_LENGTH && slot_index >= cell.slots.len()
    {
        return Err(Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
        });
    }

    // narrow slot index for pointer encoding
    if state.bounds_checks {
        return u32::try_from(slot_index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
        });
    }

    debug_assert!(
        slot_index <= u32::MAX as usize,
        "heap element slot index overflow"
    );
    Ok(slot_index as u32)
}

/// Resolve an element slot for a raw heap pointer.
#[inline(always)]
fn resolve_raw_element_slot(
    state: &mut ThreadedState<'_>,
    pointer: RawPointer,
    index: u64,
    array_length: u64,
) -> Result<u32, Error> {
    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // look up the raw heap cell
    let cell = state
        .interpreter
        .raw_heap
        .get(pointer)
        .ok_or(Error::InvalidHeapHandle)?;

    // compute the absolute slot offset
    let index_usize = if state.bounds_checks {
        usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
        })?
    } else {
        index as usize
    };
    let slot_index = if state.bounds_checks {
        pointer
            .slot_index()
            .checked_add(index_usize)
            .ok_or(Error::InvalidArrayAccess {
                index,
                length: cell.slots.len() as u64,
            })?
    } else {
        pointer.slot_index().wrapping_add(index_usize)
    };

    // validate bounds when array length is unknown
    if state.bounds_checks && array_length == UNKNOWN_ARRAY_LENGTH && slot_index >= cell.slots.len()
    {
        return Err(Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
        });
    }

    // narrow slot index for pointer encoding
    if state.bounds_checks {
        return u32::try_from(slot_index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
        });
    }

    debug_assert!(
        slot_index <= u32::MAX as usize,
        "raw element slot index overflow"
    );
    Ok(slot_index as u32)
}

/// Resolve an element slot for a stack pointer.
#[inline(always)]
fn resolve_stack_element_slot(
    state: &mut ThreadedState<'_>,
    pointer: StackPointer,
    index: u64,
    array_length: u64,
) -> Result<usize, Error> {
    // look up the stack cell
    let frame = state.frame_by_index(pointer.frame_idx)?;
    let cell = frame
        .get_stack_cell(pointer.slot)
        .ok_or(Error::InvalidHeapHandle)?;

    // compute the absolute slot offset
    let index_usize = if state.bounds_checks {
        usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
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
                length: cell.slots.len() as u64,
            })?
    } else {
        pointer.slot_offset.wrapping_add(index_usize)
    };

    // validate bounds when array length is unknown
    if state.bounds_checks && array_length == UNKNOWN_ARRAY_LENGTH && slot_index >= cell.slots.len()
    {
        return Err(Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
        });
    }

    Ok(slot_index)
}

/// Resolve an element slot for a global pointer.
#[inline(always)]
fn resolve_global_element_slot(
    state: &mut ThreadedState<'_>,
    global: GlobalPointer,
    index: u64,
    array_length: u64,
) -> Result<usize, Error> {
    // load the global value
    let value = state
        .interpreter
        .globals
        .get(global.id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: global.id })?;

    // require aggregate payload
    if value.tag() != ValueTag::Aggregate {
        return Err(Error::InvalidArrayAccess { index, length: 0 });
    }

    let handle = value.as_heap_handle().unwrap();

    // look up the managed heap cell
    let cell = state
        .interpreter
        .managed_heap
        .get(handle)
        .ok_or(Error::InvalidHeapHandle)?;

    // compute the absolute slot offset
    let index_usize = if state.bounds_checks {
        usize::try_from(index).map_err(|_| Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
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
                length: cell.slots.len() as u64,
            })?
    } else {
        global.slot_offset.wrapping_add(index_usize)
    };

    // validate bounds when array length is unknown
    if state.bounds_checks && array_length == UNKNOWN_ARRAY_LENGTH && slot_index >= cell.slots.len()
    {
        return Err(Error::InvalidArrayAccess {
            index,
            length: cell.slots.len() as u64,
        });
    }

    Ok(slot_index)
}

/// Load a slot from a stack allocation.
#[inline(always)]
fn load_stack_slot(
    state: &mut ThreadedState<'_>,
    sp: StackPointer,
    slot_index: usize,
) -> Result<Value, Error> {
    // resolve the stack cell
    let frame = state.frame_by_index(sp.frame_idx)?;
    let cell = frame
        .get_stack_cell(sp.slot)
        .ok_or(Error::InvalidHeapHandle)?;

    // treat empty slot 0 as void
    if cell.slots.is_empty() && slot_index == 0 {
        return Ok(Value::VOID);
    }

    // fast path without bounds checks
    if !state.bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "stack slot out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        let value = unsafe { *cell.slots.get_unchecked(slot_index) };
        return Ok(value);
    }

    // read the slot when in bounds
    if let Some(value) = cell.slots.get(slot_index).copied() {
        return Ok(value);
    }

    Err(Error::InvalidFieldAccess {
        index: slot_index as u32,
        field_count: cell.slots.len(),
    })
}

/// Store a slot into a stack allocation.
#[inline(always)]
fn store_stack_slot(
    state: &mut ThreadedState<'_>,
    sp: StackPointer,
    slot_index: usize,
    value: Value,
) -> Result<(), Error> {
    // cache bounds checks setting
    let bounds_checks = state.bounds_checks;

    // resolve the stack cell
    let frame = state.frame_by_index_mut(sp.frame_idx)?;
    let cell = frame
        .get_stack_cell_mut(sp.slot)
        .ok_or(Error::InvalidHeapHandle)?;

    // resize slots as needed when bounds checks are enabled
    if bounds_checks && cell.slots.len() <= slot_index {
        cell.slots.resize(slot_index + 1, Value::VOID);
    }

    // fast path without bounds checks
    if !bounds_checks {
        debug_assert!(slot_index < cell.slots.len(), "stack slot out of bounds");
        // #Safety: bounds checks are disabled and slot is trusted
        unsafe {
            *cell.slots.get_unchecked_mut(slot_index) = value;
        }
        return Ok(());
    }

    // write slot when in bounds
    if let Some(slot) = cell.slots.get_mut(slot_index) {
        *slot = value;
        return Ok(());
    }

    Err(Error::InvalidFieldAccess {
        index: slot_index as u32,
        field_count: cell.slots.len(),
    })
}
