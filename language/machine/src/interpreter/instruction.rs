use crate::diagnostic::Error;
use crate::memory::{StackPointer, Value, ValueTag};

use super::threaded::ThreadedState;

/// Load a value from a pointer.
#[inline(always)]
pub(super) fn load_from_pointer(state: &mut ThreadedState<'_>, ptr: Value) -> Result<Value, Error> {
    state.interpreter.statistics.loads += 1;

    match ptr.tag() {
        ValueTag::ManagedReference | ValueTag::Aggregate => {
            let handle = ptr.as_heap_handle().unwrap();
            if handle.is_null() {
                return Err(Error::NullPointerDereference);
            }
            if let Some(cell) = state.interpreter.managed_heap.get(handle) {
                Ok(cell.slots.first().copied().unwrap_or(Value::VOID))
            } else {
                Err(Error::InvalidHeapHandle)
            }
        }
        ValueTag::RawPointer => {
            let rp = ptr.as_raw_pointer().unwrap();
            if rp.is_null() {
                return Err(Error::NullPointerDereference);
            }
            if let Some(cell) = state.interpreter.raw_heap.get(rp) {
                Ok(cell.slots.first().copied().unwrap_or(Value::VOID))
            } else {
                Err(Error::InvalidHeapHandle)
            }
        }
        ValueTag::StackPointer => {
            let sp = ptr.as_stack_pointer().unwrap();
            load_stack_slot(state, sp, 0)
        }
        ValueTag::GlobalPointer => {
            let global = ptr.as_global_pointer().unwrap();
            state
                .interpreter
                .globals
                .get(global)
                .copied()
                .ok_or(Error::UndefinedGlobal { global })
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
    state.interpreter.statistics.stores += 1;

    match ptr.tag() {
        ValueTag::ManagedReference => {
            let handle = ptr.as_heap_handle().unwrap();
            if handle.is_null() {
                return Err(Error::NullPointerDereference);
            }
            if let Some(cell) = state.interpreter.managed_heap.get_mut(handle) {
                if cell.slots.is_empty() {
                    cell.slots.push(val);
                } else {
                    cell.slots[0] = val;
                }
                Ok(())
            } else {
                Err(Error::InvalidHeapHandle)
            }
        }
        ValueTag::RawPointer => {
            let rp = ptr.as_raw_pointer().unwrap();
            if rp.is_null() {
                return Err(Error::NullPointerDereference);
            }
            if let Some(cell) = state.interpreter.raw_heap.get_mut(rp) {
                if cell.slots.is_empty() {
                    cell.slots.push(val);
                } else {
                    cell.slots[0] = val;
                }
                Ok(())
            } else {
                Err(Error::InvalidHeapHandle)
            }
        }
        ValueTag::StackPointer => {
            let sp = ptr.as_stack_pointer().unwrap();
            store_stack_slot(state, sp, 0, val)
        }
        ValueTag::GlobalPointer => {
            let global = ptr.as_global_pointer().unwrap();
            let global_def = state.interpreter.tree.get(global);
            if !global_def.is_mutable() {
                return Err(Error::ImmutableGlobalWrite { global });
            }
            state.interpreter.globals.set(global, val);
            Ok(())
        }
        _ => Err(Error::InvalidPointerType {
            actual: format!("{ptr:?}"),
        }),
    }
}

/// Get a field from an aggregate value.
#[inline(always)]
pub(super) fn get_field(
    state: &mut ThreadedState<'_>,
    agg: Value,
    index: u32,
) -> Result<Value, Error> {
    match agg.tag() {
        ValueTag::Aggregate | ValueTag::ManagedReference => {
            let handle = agg.as_heap_handle().unwrap();
            if handle.is_null() {
                return Err(Error::NullPointerDereference);
            }
            if let Some(cell) = state.interpreter.managed_heap.get(handle) {
                cell.slots
                    .get(index as usize)
                    .copied()
                    .ok_or(Error::InvalidFieldAccess {
                        index,
                        field_count: cell.slots.len(),
                    })
            } else {
                Err(Error::InvalidHeapHandle)
            }
        }
        ValueTag::RawPointer => {
            let rp = agg.as_raw_pointer().unwrap();
            if rp.is_null() {
                return Err(Error::NullPointerDereference);
            }
            if let Some(cell) = state.interpreter.raw_heap.get(rp) {
                cell.slots
                    .get(index as usize)
                    .copied()
                    .ok_or(Error::InvalidFieldAccess {
                        index,
                        field_count: cell.slots.len(),
                    })
            } else {
                Err(Error::InvalidHeapHandle)
            }
        }
        ValueTag::StackPointer => {
            let sp = agg.as_stack_pointer().unwrap();
            load_stack_slot(state, sp, index as usize)
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
    match agg.tag() {
        ValueTag::Aggregate | ValueTag::ManagedReference => {
            let handle = agg.as_heap_handle().unwrap();
            if handle.is_null() {
                return Err(Error::NullPointerDereference);
            }
            let mut invalid_field_count = None;
            if let Some(cell) = state.interpreter.managed_heap.get_mut(handle) {
                if (index as usize) < cell.slots.len() {
                    cell.slots[index as usize] = val;
                } else {
                    invalid_field_count = Some(cell.slots.len());
                }
            } else {
                return Err(Error::InvalidHeapHandle);
            }
            if let Some(field_count) = invalid_field_count {
                return Err(Error::InvalidFieldAccess { index, field_count });
            }
            Ok(agg)
        }
        ValueTag::RawPointer => {
            let rp = agg.as_raw_pointer().unwrap();
            if rp.is_null() {
                return Err(Error::NullPointerDereference);
            }
            let mut invalid_field_count = None;
            if let Some(cell) = state.interpreter.raw_heap.get_mut(rp) {
                if (index as usize) < cell.slots.len() {
                    cell.slots[index as usize] = val;
                } else {
                    invalid_field_count = Some(cell.slots.len());
                }
            } else {
                return Err(Error::InvalidHeapHandle);
            }
            if let Some(field_count) = invalid_field_count {
                return Err(Error::InvalidFieldAccess { index, field_count });
            }
            Ok(Value::raw_pointer(rp))
        }
        ValueTag::StackPointer => {
            let sp = agg.as_stack_pointer().unwrap();
            store_stack_slot(state, sp, index as usize, val)?;
            Ok(Value::stack_pointer(sp))
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
    match arr.tag() {
        ValueTag::Aggregate | ValueTag::ManagedReference => {
            let handle = arr.as_heap_handle().unwrap();
            if handle.is_null() {
                return Err(Error::NullPointerDereference);
            }
            if let Some(cell) = state.interpreter.managed_heap.get(handle) {
                cell.slots
                    .get(index as usize)
                    .copied()
                    .ok_or(Error::InvalidArrayAccess {
                        index,
                        length: cell.slots.len() as u64,
                    })
            } else {
                Err(Error::InvalidHeapHandle)
            }
        }
        ValueTag::RawPointer => {
            let rp = arr.as_raw_pointer().unwrap();
            if rp.is_null() {
                return Err(Error::NullPointerDereference);
            }
            if let Some(cell) = state.interpreter.raw_heap.get(rp) {
                cell.slots
                    .get(index as usize)
                    .copied()
                    .ok_or(Error::InvalidArrayAccess {
                        index,
                        length: cell.slots.len() as u64,
                    })
            } else {
                Err(Error::InvalidHeapHandle)
            }
        }
        ValueTag::StackPointer => {
            let sp = arr.as_stack_pointer().unwrap();
            load_stack_slot(state, sp, index as usize)
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
    match arr.tag() {
        ValueTag::Aggregate | ValueTag::ManagedReference => {
            let handle = arr.as_heap_handle().unwrap();
            if handle.is_null() {
                return Err(Error::NullPointerDereference);
            }
            let mut invalid_length = None;
            if let Some(cell) = state.interpreter.managed_heap.get_mut(handle) {
                if (index as usize) < cell.slots.len() {
                    cell.slots[index as usize] = val;
                } else {
                    invalid_length = Some(cell.slots.len() as u64);
                }
            } else {
                return Err(Error::InvalidHeapHandle);
            }
            if let Some(length) = invalid_length {
                return Err(Error::InvalidArrayAccess { index, length });
            }
            Ok(arr)
        }
        ValueTag::RawPointer => {
            let rp = arr.as_raw_pointer().unwrap();
            if rp.is_null() {
                return Err(Error::NullPointerDereference);
            }
            let mut invalid_length = None;
            if let Some(cell) = state.interpreter.raw_heap.get_mut(rp) {
                if (index as usize) < cell.slots.len() {
                    cell.slots[index as usize] = val;
                } else {
                    invalid_length = Some(cell.slots.len() as u64);
                }
            } else {
                return Err(Error::InvalidHeapHandle);
            }
            if let Some(length) = invalid_length {
                return Err(Error::InvalidArrayAccess { index, length });
            }
            Ok(Value::raw_pointer(rp))
        }
        ValueTag::StackPointer => {
            let sp = arr.as_stack_pointer().unwrap();
            store_stack_slot(state, sp, index as usize, val)?;
            Ok(Value::stack_pointer(sp))
        }
        _ => Err(Error::TypeMismatch {
            expected: "array".to_string(),
            actual: format!("{arr:?}"),
        }),
    }
}

/// Load a slot from a stack allocation.
#[inline(always)]
fn load_stack_slot(
    state: &mut ThreadedState<'_>,
    sp: StackPointer,
    slot_index: usize,
) -> Result<Value, Error> {
    let frame = state.frame_by_index(sp.frame_idx)?;
    let cell = frame
        .get_stack_cell(sp.slot)
        .ok_or(Error::InvalidHeapHandle)?;

    if let Some(value) = cell.slots.get(slot_index).copied() {
        return Ok(value);
    }

    if cell.slots.is_empty() && slot_index == 0 {
        return Ok(Value::VOID);
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
    let frame = state.frame_by_index_mut(sp.frame_idx)?;
    let cell = frame
        .get_stack_cell_mut(sp.slot)
        .ok_or(Error::InvalidHeapHandle)?;

    while cell.slots.len() <= slot_index {
        cell.slots.push(Value::VOID);
    }
    cell.slots[slot_index] = value;
    Ok(())
}
