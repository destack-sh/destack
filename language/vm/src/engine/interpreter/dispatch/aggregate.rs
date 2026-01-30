use super::*;

/// Handle field get.
pub(crate) fn handle_field_get(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldGet {
        dest,
        aggregate,
        index,
        field_count: _,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);

    // load field value
    let value = match instruction::get_field(state, agg, *index) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field get for small inline aggregates (≤2 fields).
#[inline(always)]
pub(crate) fn handle_field_get_inline(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldGet {
        dest,
        aggregate,
        index,
        field_count: _,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate and extract heap handle
    let agg = state.get(*aggregate);
    let handle = match agg.as_heap_handle() {
        Some(h) => h,
        None => return ControlFlow::Error(crate::diagnostic::Error::InvalidHeapHandle),
    };

    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return ControlFlow::Error(crate::diagnostic::Error::NullPointerDereference);
    }

    // fast path: directly access heap cell and inline slots
    let value = unsafe {
        let heap = state.heap_ref();
        let cell = heap.managed.get_unchecked(handle);
        let slot_index = handle.slot_index().wrapping_add(*index as usize);
        // inline storage is guaranteed for field_count ≤ 2
        *cell.slots.get_unchecked(slot_index)
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field store on small managed aggregates (≤2 fields, inline storage).
#[inline(always)]
pub(crate) fn handle_field_store_inline(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldStore {
        aggregate,
        index,
        value,
        reference: _,
        field_count: _,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // track stores
    if state.collect_stats {
        stat_inc!(state.interpreter.engine.statistics, stores);
    }

    // load aggregate and extract heap handle
    let agg = state.get(*aggregate);
    let handle = match agg.as_heap_handle() {
        Some(h) => h,
        None => return ControlFlow::Error(crate::diagnostic::Error::InvalidHeapHandle),
    };

    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return ControlFlow::Error(crate::diagnostic::Error::NullPointerDereference);
    }

    // load value to store
    let val = state.get(*value);

    // fast path: directly access heap cell and inline slots
    unsafe {
        let heap = state.heap();
        let cell = heap.managed.get_unchecked_mut(handle);
        let slot_index = handle.slot_index().wrapping_add(*index as usize);
        // inline storage is guaranteed for field_count ≤ 2
        *cell.slots.get_unchecked_mut(slot_index) = val;
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field addr.
pub(crate) fn handle_field_addr(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldAddr {
        dest,
        aggregate,
        index,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);

    // compute field address
    let value = match instruction::field_addr(state, agg, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field addr on aggregate values.
pub(crate) fn handle_field_addr_aggregate(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldAddr {
        dest,
        aggregate,
        index,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if !matches!(agg.tag(), ValueTag::Aggregate | ValueTag::String) {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "aggregate".to_string(),
            actual: format!("{agg:?}"),
        });
    }

    // compute field address
    let handle = agg.as_heap_handle().unwrap();
    let value = match instruction::field_addr_managed(state, handle, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field addr on managed references.
pub(crate) fn handle_field_addr_managed(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldAddr {
        dest,
        aggregate,
        index,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::ManagedReference {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // compute field address
    let handle = agg.as_heap_handle().unwrap();
    let value = match instruction::field_addr_managed(state, handle, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field addr on raw pointers.
pub(crate) fn handle_field_addr_raw(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldAddr {
        dest,
        aggregate,
        index,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::RawPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // compute field address
    let pointer = agg.as_raw_pointer().unwrap();
    let value = match instruction::field_addr_raw(state, pointer, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field addr on stack pointers.
pub(crate) fn handle_field_addr_stack(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldAddr {
        dest,
        aggregate,
        index,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::StackPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // compute field address
    let pointer = agg.as_stack_pointer().unwrap();
    let value = match instruction::field_addr_stack(state, pointer, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field addr on global pointers.
pub(crate) fn handle_field_addr_global(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldAddr {
        dest,
        aggregate,
        index,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::GlobalPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // compute field address
    let pointer = agg.as_global_pointer().unwrap();
    let value = match instruction::field_addr_global(state, pointer, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field load.
pub(crate) fn handle_field_load(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldLoad {
        dest,
        aggregate,
        index,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);

    // compute field address
    let pointer = match instruction::field_addr(state, agg, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // load value
    let value = match instruction::load_from_pointer(state, pointer) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field load on aggregate values.
pub(crate) fn handle_field_load_aggregate(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldLoad {
        dest,
        aggregate,
        index,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if !matches!(agg.tag(), ValueTag::Aggregate | ValueTag::String) {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "aggregate".to_string(),
            actual: format!("{agg:?}"),
        });
    }

    // load field value
    let handle = agg.as_heap_handle().unwrap();
    let value = match instruction::load_field_managed(state, handle, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field load on managed references.
pub(crate) fn handle_field_load_managed(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldLoad {
        dest,
        aggregate,
        index,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::ManagedReference {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // load field value
    let handle = agg.as_heap_handle().unwrap();
    let value = match instruction::load_field_managed(state, handle, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field load on raw pointers.
pub(crate) fn handle_field_load_raw(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldLoad {
        dest,
        aggregate,
        index,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::RawPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // load field value
    let pointer = agg.as_raw_pointer().unwrap();
    let value = match instruction::load_field_raw(state, pointer, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field load on stack pointers.
pub(crate) fn handle_field_load_stack(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldLoad {
        dest,
        aggregate,
        index,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::StackPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // load field value
    let pointer = agg.as_stack_pointer().unwrap();
    let value = match instruction::load_field_stack(state, pointer, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field load on global pointers.
pub(crate) fn handle_field_load_global(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldLoad {
        dest,
        aggregate,
        index,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::GlobalPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // load field value
    let pointer = agg.as_global_pointer().unwrap();
    let value = match instruction::load_field_global(state, pointer, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field set.
pub(crate) fn handle_field_set(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldSet {
        dest,
        aggregate,
        index,
        value,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let agg = state.get(*aggregate);
    let val = state.get(*value);

    // write field
    let result = match instruction::set_field(state, agg, *index, val) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field store.
pub(crate) fn handle_field_store(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldStore {
        aggregate,
        index,
        value,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let agg = state.get(*aggregate);
    let val = state.get(*value);

    // compute field address
    let pointer = match instruction::field_addr(state, agg, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    let pointer = pointer.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) = instruction::store_to_pointer(state, pointer, val) {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field store on aggregate values.
pub(crate) fn handle_field_store_aggregate(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldStore {
        aggregate,
        index,
        value,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let agg = state.get(*aggregate);
    if !matches!(agg.tag(), ValueTag::Aggregate | ValueTag::String) {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "aggregate".to_string(),
            actual: format!("{agg:?}"),
        });
    }
    let val = state.get(*value);

    // validate reference kind
    let handle = agg.as_heap_handle().unwrap();
    let pointer = Value::managed_reference_with_meta(handle, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) = instruction::store_field_managed(state, handle, *index, *field_count, val) {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field store on managed references.
pub(crate) fn handle_field_store_managed(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldStore {
        aggregate,
        index,
        value,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::ManagedReference {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }
    let val = state.get(*value);

    // validate reference kind
    let handle = agg.as_heap_handle().unwrap();
    let pointer = Value::managed_reference_with_meta(handle, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) = instruction::store_field_managed(state, handle, *index, *field_count, val) {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field store on raw pointers.
pub(crate) fn handle_field_store_raw(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldStore {
        aggregate,
        index,
        value,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::RawPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }
    let val = state.get(*value);

    // validate reference kind
    let raw_pointer = agg.as_raw_pointer().unwrap();
    let pointer = Value::raw_pointer_with_meta(raw_pointer, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) = instruction::store_field_raw(state, raw_pointer, *index, *field_count, val)
    {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field store on stack pointers.
pub(crate) fn handle_field_store_stack(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldStore {
        aggregate,
        index,
        value,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::StackPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }
    let val = state.get(*value);

    // validate reference semantics
    let stack_pointer = agg.as_stack_pointer().unwrap();
    let pointer = Value::stack_pointer_with_meta(stack_pointer, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) =
        instruction::store_field_stack(state, stack_pointer, *index, *field_count, val)
    {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field store on global pointers.
pub(crate) fn handle_field_store_global(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldStore {
        aggregate,
        index,
        value,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::GlobalPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }
    let val = state.get(*value);

    // validate reference semantics
    let global_pointer = agg.as_global_pointer().unwrap();
    let pointer =
        Value::global_pointer_with_meta(global_pointer.id, global_pointer.slot_offset, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) =
        instruction::store_field_global(state, global_pointer, *index, *field_count, val)
    {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element get.
pub(crate) fn handle_element_get(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementGet { dest, array, index } = &block[pc].data else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // load element value
    let value = match instruction::get_element(state, arr, idx_val) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element addr.
pub(crate) fn handle_element_addr(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // compute element address
    let value = match instruction::element_addr(state, arr, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element addr on aggregate values.
pub(crate) fn handle_element_addr_aggregate(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::Aggregate {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "array".to_string(),
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // compute element address
    let handle = arr.as_heap_handle().unwrap();
    let value = match instruction::element_addr_managed(state, handle, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element addr on managed references.
pub(crate) fn handle_element_addr_managed(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::ManagedReference {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // compute element address
    let handle = arr.as_heap_handle().unwrap();
    let value = match instruction::element_addr_managed(state, handle, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element addr on raw pointers.
pub(crate) fn handle_element_addr_raw(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::RawPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // compute element address
    let pointer = arr.as_raw_pointer().unwrap();
    let value = match instruction::element_addr_raw(state, pointer, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element addr on stack pointers.
pub(crate) fn handle_element_addr_stack(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::StackPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // compute element address
    let pointer = arr.as_stack_pointer().unwrap();
    let value = match instruction::element_addr_stack(state, pointer, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element addr on global pointers.
pub(crate) fn handle_element_addr_global(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::GlobalPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // compute element address
    let pointer = arr.as_global_pointer().unwrap();
    let value = match instruction::element_addr_global(state, pointer, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element load.
pub(crate) fn handle_element_load(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementLoad {
        dest,
        array,
        index,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // compute element address
    let pointer = match instruction::element_addr(state, arr, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // load value
    let value = match instruction::load_from_pointer(state, pointer) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element load on aggregate values.
pub(crate) fn handle_element_load_aggregate(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementLoad {
        dest,
        array,
        index,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::Aggregate {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "array".to_string(),
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // load element value
    let handle = arr.as_heap_handle().unwrap();
    let value = match instruction::load_element_managed(state, handle, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element load on managed references.
pub(crate) fn handle_element_load_managed(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementLoad {
        dest,
        array,
        index,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::ManagedReference {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // load element value
    let handle = arr.as_heap_handle().unwrap();
    let value = match instruction::load_element_managed(state, handle, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element load on raw pointers.
pub(crate) fn handle_element_load_raw(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementLoad {
        dest,
        array,
        index,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::RawPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // load element value
    let pointer = arr.as_raw_pointer().unwrap();
    let value = match instruction::load_element_raw(state, pointer, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element load on stack pointers.
pub(crate) fn handle_element_load_stack(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementLoad {
        dest,
        array,
        index,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::StackPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // load element value
    let pointer = arr.as_stack_pointer().unwrap();
    let value = match instruction::load_element_stack(state, pointer, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element load on global pointers.
pub(crate) fn handle_element_load_global(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementLoad {
        dest,
        array,
        index,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::GlobalPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // load element value
    let pointer = arr.as_global_pointer().unwrap();
    let value = match instruction::load_element_global(state, pointer, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element set.
pub(crate) fn handle_element_set(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementSet {
        dest,
        array,
        index,
        value,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let arr = state.get(*array);
    let idx = state.get(*index);
    let val = state.get(*value);
    let idx_val = idx.as_uint().unwrap_or(0);

    // write element
    let result = match instruction::set_element(state, arr, idx_val, val) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element store.
pub(crate) fn handle_element_store(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let arr = state.get(*array);
    let idx = state.get(*index);
    let val = state.get(*value);
    let idx_val = idx.as_uint().unwrap_or(0);

    // compute element address
    let pointer = match instruction::element_addr(state, arr, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    let pointer = pointer.with_reference_meta(*reference);

    // validate reference semantics
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) = instruction::store_to_pointer(state, pointer, val) {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element store on aggregate values.
pub(crate) fn handle_element_store_aggregate(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let arr = state.get(*array);
    if arr.tag() != ValueTag::Aggregate {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "array".to_string(),
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let val = state.get(*value);
    let idx_val = idx.as_uint().unwrap_or(0);

    // validate reference semantics
    let handle = arr.as_heap_handle().unwrap();
    let pointer = Value::managed_reference_with_meta(handle, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) =
        instruction::store_element_managed(state, handle, idx_val, *array_length, val)
    {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

// vector operations
/// Handle element store on managed references.
pub(crate) fn handle_element_store_managed(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let arr = state.get(*array);
    if arr.tag() != ValueTag::ManagedReference {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let val = state.get(*value);
    let idx_val = idx.as_uint().unwrap_or(0);

    // validate reference semantics
    let handle = arr.as_heap_handle().unwrap();
    let pointer = Value::managed_reference_with_meta(handle, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) =
        instruction::store_element_managed(state, handle, idx_val, *array_length, val)
    {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element store on raw pointers.
pub(crate) fn handle_element_store_raw(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let arr = state.get(*array);
    if arr.tag() != ValueTag::RawPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let val = state.get(*value);
    let idx_val = idx.as_uint().unwrap_or(0);

    // validate reference semantics
    let raw_pointer = arr.as_raw_pointer().unwrap();
    let pointer = Value::raw_pointer_with_meta(raw_pointer, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) =
        instruction::store_element_raw(state, raw_pointer, idx_val, *array_length, val)
    {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element store on stack pointers.
pub(crate) fn handle_element_store_stack(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let arr = state.get(*array);
    if arr.tag() != ValueTag::StackPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let val = state.get(*value);
    let idx_val = idx.as_uint().unwrap_or(0);

    // validate reference semantics
    let stack_pointer = arr.as_stack_pointer().unwrap();
    let pointer = Value::stack_pointer_with_meta(stack_pointer, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) =
        instruction::store_element_stack(state, stack_pointer, idx_val, *array_length, val)
    {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element store on global pointers.
pub(crate) fn handle_element_store_global(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let arr = state.get(*array);
    if arr.tag() != ValueTag::GlobalPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let val = state.get(*value);
    let idx_val = idx.as_uint().unwrap_or(0);

    // validate reference semantics
    let global_pointer = arr.as_global_pointer().unwrap();
    let pointer =
        Value::global_pointer_with_meta(global_pointer.id, global_pointer.slot_offset, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) =
        instruction::store_element_global(state, global_pointer, idx_val, *array_length, val)
    {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle aggregate construction.
pub(crate) fn handle_aggregate(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Aggregate { dest, elements } = &block[pc].data else {
        unreachable!()
    };

    // resolve element values from the argument pool
    let element_slice = state.argument_slice(*elements);
    let result = match element_slice {
        // empty aggregate
        [] => state.allocate_aggregate(Vec::new()),
        // single element aggregate
        [first] => state.allocate_single(state.get(*first)),
        // pair aggregate fast path
        [first, second] => state.allocate_pair(state.get(*first), state.get(*second)),
        // general aggregate
        _ => {
            // collect element values into a vec
            let mut element_values = Vec::with_capacity(element_slice.len());
            for value in element_slice {
                element_values.push(state.get(*value));
            }

            // allocate the aggregate on the heap
            state.allocate_aggregate(element_values)
        }
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}
