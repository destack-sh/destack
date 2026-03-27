use super::prelude::*;
use crate::telemetry::stat_inc;

/// Record one load in the VM statistics.
#[inline(always)]
fn record_load(state: &mut StepState<'_, '_>) {
    if state.collect_stats {
        stat_inc!(state.engine.statistics, loads);
    }
}

/// Record one store in the VM statistics.
#[inline(always)]
fn record_store(state: &mut StepState<'_, '_>) {
    if state.collect_stats {
        stat_inc!(state.engine.statistics, stores);
    }
}

/// Load one global value directly from isolate storage.
#[inline(always)]
fn load_global_value(
    state: &StepState<'_, '_>,
    global: u32,
) -> Result<(mir::LocalNodeId<mir::Global>, Value), Error> {
    let global_id = global_id(global);
    let value = state
        .globals
        .get(global_id)
        .copied()
        .ok_or(Error::UndefinedGlobal { global: global_id })?;

    Ok((global_id, value))
}

/// Step local variable load.
#[inline(always)]
pub(crate) fn step_local_get(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::LocalGet { dest, local } = &block[pc].data else {
        unreachable!()
    };

    // load local value
    let value = state.get_local_by_index(*local);

    // store value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step local variable store.
#[inline(always)]
pub(crate) fn step_local_set(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::LocalSet { local, value } = &block[pc].data else {
        unreachable!()
    };

    // load value
    let value = state.get(*value);

    // store local value
    state.set_local_by_index(*local, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step local address.
#[inline(always)]
pub(crate) fn step_local_addr(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::LocalAddr {
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
        return Transfer::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step global address.
pub(crate) fn step_global_addr(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::GlobalAddr {
        dest,
        global,
        reference,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // build global pointer
    let pointer = Value::global_pointer_with_meta(global_id(*global), 0, *reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return Transfer::Error(error);
    }

    // store result
    state.set(*dest, pointer);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step global constant load.
pub(crate) fn step_global_const(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::GlobalConst { dest, global } = &block[pc].data else {
        unreachable!()
    };

    // load global value
    let value = match load_global_value(state, *global) {
        Ok((_global_id, value)) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step fused global address + load.
#[inline(always)]
pub(crate) fn step_global_load(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::GlobalLoad { dest, global } = &block[pc].data else {
        unreachable!()
    };

    // track loads
    record_load(state);

    // load global value directly
    let value = match load_global_value(state, *global) {
        Ok((_global_id, value)) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step fused global address + store.
#[inline(always)]
pub(crate) fn step_global_store(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::GlobalStore {
        global,
        value,
        reference,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // track stores
    record_store(state);

    // load value to store
    let value = state.get(*value);

    // check mutability via reference metadata
    let global_id = global_id(*global);
    if reference.mutability() != Some(mir::Mutability::Mutable) {
        return Transfer::Error(Error::ImmutableGlobalWrite { global: global_id });
    }

    // store to global directly
    state.globals.set(global_id, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step pointer load.
pub(crate) fn step_load(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::Load {
        dest,
        pointer,
        raw_pointee,
        ..
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from pointer
    let value = match access::load_from_pointer_with_raw_pointee(state, ptr, *raw_pointee) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step pointer store.
pub(crate) fn step_store(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::Store {
        pointer,
        value,
        reference,
        raw_pointee,
        ..
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let ptr = state.get(*pointer);
    let val = state.get(*value);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return Transfer::Error(error);
    }

    // write through pointer
    if let Err(e) = access::store_to_pointer_with_raw_pointee(state, ptr, *raw_pointee, val) {
        return Transfer::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Step atomic load.
pub(crate) fn step_atomic_load(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::AtomicLoad {
        dest,
        pointer,
        raw_pointee,
        ..
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load pointer
    let pointer = state.get(*pointer);

    // execute the load
    let value = match state.execute_atomic_load_value(
        pointer,
        *raw_pointee,
        mir::MemoryOrdering::SeqCst,
        mir::AtomicScope::Device,
        mir::MemoryScope::Device,
        mir::MemorySemantics::default(),
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error.error),
    };

    // store the result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step atomic store.
pub(crate) fn step_atomic_store(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::AtomicStore {
        pointer,
        value,
        raw_pointee,
        ..
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let pointer = state.get(*pointer);
    let value = state.get(*value);

    // execute the store
    if let Err(error) = state.execute_atomic_store_value(
        pointer,
        value,
        *raw_pointee,
        mir::MemoryOrdering::SeqCst,
        mir::AtomicScope::Device,
        mir::MemoryScope::Device,
        mir::MemorySemantics::default(),
    ) {
        return Transfer::Error(error.error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Step atomic compare exchange.
pub(crate) fn step_atomic_compare_exchange(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::AtomicCompareExchange {
        dest,
        pointer,
        expected,
        new_value,
        raw_pointee,
        ..
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let pointer = state.get(*pointer);
    let expected = state.get(*expected);
    let new_value = state.get(*new_value);

    // execute the compare exchange
    let result = match state.execute_atomic_compare_exchange_value(
        pointer,
        expected,
        new_value,
        *raw_pointee,
        false,
        mir::MemoryOrdering::SeqCst,
        mir::AtomicScope::Device,
        mir::MemoryScope::Device,
        mir::MemorySemantics::default(),
    ) {
        Ok(result) => result,
        Err(error) => return Transfer::Error(error.error),
    };

    // store the result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step atomic read modify write.
pub(crate) fn step_atomic_rmw(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::AtomicRmw {
        dest,
        operator,
        pointer,
        value,
        raw_pointee,
        ..
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let pointer = state.get(*pointer);
    let value = state.get(*value);

    // execute the read modify write
    let result = match state.execute_atomic_rmw_value(
        *operator,
        pointer,
        value,
        *raw_pointee,
        mir::MemoryOrdering::SeqCst,
        mir::AtomicScope::Device,
        mir::MemoryScope::Device,
        mir::MemorySemantics::default(),
    ) {
        Ok(result) => result,
        Err(error) => return Transfer::Error(error.error),
    };

    // store the result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step atomic fence.
pub(crate) fn step_atomic_fence(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::AtomicFence = &block[pc].data else {
        unreachable!()
    };

    // execute the fence
    if let Err(error) = state.execute_atomic_fence(
        mir::MemoryOrdering::SeqCst,
        mir::AtomicScope::Device,
        mir::MemoryScope::Device,
        mir::MemorySemantics::default(),
    ) {
        return Transfer::Error(error.error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Step a synchronization barrier.
pub(crate) fn step_barrier(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::Barrier = &block[pc].data else {
        unreachable!()
    };

    // execute the barrier
    if let Err(error) = state.execute_barrier(
        mir::AtomicScope::Device,
        mir::MemoryScope::Device,
        mir::MemorySemantics::default(),
    ) {
        return Transfer::Error(error.error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Step managed reference load.
#[inline(always)]
pub(crate) fn step_load_managed(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::Load {
        dest,
        pointer,
        managed_pointee,
        raw_pointee: _,
        ..
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from managed reference
    let Some(managed_pointee) = *managed_pointee else {
        return Transfer::Error(Error::InvalidManagedReference);
    };
    let value = match access::load_from_managed_reference_typed(state, ptr, managed_pointee) {
        Ok(v) => v,
        Err(error) => return Transfer::Error(error),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step raw pointer load.
#[inline(always)]
pub(crate) fn step_load_raw(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::Load {
        dest,
        pointer,
        raw_pointee,
        ..
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from raw pointer
    let Some(raw_pointee) = *raw_pointee else {
        return Transfer::Error(Error::InvalidManagedReference);
    };
    let value = match access::load_from_raw_pointer_typed(state, ptr, raw_pointee) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step stack pointer load.
#[inline(always)]
pub(crate) fn step_load_stack(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::Load {
        dest,
        pointer,
        raw_pointee: _,
        ..
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from stack pointer
    let value = match access::load_from_stack_pointer(state, ptr) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step local pointer load.
#[inline(always)]
pub(crate) fn step_load_local(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::Load {
        dest,
        pointer,
        raw_pointee: _,
        ..
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from local pointer
    let value = match access::load_from_local_pointer(state, ptr) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step global pointer load.
#[inline(always)]
pub(crate) fn step_load_global(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::Load {
        dest,
        pointer,
        raw_pointee: _,
        ..
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from global pointer
    let value = match access::load_from_global_pointer(state, ptr) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step managed reference store.
#[inline(always)]
pub(crate) fn step_store_managed(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::Store {
        pointer,
        value,
        reference,
        managed_pointee,
        raw_pointee: _,
        ..
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let ptr = state.get(*pointer);
    let val = state.get(*value);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return Transfer::Error(error);
    }

    // write through managed reference
    let Some(managed_pointee) = *managed_pointee else {
        return Transfer::Error(Error::InvalidManagedReference);
    };
    if let Err(e) = access::store_to_managed_reference_typed(state, ptr, managed_pointee, val) {
        return Transfer::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Step raw pointer store.
#[inline(always)]
pub(crate) fn step_store_raw(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::Store {
        pointer,
        value,
        reference,
        raw_pointee,
        ..
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let ptr = state.get(*pointer);
    let val = state.get(*value);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return Transfer::Error(error);
    }

    // write through raw pointer
    let Some(raw_pointee) = *raw_pointee else {
        return Transfer::Error(Error::InvalidManagedReference);
    };
    if let Err(e) = access::store_to_raw_pointer_typed(state, ptr, raw_pointee, val) {
        return Transfer::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Step stack pointer store.
#[inline(always)]
pub(crate) fn step_store_stack(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::Store {
        pointer,
        value,
        reference,
        raw_pointee: _,
        ..
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let ptr = state.get(*pointer);
    let val = state.get(*value);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return Transfer::Error(error);
    }

    // write through stack pointer
    if let Err(e) = access::store_to_stack_pointer(state, ptr, val) {
        return Transfer::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Step local pointer store.
#[inline(always)]
pub(crate) fn step_store_local(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::Store {
        pointer,
        value,
        reference,
        raw_pointee: _,
        ..
    } = &block[pc].data
    else {
        unreachable!()
    };

    // validate reference metadata
    if let Err(error) = check_reference_mutability(state, *reference) {
        return Transfer::Error(error);
    }

    // load pointer and value
    let ptr = state.get(*pointer);
    let val = state.get(*value);

    // store to local pointer
    if let Err(e) = access::store_to_local_pointer(state, ptr, val) {
        return Transfer::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Step global pointer store.
#[inline(always)]
pub(crate) fn step_store_global(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::Store {
        pointer,
        value,
        reference,
        raw_pointee: _,
        ..
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let ptr = state.get(*pointer);
    let val = state.get(*value);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return Transfer::Error(error);
    }

    // write through global pointer
    if let Err(e) = access::store_to_global_pointer(state, ptr, val) {
        return Transfer::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}
/// Step managed allocation.
pub(crate) fn step_managed_alloc(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let max_managed_allocations = state.options().limits.max_managed_allocations;

    // decode instruction data
    let InstructionData::ManagedAlloc {
        dest,
        reference,
        layout_id,
        byte_len,
        trace,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // allocate managed storage
    let handle = {
        let heap = state.heap();
        if heap.managed_allocation_count() >= max_managed_allocations {
            return Transfer::Error(Error::AllocationFailed);
        }

        heap.allocate_managed_zeroed(*byte_len as usize, trace.clone(), *layout_id)
    };
    let handle = match handle {
        Ok(handle) => handle,
        Err(error) => return Transfer::Error(Error::from(error)),
    };
    if state.collect_stats {
        state.engine.statistics.heap_allocations += 1;
    }
    let value = Value::managed_reference_with_meta(handle, *reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return Transfer::Error(error);
    }

    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step managed array allocation.
pub(crate) fn step_managed_alloc_array(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let max_managed_allocations = state.options().limits.max_managed_allocations;

    // decode instruction data
    let InstructionData::ManagedAllocArray {
        dest,
        length,
        reference,
        element_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve array length
    let len_val = state.get(*length);
    let length = len_val.as_uint().unwrap_or(0) as usize;

    // allocate managed array storage
    let handle = {
        let tree = state.tree();
        let element_byte_len = match access::managed_type_size(tree, *element_type) {
            Ok(byte_len) => byte_len,
            Err(error) => return Transfer::Error(error),
        };
        let byte_len = match length.checked_mul(element_byte_len) {
            Some(byte_len) => byte_len,
            None => return Transfer::Error(Error::AllocationFailed),
        };
        let trace = access::managed_array_reference_map(tree, *element_type, length);

        let heap = state.heap();
        if heap.managed_allocation_count() >= max_managed_allocations {
            return Transfer::Error(Error::AllocationFailed);
        }
        heap.allocate_managed_zeroed(byte_len, trace, None)
    };
    let handle = match handle {
        Ok(handle) => handle,
        Err(error) => return Transfer::Error(Error::from(error)),
    };
    if state.collect_stats {
        state.engine.statistics.heap_allocations += 1;
    }
    let value = Value::managed_reference_with_meta(handle, *reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return Transfer::Error(error);
    }

    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step raw allocation.
pub(crate) fn step_raw_alloc(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let max_raw_allocations = state.options().limits.max_raw_allocations;

    // decode instruction data
    let InstructionData::RawAlloc {
        dest,
        reference,
        byte_len,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // allocate raw heap bytes
    let ptr = {
        let heap = state.heap();
        if heap.raw_allocation_count() >= max_raw_allocations {
            return Transfer::Error(Error::AllocationFailed);
        }

        let bytes = vec![0; *byte_len as usize];
        heap.allocate_raw_bytes(&bytes)
    };
    let ptr = match ptr {
        Ok(ptr) => ptr,
        Err(error) => return Transfer::Error(Error::from(error)),
    };
    if state.collect_stats {
        state.engine.statistics.heap_allocations += 1;
    }
    let value = Value::raw_pointer_with_meta(ptr, *reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return Transfer::Error(error);
    }

    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step raw free.
pub(crate) fn step_raw_free(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::RawFree { pointer } = &block[pc].data else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // accept raw pointer values
    if let Some(p) = ptr.as_raw_pointer() {
        // report invalid handle
        let heap = state.heap();
        if !heap.free_raw(p) {
            return Transfer::Error(Error::InvalidManagedReference);
        }
    }
    // otherwise report type mismatch
    else {
        return Transfer::Error(Error::TypeMismatch {
            expected: "raw_pointer".to_string(),
            actual: format!("{ptr:?}"),
        });
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Step raw drop (compiler-inserted deallocation at ownership end).
/// Semantically equivalent to raw_free but signals ownership transfer.
pub(crate) fn step_raw_drop(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::RawDrop { value } = &block[pc].data else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*value);

    // accept raw pointer values - deallocate like raw_free
    if let Some(p) = ptr.as_raw_pointer() {
        // report invalid handle
        let heap = state.heap();
        if !heap.free_raw(p) {
            return Transfer::Error(Error::InvalidManagedReference);
        }
    }
    // otherwise report type mismatch
    else {
        return Transfer::Error(Error::TypeMismatch {
            expected: "raw_pointer".to_string(),
            actual: format!("{ptr:?}"),
        });
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Step stack allocation.
pub(crate) fn step_stack_alloc(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::StackAlloc {
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
        state.current_frame_mut().allocate_stack_buffer()
    } else {
        state
            .current_frame_mut()
            .allocate_stack_buffer_with_values(*slot_count as usize)
    };
    let sp = destack_heap::StackPointer::new(frame_index, slot);
    let value = Value::stack_pointer_with_meta(sp, *reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return Transfer::Error(error);
    }

    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step stack drop (compiler-inserted lifetime end marker).
pub(crate) fn step_stack_drop(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::StackDrop { value } = &block[pc].data else {
        unreachable!()
    };

    // load the stack pointer being retired
    let pointer = state.get(*value);

    // reject non-stack values
    let Some(pointer) = pointer.as_stack_pointer() else {
        return Transfer::Error(Error::TypeMismatch {
            expected: "stack_pointer".to_string(),
            actual: format!("{pointer:?}"),
        });
    };

    // reject cross-frame access
    if pointer.frame_idx != state.frame_index {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{pointer:?}"),
        });
    }

    // retire the stack buffer
    if !state.current_frame_mut().retire_stack_buffer(pointer.slot) {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{pointer:?}"),
        });
    }

    next!(state, block, pc)
}
