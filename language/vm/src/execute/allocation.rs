use super::reference::check_reference_address_space;
use super::slice::{load_slice_length, store_slice};
use crate::diagnostic::Error;
use crate::interpreter::DispatchState;
use crate::program::{
    AsyncDispose, Dispose, DropValue, Instruction, New, NewSlice, Pin, PointerClass, RawAlloc,
    RawFree, StackAlloc, Transfer, UnpinValue, ValueLayout, value_layout_from_type,
};
use crate::{HeapReference, RawPointer, StackPointer, Word};
use destack_heap::{AllocationPlan, HeapError, Payload, SharedRawPointer, repeated_layout};
use destack_mir as mir;

/// Execute local heap allocation.
pub(crate) fn execute_allocate_heap(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let New { dest, allocation } = instruction.payload_as::<New>();
    let table = state.operand_table_ptr();
    let allocation = unsafe { (*table).allocation_layout(*allocation) };
    let reference_map = unsafe { (*table).reference_map(allocation.reference_map) };
    let class = unsafe { (*table).allocation_class(allocation.class) };

    // stay on the active young run when the compiled class permits it
    let reference = if allocation.is_noscan
        && let Some(small) = class.small()
    {
        match state.reserve_young(small) {
            Some(reference) => reference,
            None => {
                match state
                    .allocate_zeroed_heap_layout(&allocation.heap_layout(reference_map, class))
                {
                    Ok(reference) => reference,
                    Err(error) => return Transfer::Error(error),
                }
            }
        }
    } else {
        match state.allocate_zeroed_heap_layout(&allocation.heap_layout(reference_map, class)) {
            Ok(reference) => reference,
            Err(error) => return Transfer::Error(error),
        }
    };

    // store result
    state.set_word(*dest, Word::heap_reference(reference));

    Transfer::Continue
}

/// Execute shared heap allocation.
pub(crate) fn execute_allocate_shared_heap(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let New { dest, allocation } = instruction.payload_as::<New>();
    let table = state.operand_table_ptr();
    let allocation = unsafe { (*table).allocation_layout(*allocation) };
    let reference_map = unsafe { (*table).reference_map(allocation.reference_map) };
    let class = unsafe { (*table).allocation_class(allocation.class) };

    // stay on the active worker run when the compiled class permits it
    let reference = if allocation.is_noscan
        && let Some(small) = class.small()
    {
        match state.reserve_shared_small(small) {
            Some(reference) => reference,
            None => {
                match state.allocate_zeroed_shared_heap_layout(
                    &allocation.heap_layout(reference_map, class),
                ) {
                    Ok(reference) => reference,
                    Err(error) => return Transfer::Error(error),
                }
            }
        }
    } else {
        match state
            .allocate_zeroed_shared_heap_layout(&allocation.heap_layout(reference_map, class))
        {
            Ok(reference) => reference,
            Err(error) => return Transfer::Error(error),
        }
    };

    // store result
    state.set_word(*dest, Word::shared_heap_reference(reference));

    Transfer::Continue
}

/// Execute slice allocation.
pub(crate) fn execute_allocate_slice(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let NewSlice {
        dest,
        length,
        pointer_class,
        element,
        element_alignment,
    } = instruction.payload_as::<NewSlice>();
    let table = state.operand_table_ptr();
    let element = unsafe { (*table).allocation_layout(*element) };
    let element_reference_map = unsafe { (*table).reference_map(element.reference_map) };

    // load slice length
    let length = match load_slice_length(state, *length) {
        Ok(length) => length,
        Err(error) => return Transfer::Error(error),
    };

    // build the backing array allocation plan
    let backing_reference = {
        let element_plan = element.plan(element_reference_map);
        let (byte_len, reference_map) = match repeated_layout(
            element_plan.byte_len,
            *element_alignment,
            element_plan.reference_map,
            length,
        ) {
            Ok(layout) => layout,
            Err(error) => return Transfer::Error(Error::from(error)),
        };
        let plan = AllocationPlan::new(byte_len, *element_alignment, &reference_map);

        match pointer_class {
            PointerClass::Heap => state
                .allocate_zeroed_heap_plan(plan)
                .map(Word::heap_reference),
            PointerClass::SharedHeap => state
                .allocate_zeroed_shared_heap_plan(plan)
                .map(Word::shared_heap_reference),
            _ => Err(Error::InvalidPointerType {
                actual: format!("{pointer_class:?}"),
            }),
        }
    };
    let backing_reference = match backing_reference {
        Ok(reference) => reference,
        Err(error) => return Transfer::Error(error),
    };

    // write the slice descriptor
    if let Err(error) = store_slice(state, *dest, backing_reference, length) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute raw allocation.
pub(crate) fn execute_allocate_raw(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let RawAlloc { dest, byte_len } = instruction.payload_as::<RawAlloc>();

    // allocate raw heap bytes
    let pointer = state.heap_mut().allocate_raw(*byte_len, Payload::Zeroed);
    let pointer = match pointer {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(Error::from(error)),
    };
    let value = Word::raw_pointer(pointer);

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute raw free.
pub(crate) fn execute_free_raw(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let RawFree { pointer } = instruction.payload_as::<RawFree>();

    // free the pointed raw allocation
    let pointer = state.get(*pointer);
    let pointer = RawPointer::from_bits(pointer.bits() as usize);
    let heap = state.heap_mut();
    match heap.free_raw(pointer) {
        Ok(true) => {}
        Ok(false) => return Transfer::Error(Error::InvalidRawPointer),
        Err(HeapError::InvalidRawPointer { .. }) => {
            return Transfer::Error(Error::InvalidRawPointer);
        }
        Err(error) => return Transfer::Error(Error::from(error)),
    }

    Transfer::Continue
}

/// Execute explicit synchronous cleanup.
pub(crate) fn execute_dispose(
    _state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let _ = instruction.payload_as::<Dispose>();

    Transfer::Continue
}

/// Execute explicit asynchronous cleanup.
pub(crate) fn execute_async_dispose(
    _state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let _ = instruction.payload_as::<AsyncDispose>();

    Transfer::Continue
}

/// Execute local heap pin.
pub(crate) fn execute_pin(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let Pin { value } = instruction.payload_as::<Pin>();

    // require a local heap reference
    let pinned_value = state.get(*value);
    let value_type = match state.value_type(*value) {
        Ok(value_type) => value_type,
        Err(error) => return Transfer::Error(error),
    };
    let repr = value_layout_from_type(state.tree(), value_type);
    if !matches!(
        repr,
        ValueLayout::Pointer {
            pointer_class: PointerClass::Heap,
            ..
        }
    ) {
        return Transfer::Error(Error::TypeMismatch {
            expected: "heap_reference".to_string(),
            actual: format!("{pinned_value:?}"),
        });
    }

    // pin in the owning heap
    let reference = HeapReference::from_bits(pinned_value.bits() as usize);
    let heap = state.heap_mut();
    match heap.pin_heap(reference) {
        Ok(reference) => state.set_word(*value, Word::heap_reference(reference)),
        Err(HeapError::InvalidHeapReference { .. }) => {
            return Transfer::Error(Error::InvalidHeapReference);
        }
        Err(error) => return Transfer::Error(Error::from(error)),
    }

    Transfer::Continue
}

/// Execute local heap unpin.
pub(crate) fn execute_unpin(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let UnpinValue { value } = instruction.payload_as::<UnpinValue>();

    // require a local heap reference
    let pinned_value = state.get(*value);
    let value_type = match state.value_type(*value) {
        Ok(value_type) => value_type,
        Err(error) => return Transfer::Error(error),
    };
    let repr = value_layout_from_type(state.tree(), value_type);
    if !matches!(
        repr,
        ValueLayout::Pointer {
            pointer_class: PointerClass::Heap,
            ..
        }
    ) {
        return Transfer::Error(Error::TypeMismatch {
            expected: "heap_reference".to_string(),
            actual: format!("{pinned_value:?}"),
        });
    }

    // unpin in the owning heap
    let reference = HeapReference::from_bits(pinned_value.bits() as usize);
    let heap = state.heap_mut();
    match heap.unpin_heap(reference) {
        Ok(()) => {}
        Err(HeapError::InvalidHeapReference { .. }) => {
            return Transfer::Error(Error::InvalidHeapReference);
        }
        Err(error) => return Transfer::Error(Error::from(error)),
    }

    Transfer::Continue
}

/// Drop one runtime value according to its MIR type.
fn drop_value(
    state: &mut DispatchState<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    value: Word,
) -> Result<(), Error> {
    let repr = value_layout_from_type(state.tree(), ty);
    let ValueLayout::Pointer { pointer_class, .. } = repr else {
        return Err(Error::TypeMismatch {
            expected: "droppable reference".to_string(),
            actual: format!("{value:?}"),
        });
    };

    match pointer_class {
        PointerClass::Heap
        | PointerClass::SharedHeap
        | PointerClass::HeapAddress
        | PointerClass::SharedHeapAddress => Ok(()),
        PointerClass::Raw => {
            let pointer = RawPointer::from_bits(value.bits() as usize);
            let heap = state.heap_mut();
            match heap.free_raw(pointer) {
                Ok(true) => Ok(()),
                Ok(false) => Err(Error::InvalidRawPointer),
                Err(HeapError::InvalidRawPointer { .. }) => Err(Error::InvalidRawPointer),
                Err(error) => Err(Error::from(error)),
            }
        }
        PointerClass::SharedRaw => {
            let pointer = SharedRawPointer::from_bits(value.bits() as usize);
            match state.shared().free_raw(pointer) {
                Ok(true) => Ok(()),
                Ok(false) => Err(Error::InvalidSharedRawPointer),
                Err(HeapError::InvalidSharedRawPointer { .. }) => {
                    Err(Error::InvalidSharedRawPointer)
                }
                Err(error) => Err(Error::from(error)),
            }
        }
        PointerClass::Stack => {
            let pointer = StackPointer::from_address(value.bits() as usize);
            if state.owns_stack_range(pointer, 1) {
                return Ok(());
            }

            Err(Error::InvalidPointerType {
                actual: format!("{pointer:?}"),
            })
        }
        PointerClass::Frame | PointerClass::Static | PointerClass::Unknown => {
            Err(Error::InvalidPointerType {
                actual: format!("{value:?}"),
            })
        }
    }
}

/// Execute ownership end.
pub(crate) fn execute_drop(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let DropValue { value } = instruction.payload_as::<DropValue>();

    // load the dropped value
    let dropped_value = state.get(*value);
    let value_type = match state.value_type(*value) {
        Ok(value_type) => value_type,
        Err(error) => return Transfer::Error(error),
    };

    // perform the reference-specific drop work first
    if let Err(error) = drop_value(state, value_type, dropped_value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute stack allocation.
pub(crate) fn execute_allocate_stack(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let StackAlloc {
        dest,
        reference,
        allocation_type,
    } = instruction.payload_as::<StackAlloc>();

    // allocate raw stack bytes from the compiled type layout
    let layout = match state.layout(*allocation_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let address = match state.allocate_stack(layout.byte_len, layout.alignment()) {
        Ok(address) => address,
        Err(error) => return Transfer::Error(error),
    };
    let sp = StackPointer::from_address(address);
    let value = Word::stack_pointer(sp);

    // validate reference address space
    if let Err(error) = check_reference_address_space(state, *reference) {
        return Transfer::Error(error);
    }

    state.set_word(*dest, value);

    Transfer::Continue
}
