use super::*;

/// Handle local variable load.
#[inline(always)]
pub(crate) fn handle_local_get(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::LocalGet { dest, local } = &block[pc].data else {
        unreachable!()
    };

    // load local value
    let value = state.get_local_by_index(*local);

    // store value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle local variable store.
#[inline(always)]
pub(crate) fn handle_local_set(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::LocalSet { local, value } = &block[pc].data else {
        unreachable!()
    };

    // load value
    let val = state.get(*value);

    // store local value
    state.set_local_by_index(*local, val);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle local address.
#[inline(always)]
pub(crate) fn handle_local_addr(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::LocalAddr {
        dest,
        local,
        reference,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // build local pointer
    let pointer = LocalPointer::new(state.frame_index, *local as usize);
    let value = Value::local_pointer_with_meta(pointer, *reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle global address.
pub(crate) fn handle_global_addr(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::GlobalAddr {
        dest,
        global,
        reference,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // write global pointer
    let ptr = Value::global_pointer_with_meta(global_id(*global), 0, *reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return ControlFlow::Error(error);
    }

    state.set(*dest, ptr);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle global constant load.
pub(crate) fn handle_global_const(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::GlobalConst { dest, global } = &block[pc].data else {
        unreachable!()
    };

    // load global value
    let global_id = global_id(*global);
    let value = match state.interpreter.isolate.globals.get(global_id).copied() {
        Some(v) => v,
        None => return ControlFlow::Error(Error::UndefinedGlobal { global: global_id }),
    };

    // store value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle fused global address + load.
#[inline(always)]
pub(crate) fn handle_global_load(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::GlobalLoad { dest, global } = &block[pc].data else {
        unreachable!()
    };

    // track loads
    if state.collect_stats {
        stat_inc!(state.interpreter.engine.statistics, loads);
    }

    // load global value directly
    let global_id = global_id(*global);
    let value = match state.interpreter.isolate.globals.get(global_id).copied() {
        Some(v) => v,
        None => return ControlFlow::Error(Error::UndefinedGlobal { global: global_id }),
    };

    // store value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle fused global address + store.
#[inline(always)]
pub(crate) fn handle_global_store(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::GlobalStore {
        global,
        value,
        reference,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // track stores
    if state.collect_stats {
        stat_inc!(state.interpreter.engine.statistics, stores);
    }

    // load value to store
    let val = state.get(*value);

    // check mutability via reference metadata
    let global_id = global_id(*global);
    if reference.mutability() != Some(mir::Mutability::Mutable) {
        return ControlFlow::Error(Error::ImmutableGlobalWrite { global: global_id });
    }

    // store to global directly
    state.interpreter.isolate.globals.set(global_id, val);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle pointer load.
pub(crate) fn handle_load(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Load { dest, pointer } = &block[pc].data else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from pointer
    let value = match instruction::load_from_pointer(state, ptr) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle pointer store.
pub(crate) fn handle_store(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Store {
        pointer,
        value,
        reference,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let ptr = state.get(*pointer);
    let val = state.get(*value);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return ControlFlow::Error(error);
    }

    // write through pointer
    if let Err(e) = instruction::store_to_pointer(state, ptr, val) {
        return ControlFlow::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle managed pointer load.
#[inline(always)]
pub(crate) fn handle_load_managed(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Load { dest, pointer } = &block[pc].data else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from managed pointer
    let value = match instruction::load_from_managed_reference(state, ptr) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle raw pointer load.
#[inline(always)]
pub(crate) fn handle_load_raw(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Load { dest, pointer } = &block[pc].data else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from raw pointer
    let value = match instruction::load_from_raw_pointer(state, ptr) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle stack pointer load.
#[inline(always)]
pub(crate) fn handle_load_stack(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Load { dest, pointer } = &block[pc].data else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from stack pointer
    let value = match instruction::load_from_stack_pointer(state, ptr) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle local pointer load.
#[inline(always)]
pub(crate) fn handle_load_local(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Load { dest, pointer } = &block[pc].data else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from local pointer
    let value = match instruction::load_from_local_pointer(state, ptr) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle global pointer load.
#[inline(always)]
pub(crate) fn handle_load_global(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Load { dest, pointer } = &block[pc].data else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from global pointer
    let value = match instruction::load_from_global_pointer(state, ptr) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle managed pointer store.
#[inline(always)]
pub(crate) fn handle_store_managed(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Store {
        pointer,
        value,
        reference,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let ptr = state.get(*pointer);
    let val = state.get(*value);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return ControlFlow::Error(error);
    }

    // write through managed pointer
    if let Err(e) = instruction::store_to_managed_reference(state, ptr, val) {
        return ControlFlow::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle raw pointer store.
#[inline(always)]
pub(crate) fn handle_store_raw(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Store {
        pointer,
        value,
        reference,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let ptr = state.get(*pointer);
    let val = state.get(*value);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return ControlFlow::Error(error);
    }

    // write through raw pointer
    if let Err(e) = instruction::store_to_raw_pointer(state, ptr, val) {
        return ControlFlow::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle stack pointer store.
#[inline(always)]
pub(crate) fn handle_store_stack(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Store {
        pointer,
        value,
        reference,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let ptr = state.get(*pointer);
    let val = state.get(*value);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return ControlFlow::Error(error);
    }

    // write through stack pointer
    if let Err(e) = instruction::store_to_stack_pointer(state, ptr, val) {
        return ControlFlow::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle local pointer store.
#[inline(always)]
pub(crate) fn handle_store_local(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Store {
        pointer,
        value,
        reference,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // validate reference metadata
    if let Err(error) = check_reference_mutability(state, *reference) {
        return ControlFlow::Error(error);
    }

    // load pointer and value
    let ptr = state.get(*pointer);
    let val = state.get(*value);

    // store to local pointer
    if let Err(e) = instruction::store_to_local_pointer(state, ptr, val) {
        return ControlFlow::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle global pointer store.
#[inline(always)]
pub(crate) fn handle_store_global(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Store {
        pointer,
        value,
        reference,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let ptr = state.get(*pointer);
    let val = state.get(*value);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return ControlFlow::Error(error);
    }

    // write through global pointer
    if let Err(e) = instruction::store_to_global_pointer(state, ptr, val) {
        return ControlFlow::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}
/// Handle managed allocation.
pub(crate) fn handle_managed_alloc(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let max_heap_cells = state.interpreter.isolate.options.limits.max_heap_cells;

    // decode instruction data
    let ThreadedInstructionData::ManagedAlloc {
        dest,
        reference,
        slot_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // allocate heap cell
    let handle = {
        let heap = state.heap();
        if heap.managed.cell_count() >= max_heap_cells {
            return ControlFlow::Error(Error::AllocationFailed);
        }

        if *slot_count == UNKNOWN_SLOT_COUNT {
            heap.managed.allocate()
        } else {
            heap.managed.allocate_with_slots(*slot_count as usize)
        }
    };
    if state.collect_stats {
        state.interpreter.engine.statistics.heap_allocations += 1;
    }
    let value = Value::managed_reference_with_meta(handle, *reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle managed array allocation.
pub(crate) fn handle_managed_alloc_array(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let max_heap_cells = state.interpreter.isolate.options.limits.max_heap_cells;

    // decode instruction data
    let ThreadedInstructionData::ManagedAllocArray {
        dest,
        length,
        reference,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve array length
    let len_val = state.get(*length);
    let length = len_val.as_uint().unwrap_or(0) as usize;

    // allocate heap cell with slots
    let handle = {
        let heap = state.heap();
        if heap.managed.cell_count() >= max_heap_cells {
            return ControlFlow::Error(Error::AllocationFailed);
        }

        heap.managed.allocate_with_slots(length)
    };
    if state.collect_stats {
        state.interpreter.engine.statistics.heap_allocations += 1;
    }
    let value = Value::managed_reference_with_meta(handle, *reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle raw allocation.
pub(crate) fn handle_raw_alloc(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let max_raw_cells = state.interpreter.isolate.options.limits.max_raw_cells;

    // decode instruction data
    let ThreadedInstructionData::RawAlloc {
        dest,
        reference,
        slot_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // allocate raw heap cell
    let ptr = {
        let heap = state.heap();
        if heap.raw.cell_count() >= max_raw_cells {
            return ControlFlow::Error(Error::AllocationFailed);
        }

        if *slot_count == UNKNOWN_SLOT_COUNT {
            heap.raw.allocate()
        } else {
            heap.raw.allocate_with_slots(*slot_count as usize)
        }
    };
    if state.collect_stats {
        state.interpreter.engine.statistics.heap_allocations += 1;
    }
    let value = Value::raw_pointer_with_meta(ptr, *reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle raw free.
pub(crate) fn handle_raw_free(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::RawFree { pointer } = &block[pc].data else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // accept raw pointer values
    if let Some(p) = ptr.as_raw_pointer() {
        // report invalid handle
        let heap = state.heap();
        if !heap.raw.free(p) {
            return ControlFlow::Error(Error::InvalidHeapHandle);
        }
    }
    // otherwise report type mismatch
    else {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "raw_pointer".to_string(),
            actual: format!("{ptr:?}"),
        });
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle raw drop (compiler-inserted deallocation at ownership end).
/// Semantically equivalent to raw_free but signals ownership transfer.
pub(crate) fn handle_raw_drop(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::RawDrop { value } = &block[pc].data else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*value);

    // accept raw pointer values - deallocate like raw_free
    if let Some(p) = ptr.as_raw_pointer() {
        // report invalid handle
        let heap = state.heap();
        if !heap.raw.free(p) {
            return ControlFlow::Error(Error::InvalidHeapHandle);
        }
    }
    // otherwise report type mismatch
    else {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "raw_pointer".to_string(),
            actual: format!("{ptr:?}"),
        });
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle stack allocation.
pub(crate) fn handle_stack_alloc(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::StackAlloc {
        dest,
        reference,
        slot_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // NOTE #Broken: stack allocation requires proper layout sizing
    let frame_index = state.frame_index;
    let slot = if *slot_count == UNKNOWN_SLOT_COUNT {
        state.current_frame_mut().allocate_stack_cell()
    } else {
        state
            .current_frame_mut()
            .allocate_stack_cell_with_slots(*slot_count as usize)
    };
    let sp = crate::memory::StackPointer::new(frame_index, slot);
    let value = Value::stack_pointer_with_meta(sp, *reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle stack drop (compiler-inserted lifetime end marker).
/// Currently a no-op - stack memory is freed when the frame exits.
/// Exists for NLL support and potential future optimizations.
pub(crate) fn handle_stack_drop(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data - validate it's the right instruction
    let ThreadedInstructionData::StackDrop { value: _ } = &block[pc].data else {
        unreachable!()
    };

    // no-op: stack memory is managed by frame lifetime
    // the instruction exists to mark the end of the value's lifetime for NLL
    // continue to next instruction
    next!(state, block, pc)
}
