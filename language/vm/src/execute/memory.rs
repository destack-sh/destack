use super::prelude::*;
use crate::telemetry::stat_inc;
use destack_heap::{HeapError, Payload};

/// Record one load in the VM statistics.
#[inline(always)]
fn record_load(state: &mut ExecutionState<'_, '_>) {
    if state.collect_stats {
        stat_inc!(state.engine.statistics, loads);
    }
}

/// Record one store in the VM statistics.
#[inline(always)]
fn record_store(state: &mut ExecutionState<'_, '_>) {
    if state.collect_stats {
        stat_inc!(state.engine.statistics, stores);
    }
}

/// Load one heap array length operand as a host usize.
#[inline(always)]
fn load_heap_array_length(
    state: &ExecutionState<'_, '_>,
    value: mir::Value,
) -> Result<usize, Error> {
    let value = state.get(value);
    let length = value.as_uint().ok_or_else(|| Error::TypeMismatch {
        expected: "unsigned integer".to_string(),
        actual: format!("{value:?}"),
    })?;

    usize::try_from(length).map_err(|_| Error::AllocationFailed)
}

/// Load one static value directly from isolate storage.
#[inline(always)]
fn load_static_value(
    state: &ExecutionState<'_, '_>,
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

/// Execute local variable load.
#[inline(always)]
pub(crate) fn execute_local_get(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::LocalGet { dest, local } = &block[pc].immediate else {
        unreachable!()
    };

    // load local value
    let value = state.get_local_by_index(*local);

    // store value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute local variable store.
#[inline(always)]
pub(crate) fn execute_local_set(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::LocalSet { local, value } = &block[pc].immediate else {
        unreachable!()
    };

    // load value
    let value = state.get(*value);

    // store local value
    state.set_local_by_index(*local, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute local address.
#[inline(always)]
pub(crate) fn execute_local_addr(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::LocalAddr {
        dest,
        local,
        reference,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // build frame pointer
    let pointer = FramePointer::new(state.frame_index, *local as usize);
    let value = match frame_pointer_value_with_meta(pointer, *reference) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return Transfer::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute static address.
pub(crate) fn execute_static_addr(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::StaticAddr {
        dest,
        global,
        reference,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // build static pointer
    let pointer = match static_pointer_value_with_meta(global_id(*global), 0, *reference) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return Transfer::Error(error);
    }

    // store result
    state.set(*dest, pointer);

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute fused static address and load.
#[inline(always)]
pub(crate) fn execute_static_load(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::StaticLoad { dest, global } = &block[pc].immediate else {
        unreachable!()
    };

    // track loads
    record_load(state);

    // load static value directly
    let value = match load_static_value(state, *global) {
        Ok((_global_id, value)) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute fused static address and store.
#[inline(always)]
pub(crate) fn execute_static_store(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::StaticStore {
        global,
        value,
        reference,
    } = &block[pc].immediate
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

    // store to static directly
    state.globals.set(global_id, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute pointer load.
pub(crate) fn execute_load(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Load {
        dest,
        pointer,
        access,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from pointer
    let value = match access::load_from_pointer_with_access(state, ptr, *access) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute pointer store.
pub(crate) fn execute_store(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Store {
        pointer,
        value,
        reference,
        access,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load values
    let ptr = state.get(*pointer);
    let val = state.get(*value);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return Transfer::Error(error);
    }

    // write through pointer
    if let Err(e) = access::store_to_pointer_with_access(state, ptr, *access, val) {
        return Transfer::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute atomic load.
pub(crate) fn execute_atomic_load(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::AtomicLoad {
        dest,
        pointer,
        raw_pointee,
        ..
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load pointer
    let pointer = state.get(*pointer);

    // execute the load
    let value = match state.execute_atomic_load_value(
        pointer,
        *raw_pointee,
        mir::MemoryOrdering::SequentiallyConsistent,
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

/// Execute atomic store.
pub(crate) fn execute_atomic_store(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::AtomicStore {
        pointer,
        value,
        raw_pointee,
        ..
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load values
    let pointer = state.get(*pointer);
    let value = state.get(*value);

    // execute the store
    if let Err(error) = state.execute_atomic_store_value(
        pointer,
        value,
        *raw_pointee,
        mir::MemoryOrdering::SequentiallyConsistent,
        mir::AtomicScope::Device,
        mir::MemoryScope::Device,
        mir::MemorySemantics::default(),
    ) {
        return Transfer::Error(error.error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute atomic compare exchange.
pub(crate) fn execute_atomic_compare_exchange(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::AtomicCompareExchange {
        dest,
        pointer,
        expected,
        new_value,
        raw_pointee,
        ..
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load values
    let pointer = state.get(*pointer);
    let expected = state.get(*expected);
    let new_value = state.get(*new_value);

    // execute the compare exchange
    let result = match state.execute_atomic_compare_exchange_value(
        *dest,
        pointer,
        expected,
        new_value,
        *raw_pointee,
        false,
        mir::MemoryOrdering::SequentiallyConsistent,
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

/// Execute atomic read modify write.
pub(crate) fn execute_atomic_rmw(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::AtomicRmw {
        dest,
        operator,
        pointer,
        value,
        raw_pointee,
        ..
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load values
    let pointer = state.get(*pointer);
    let value = state.get(*value);

    // execute the read modify write
    let result = match state.execute_atomic_rmw_value(
        *operator,
        pointer,
        value,
        *raw_pointee,
        mir::MemoryOrdering::SequentiallyConsistent,
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

/// Execute atomic fence.
pub(crate) fn execute_atomic_fence(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::AtomicFence = &block[pc].immediate else {
        unreachable!()
    };

    // execute the fence
    if let Err(error) = state.execute_atomic_fence(
        mir::MemoryOrdering::SequentiallyConsistent,
        mir::AtomicScope::Device,
        mir::MemoryScope::Device,
        mir::MemorySemantics::default(),
    ) {
        return Transfer::Error(error.error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute one execution and memory synchronization barrier.
pub(crate) fn execute_barrier(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Barrier = &block[pc].immediate else {
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

/// Execute heap reference load.
#[inline(always)]
pub(crate) fn execute_load_heap(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Load {
        dest,
        pointer,
        access,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from heap reference
    let Some(access) = *access else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let value = match access::load_from_heap_reference_typed(state, ptr, access) {
        Ok(v) => v,
        Err(error) => return Transfer::Error(error),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute raw pointer load.
#[inline(always)]
pub(crate) fn execute_load_raw(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Load {
        dest,
        pointer,
        access,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from raw pointer
    let Some(access) = *access else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let value = match access::load_from_raw_pointer_typed(state, ptr, access) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute stack pointer load.
#[inline(always)]
pub(crate) fn execute_load_stack(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Load {
        dest,
        pointer,
        access,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from stack pointer
    let Some(access) = *access else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let pointer = match ptr.as_stack_pointer() {
        Some(pointer) => pointer,
        None => {
            return Transfer::Error(Error::InvalidPointerType {
                actual: format!("{ptr:?}"),
            });
        }
    };
    let value = match access::load_from_stack_pointer_typed(state, pointer, access) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute frame pointer load.
#[inline(always)]
pub(crate) fn execute_load_frame(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Load {
        dest,
        pointer,
        access: _,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from frame pointer
    let value = match access::load_from_frame_pointer(state, ptr) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute static pointer load.
#[inline(always)]
pub(crate) fn execute_load_static(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Load {
        dest,
        pointer,
        access: _,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from static pointer
    let value = match access::load_from_static_pointer(state, ptr) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute heap reference store.
#[inline(always)]
pub(crate) fn execute_store_heap(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Store {
        pointer,
        value,
        reference,
        access,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load values
    let ptr = state.get(*pointer);
    let val = state.get(*value);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return Transfer::Error(error);
    }

    // write through heap reference
    let Some(access) = *access else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    if let Err(e) = access::store_to_heap_reference_typed(state, ptr, access, val) {
        return Transfer::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute raw pointer store.
#[inline(always)]
pub(crate) fn execute_store_raw(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Store {
        pointer,
        value,
        reference,
        access,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load values
    let ptr = state.get(*pointer);
    let val = state.get(*value);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return Transfer::Error(error);
    }

    // write through raw pointer
    let Some(access) = *access else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    if let Err(e) = access::store_to_raw_pointer_typed(state, ptr, access, val) {
        return Transfer::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute stack pointer store.
#[inline(always)]
pub(crate) fn execute_store_stack(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Store {
        pointer,
        value,
        reference,
        access,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load values
    let ptr = state.get(*pointer);
    let val = state.get(*value);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return Transfer::Error(error);
    }

    // write through stack pointer
    let Some(access) = *access else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let pointer = match ptr.as_stack_pointer() {
        Some(pointer) => pointer,
        None => {
            return Transfer::Error(Error::InvalidPointerType {
                actual: format!("{ptr:?}"),
            });
        }
    };
    if let Err(e) = access::store_to_stack_pointer_typed(state, pointer, access, val) {
        return Transfer::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute frame pointer store.
#[inline(always)]
pub(crate) fn execute_store_frame(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Store {
        pointer,
        value,
        reference,
        access: _,
    } = &block[pc].immediate
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

    // store to frame pointer
    if let Err(e) = access::store_to_frame_pointer(state, ptr, val) {
        return Transfer::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute static pointer store.
#[inline(always)]
pub(crate) fn execute_store_static(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Store {
        pointer,
        value,
        reference,
        access: _,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load values
    let ptr = state.get(*pointer);
    let val = state.get(*value);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return Transfer::Error(error);
    }

    // write through static pointer
    if let Err(e) = access::store_to_static_pointer(state, ptr, val) {
        return Transfer::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute typed allocation.
pub(crate) fn execute_new(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let max_heap_allocations = state.options().limits.max_heap_allocations;

    // decode instruction immediate
    let Immediate::New {
        dest,
        reference,
        storage_type,
        layout_id,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // resolve the exact managed layout id first
    let layout_id = layout_id.or_else(|| state.module.layout_id_for_type(*storage_type));
    let Some(layout_id) = layout_id else {
        return Transfer::Error(Error::InvalidInstruction);
    };
    // allocate heap storage
    let heap_reference = {
        if state.heap().heap_allocation_count() >= max_heap_allocations {
            return Transfer::Error(Error::AllocationFailed);
        }

        state.allocate_heap_layout(layout_id, Payload::Zeroed)
    };
    let heap_reference = match heap_reference {
        Ok(reference) => reference,
        Err(error) => return Transfer::Error(error),
    };

    if state.collect_stats {
        state.engine.statistics.heap_allocations += 1;
    }
    let value = Value::heap_reference_with_meta(heap_reference, *reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return Transfer::Error(error);
    }

    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute slice allocation.
pub(crate) fn execute_new_slice(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let max_heap_allocations = state.options().limits.max_heap_allocations;

    // decode instruction immediate
    let Immediate::NewSlice {
        dest,
        length,
        slice_type,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // resolve slice length
    let length = match load_heap_array_length(state, *length) {
        Ok(length) => length,
        Err(error) => return Transfer::Error(error),
    };

    // resolve heap array layout facts before taking the heap borrow
    let heap_reference = {
        let element_type = match state.module.tree.get(*slice_type) {
            mir::Type::Slice { element, .. } => match element.ty() {
                Some(element) => element,
                None => {
                    return Transfer::Error(Error::TypeMismatch {
                        expected: "concrete slice element type".to_string(),
                        actual: format!("{slice_type:?}"),
                    });
                }
            },
            _ => {
                return Transfer::Error(Error::TypeMismatch {
                    expected: "slice type".to_string(),
                    actual: format!("{slice_type:?}"),
                });
            }
        };

        let Some(element_layout_id) = state.module.layout_id_for_type(element_type) else {
            return Transfer::Error(Error::TypeMismatch {
                expected: "compiled element layout".to_string(),
                actual: format!("{element_type:?}"),
            });
        };
        let element_layout = match state.module.allocation_layout(element_layout_id) {
            Ok(layout) => layout,
            Err(error) => return Transfer::Error(error),
        };
        let element_alignment = match state.layout(element_type) {
            Ok(layout) => layout.alignment(),
            Err(error) => return Transfer::Error(error),
        };
        let (byte_len, reference_map) = match destack_heap::repeated_layout(
            element_layout.byte_len,
            element_alignment,
            element_layout.reference_map,
            length,
        ) {
            Ok(layout) => layout,
            Err(error) => return Transfer::Error(Error::from(error)),
        };
        let layout = destack_heap::AllocationLayout::new(byte_len, &reference_map);
        if state.heap().heap_allocation_count() >= max_heap_allocations {
            return Transfer::Error(Error::AllocationFailed);
        }

        state.allocate_heap(layout, Payload::Zeroed)
    };
    let heap_reference = match heap_reference {
        Ok(reference) => reference,
        Err(error) => return Transfer::Error(error),
    };
    if state.collect_stats {
        state.engine.statistics.heap_allocations += 1;
    }
    let result =
        match super::value::allocate_payload_by_index(state, *dest, |state, index, value_type| {
            match index {
                0 => {
                    let reference = match state.tree().get(value_type) {
                        mir::Type::Reference {
                            kind,
                            address_space,
                            mutability,
                            is_nullable,
                            ..
                        } => ReferenceMeta::new(
                            *kind,
                            address_space.clone(),
                            *mutability,
                            *is_nullable,
                        ),
                        _ => ReferenceMeta::NONE,
                    };
                    let value = Value::heap_reference_with_meta(heap_reference, reference);

                    check_reference_kind(state, reference, value)?;

                    Ok(value)
                }
                1 => match state.tree().get(value_type) {
                    mir::Type::Usize => Ok(Value::uint(length as u64, usize::BITS as u8)),
                    _ => Err(Error::TypeMismatch {
                        expected: "slice length usize".to_string(),
                        actual: format!("{value_type:?}"),
                    }),
                },
                _ => Err(Error::InvalidFieldAccess {
                    index,
                    field_count: 2,
                }),
            }
        }) {
            Ok(result) => result,
            Err(error) => return Transfer::Error(error),
        };

    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute raw allocation.
pub(crate) fn execute_raw_alloc(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let max_raw_allocations = state.options().limits.max_raw_allocations;

    // decode instruction immediate
    let Immediate::RawAlloc {
        dest,
        reference,
        byte_len,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // allocate raw heap bytes
    let ptr = {
        let heap = state.heap_mut();
        if heap.raw_allocation_count() >= max_raw_allocations {
            return Transfer::Error(Error::AllocationFailed);
        }

        heap.allocate_raw(*byte_len, Payload::Zeroed)
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

/// Execute raw free.
pub(crate) fn execute_raw_free(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::RawFree { pointer } = &block[pc].immediate else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // accept raw pointer values
    if let Some(p) = ptr.as_raw_pointer() {
        // report invalid reference
        let heap = state.heap_mut();
        match heap.free_raw(p) {
            Ok(true) => {}
            Ok(false) => return Transfer::Error(Error::InvalidRawPointer),
            Err(HeapError::InvalidRawPointer { .. }) => {
                return Transfer::Error(Error::InvalidRawPointer);
            }
            Err(error) => return Transfer::Error(Error::from(error)),
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

/// Execute explicit synchronous cleanup.
pub(crate) fn execute_dispose(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Dispose { value } = &block[pc].immediate else {
        unreachable!()
    };

    // cleanup hooks are not lowered yet
    let _ = state.get(*value);

    next!(state, block, pc)
}

/// Execute explicit asynchronous cleanup.
pub(crate) fn execute_async_dispose(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::AsyncDispose { value } = &block[pc].immediate else {
        unreachable!()
    };

    // async cleanup hooks are not lowered yet
    let _ = state.get(*value);

    next!(state, block, pc)
}

/// Execute local heap pin.
pub(crate) fn execute_pin(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Pin { value } = &block[pc].immediate else {
        unreachable!()
    };

    // load the pinned value
    let pinned_value = state.get(*value);

    // pin one local heap reference
    if let Some(reference) = pinned_value.as_heap_reference() {
        let heap = state.heap_mut();
        match heap.pin_heap(reference) {
            Ok(reference) => state.set(*value, Value::heap_reference(reference)),
            Err(HeapError::InvalidHeapReference { .. }) => {
                return Transfer::Error(Error::InvalidHeapReference);
            }
            Err(error) => return Transfer::Error(Error::from(error)),
        }
    }
    // otherwise reject the value shape
    else {
        return Transfer::Error(Error::TypeMismatch {
            expected: "heap_reference".to_string(),
            actual: format!("{pinned_value:?}"),
        });
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute local heap unpin.
pub(crate) fn execute_unpin(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Unpin { value } = &block[pc].immediate else {
        unreachable!()
    };

    // load the pinned value
    let value = state.get(*value);

    // release one local heap pin
    if let Some(reference) = value.as_heap_reference() {
        let heap = state.heap_mut();
        match heap.unpin_heap(reference) {
            Ok(()) => {}
            Err(HeapError::InvalidHeapReference { .. }) => {
                return Transfer::Error(Error::InvalidHeapReference);
            }
            Err(error) => return Transfer::Error(Error::from(error)),
        }
    }
    // otherwise reject the value shape
    else {
        return Transfer::Error(Error::TypeMismatch {
            expected: "heap_reference".to_string(),
            actual: format!("{value:?}"),
        });
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Drop one runtime value according to its value kind.
fn drop_value(state: &mut ExecutionState<'_, '_>, value: Value) -> Result<(), Error> {
    // raw owners free their backing storage
    if let Some(pointer) = value.as_raw_pointer() {
        let heap = state.heap_mut();
        match heap.free_raw(pointer) {
            Ok(true) => return Ok(()),
            Ok(false) => return Err(Error::InvalidRawPointer),
            Err(HeapError::InvalidRawPointer { .. }) => {
                return Err(Error::InvalidRawPointer);
            }
            Err(error) => return Err(Error::from(error)),
        }
    }

    // stack owners retire the current-frame allocation
    if let Some(pointer) = value.as_stack_pointer() {
        if pointer.frame_idx != state.frame_index {
            return Err(Error::InvalidPointerType {
                actual: format!("{pointer:?}"),
            });
        }

        if !state
            .current_frame_mut()
            .retire_stack_allocation(pointer.slot)
        {
            return Err(Error::InvalidPointerType {
                actual: format!("{pointer:?}"),
            });
        }

        return Ok(());
    }

    // heap references stay GC-managed after ownership ends
    if value.as_heap_reference().is_some() || value.as_shared_heap_reference().is_some() {
        return Ok(());
    }

    Err(Error::TypeMismatch {
        expected: "droppable reference".to_string(),
        actual: format!("{value:?}"),
    })
}

/// Execute ownership end.
pub(crate) fn execute_drop(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Drop { value } = &block[pc].immediate else {
        unreachable!()
    };

    // load the dropped value
    let value = state.get(*value);

    // perform the storage-specific drop work first
    if let Err(error) = drop_value(state, value) {
        return Transfer::Error(error);
    }

    next!(state, block, pc)
}

/// Execute stack allocation.
pub(crate) fn execute_stack_alloc(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::StackAlloc {
        dest,
        reference,
        storage_type,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // allocate raw stack storage from the compiled type layout
    let byte_len = match state.storage_byte_len(*storage_type) {
        Ok(byte_len) => byte_len,
        Err(error) => return Transfer::Error(error),
    };
    let frame_index = state.frame_index;
    let allocation = crate::interpreter::StackAllocation::new(byte_len, *storage_type);
    let slot = state
        .current_frame_mut()
        .allocate_stack_allocation(allocation);
    let sp = StackPointer::new(frame_index, slot);
    let value = match stack_pointer_value_with_meta(sp, *reference) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return Transfer::Error(error);
    }

    state.set(*dest, value);

    next!(state, block, pc)
}
