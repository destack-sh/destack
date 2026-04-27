use std::ptr;

use super::bytes::{checked_frame_value_byte_range, decode_raw_value, encode_word_bits};
use super::prelude::*;
use crate::program::{PointerClass, ValueRepr, value_repr_from_type};
use destack_heap::{HeapError, Payload, SharedRawPointer};

/// Load one heap array length operand as a host usize.
#[inline(always)]
fn load_heap_array_length(
    state: &DispatchState<'_, '_>,
    value: mir::Value,
) -> Result<usize, Error> {
    let value = state.get(value);
    let length = value.as_uint();

    usize::try_from(length).map_err(|_| Error::AllocationFailed)
}

/// Load one static value directly from isolate bytes.
#[inline(always)]
fn load_static_value(
    state: &mut DispatchState<'_, '_>,
    dest: mir::Value,
    global: u32,
) -> Result<(), Error> {
    let global_id = global_id(global);
    let global = state.program.tree.get(global_id);
    let ty = global.ty.ty().ok_or_else(|| Error::ConcreteMirRequired {
        context: "static load type".to_string(),
    })?;
    let bytes = state
        .static_bytes(global_id)
        .ok_or(Error::UndefinedGlobal { global: global_id })?;
    if state.layout(ty)?.is_scalar() {
        let value = decode_raw_value(state.tree(), ty, bytes)?;
        state.set_word(dest, value);

        return Ok(());
    }

    let source = bytes.as_ptr();
    let source_len = bytes.len();
    let target = state.value_bytes_mut(dest)?;
    if target.len() != source_len {
        return Err(Error::InvalidInstruction);
    }
    unsafe {
        std::ptr::copy_nonoverlapping(source, target.as_mut_ptr(), source_len);
    }

    Ok(())
}

/// Write one little-endian u64 prefix into one byte range.
#[inline(always)]
fn write_u64_le_to_bytes(target: &mut [u8], raw: u64, byte_len: usize) {
    let target = target.as_mut_ptr();

    // use native unaligned stores for normal scalar widths
    unsafe {
        match byte_len {
            0 => {}
            1 => ptr::write_unaligned(target, raw as u8),
            2 => ptr::write_unaligned(target.cast::<u16>(), (raw as u16).to_le()),
            4 => ptr::write_unaligned(target.cast::<u32>(), (raw as u32).to_le()),
            8 => ptr::write_unaligned(target.cast::<u64>(), raw.to_le()),
            _ => {
                for byte_index in 0..byte_len {
                    let byte = ((raw >> (byte_index * 8)) & 0xFF) as u8;
                    ptr::write_unaligned(target.add(byte_index), byte);
                }
            }
        }
    }
}

/// Execute local variable load.
#[inline(always)]
pub(crate) fn execute_local_get(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::LocalGet { dest, local } = &block[pc].operands else {
        unreachable!()
    };

    // move local bytes into the destination region
    if let Err(error) = state.move_local_to_value_by_index(*local, *dest) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute local variable store.
#[inline(always)]
pub(crate) fn execute_local_set(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::LocalSet { local, value } = &block[pc].operands else {
        unreachable!()
    };

    // move the value bytes into the local region
    if let Err(error) = state.move_value_to_local_by_index(*value, *local) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute local address.
#[inline(always)]
pub(crate) fn execute_local_addr(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::LocalAddr {
        dest,
        local,
        reference,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // take the address of the local value
    let local = mir::LocalNodeId::new(*local);
    let frame_layout = state.frame_layout() as *const destack_engine::FrameLayout;
    let address = match state
        .current_frame_mut()
        .local_address(unsafe { &*frame_layout }, local)
    {
        Ok(address) => address,
        Err(error) => return Transfer::Error(error),
    };
    let pointer = FramePointer::from_address(address);
    let value = match frame_pointer_value(pointer) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return Transfer::Error(error);
    }

    // store result
    state.set_word(*dest, value);

    // continue to next instruction
    Transfer::Continue
}

/// Execute static address.
pub(crate) fn execute_static_addr(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::StaticAddr {
        dest,
        global,
        reference,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let global = global_id(*global);
    let pointer = match state.static_pointer(global) {
        Some(pointer) => pointer,
        None => return Transfer::Error(Error::UndefinedGlobal { global }),
    };
    let pointer = match static_pointer_value(pointer, 0) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return Transfer::Error(error);
    }

    // store result
    state.set_word(*dest, pointer);

    // continue to next instruction
    Transfer::Continue
}

/// Execute fused static address and load.
#[inline(always)]
pub(crate) fn execute_static_load(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::StaticLoad { dest, global } = &block[pc].operands else {
        unreachable!()
    };

    // load static value directly
    match load_static_value(state, *dest, *global) {
        Ok(()) => {}
        Err(error) => return Transfer::Error(error),
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute fused static address and store.
#[inline(always)]
pub(crate) fn execute_static_store(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::StaticStore {
        global,
        value,
        reference,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // check mutability via reference metadata
    let global_id = global_id(*global);
    let global_def = state.program.tree.get(global_id);
    if !global_def.is_mutable() || reference.mutability() != Some(mir::Mutability::Mutable) {
        return Transfer::Error(Error::ImmutableGlobalWrite { global: global_id });
    }

    // store to static directly
    let ty = global_def
        .ty
        .ty()
        .ok_or_else(|| Error::ConcreteMirRequired {
            context: "static store type".to_string(),
        });
    let ty = match ty {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let static_id = state.program.static_id(global_id);
    let layout = match state.layout(ty) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let is_scalar = layout.is_scalar();
    let static_byte_len = layout.byte_len;

    // write scalar statics without materializing a byte vector
    if is_scalar {
        let value = state.get(*value);
        let (raw, byte_len) = match encode_word_bits(state.tree(), ty, value) {
            Ok(encoded) => encoded,
            Err(error) => return Transfer::Error(error),
        };
        let Some(target) = state.statics.bytes_mut(static_id) else {
            return Transfer::Error(Error::UndefinedGlobal { global: global_id });
        };
        if target.len() != byte_len {
            return Transfer::Error(Error::InvalidInstruction);
        }

        write_u64_le_to_bytes(target, raw, byte_len);

        return Transfer::Continue;
    }

    // copy frame bytes into the static region
    let (source, source_len) = match checked_frame_value_byte_range(state, *value, static_byte_len)
    {
        Ok(source) => source,
        Err(error) => return Transfer::Error(error),
    };
    let Some(target) = state.statics.bytes_mut(static_id) else {
        return Transfer::Error(Error::UndefinedGlobal { global: global_id });
    };
    if target.len() != source_len {
        return Transfer::Error(Error::InvalidInstruction);
    }
    unsafe {
        ptr::copy_nonoverlapping(source, target.as_mut_ptr(), source_len);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Return the destination frame bytes for one byte load.
#[inline(always)]
fn load_destination(
    state: &mut DispatchState<'_, '_>,
    dest: mir::Value,
    byte_len: usize,
) -> Result<(*mut u8, usize), Error> {
    let target = state.value_bytes_mut(dest)?;
    if target.len() != byte_len {
        return Err(Error::InvalidInstruction);
    }

    Ok((target.as_mut_ptr(), target.len()))
}

/// Return the immutable static region for one static address.
#[inline(always)]
fn immutable_static_region_for_pointer(
    state: &DispatchState<'_, '_>,
    pointer: StaticPointer,
) -> Option<mir::LocalNodeId<mir::Global>> {
    let region = state
        .statics
        .region_for_pointer(pointer)
        .or_else(|| state.program.statics.region_for_pointer(pointer))?;
    if region.is_mutable {
        return None;
    }

    Some(mir::LocalNodeId::new(region.id.0))
}

/// Execute atomic load.
pub(crate) fn execute_atomic_load(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::AtomicLoad {
        dest,
        pointer,
        raw_pointee,
        ..
    } = &block[pc].operands
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
    state.set_word(*dest, value);

    // continue to next instruction
    Transfer::Continue
}

/// Execute atomic store.
pub(crate) fn execute_atomic_store(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::AtomicStore {
        pointer,
        value,
        raw_pointee,
        ..
    } = &block[pc].operands
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
    Transfer::Continue
}

/// Execute atomic compare exchange.
pub(crate) fn execute_atomic_compare_exchange(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::AtomicCompareExchange {
        dest,
        pointer,
        expected,
        new_value,
        raw_pointee,
        ..
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load values
    let pointer = state.get(*pointer);
    let expected = state.get(*expected);
    let new_value = state.get(*new_value);

    // execute the compare exchange
    if let Err(error) = state.execute_atomic_compare_exchange_value(
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
        return Transfer::Error(error.error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute atomic read modify write.
pub(crate) fn execute_atomic_rmw(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::AtomicRmw {
        dest,
        operator,
        pointer,
        value,
        raw_pointee,
        ..
    } = &block[pc].operands
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
    state.set_word(*dest, result);

    // continue to next instruction
    Transfer::Continue
}

/// Execute atomic fence.
pub(crate) fn execute_atomic_fence(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::AtomicFence = &block[pc].operands else {
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
    Transfer::Continue
}

/// Execute one execution and memory synchronization barrier.
pub(crate) fn execute_barrier(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Barrier = &block[pc].operands else {
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
    Transfer::Continue
}

/// Execute heap reference load.
#[inline(always)]
pub(crate) fn execute_load_heap(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Load {
        dest,
        pointer,
        access,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    let value = match access::load_heap_reference(state, ptr, *access) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute shared heap reference load.
#[inline(always)]
pub(crate) fn execute_load_shared_heap(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Load {
        dest,
        pointer,
        access,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    let value = match access::load_shared_heap_reference(state, ptr, *access) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute raw pointer load.
#[inline(always)]
pub(crate) fn execute_load_raw(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Load {
        dest,
        pointer,
        access,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    if access.is_scalar {
        let value = match access::load_raw_pointer(state, ptr, *access) {
            Ok(value) => value,
            Err(error) => return Transfer::Error(error),
        };
        state.set_word(*dest, value);

        return Transfer::Continue;
    }

    let (target, target_len) = match load_destination(state, *dest, access.byte_len) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = access::load_raw_pointer_bytes_into(state, ptr, *access, target, target_len)
    {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute shared raw pointer load.
#[inline(always)]
pub(crate) fn execute_load_shared_raw(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Load {
        dest,
        pointer,
        access,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    if access.is_scalar {
        let value = match access::load_shared_raw_pointer(state, ptr, *access) {
            Ok(value) => value,
            Err(error) => return Transfer::Error(error),
        };
        state.set_word(*dest, value);

        return Transfer::Continue;
    }

    let (target, target_len) = match load_destination(state, *dest, access.byte_len) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) =
        access::load_shared_raw_pointer_bytes_into(state, ptr, *access, target, target_len)
    {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute stack pointer load.
#[inline(always)]
pub(crate) fn execute_load_stack(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Load {
        dest,
        pointer,
        access,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    if access.is_scalar {
        let value = match access::load_stack_pointer(state, ptr.as_stack_pointer(), *access) {
            Ok(value) => value,
            Err(error) => return Transfer::Error(error),
        };
        state.set_word(*dest, value);

        return Transfer::Continue;
    }

    let (target, target_len) = match load_destination(state, *dest, access.byte_len) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = access::load_stack_pointer_bytes_into(
        state,
        ptr.as_stack_pointer(),
        *access,
        target,
        target_len,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute frame pointer load.
#[inline(always)]
pub(crate) fn execute_load_frame(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Load {
        dest,
        pointer,
        access,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    if access.is_scalar {
        let value = match access::load_frame_pointer(state, ptr.as_frame_pointer(), *access) {
            Ok(value) => value,
            Err(error) => return Transfer::Error(error),
        };
        state.set_word(*dest, value);

        return Transfer::Continue;
    }

    let (target, target_len) = match load_destination(state, *dest, access.byte_len) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = access::load_frame_pointer_bytes_into(
        state,
        ptr.as_frame_pointer(),
        *access,
        target,
        target_len,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute static pointer load.
#[inline(always)]
pub(crate) fn execute_load_static(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Load {
        dest,
        pointer,
        access,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    if access.is_scalar {
        let value = match access::load_static_pointer(state, ptr.as_static_pointer(), *access) {
            Ok(value) => value,
            Err(error) => return Transfer::Error(error),
        };
        state.set_word(*dest, value);

        return Transfer::Continue;
    }

    let (target, target_len) = match load_destination(state, *dest, access.byte_len) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = access::load_static_pointer_bytes_into(
        state,
        ptr.as_static_pointer(),
        *access,
        target,
        target_len,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute heap reference store.
#[inline(always)]
pub(crate) fn execute_store_heap(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Store {
        pointer,
        value,
        reference,
        access,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load pointer
    let ptr = state.get(*pointer);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return Transfer::Error(error);
    }

    let value = state.get(*value);
    if let Err(error) = access::store_heap_reference(state, ptr, *access, value) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute heap reference byte store.
#[inline(always)]
pub(crate) fn execute_store_heap_bytes(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Store {
        pointer,
        value,
        reference,
        access,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load pointer
    let ptr = state.get(*pointer);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return Transfer::Error(error);
    }

    let (source, source_len) = match checked_frame_value_byte_range(state, *value, access.byte_len)
    {
        Ok(source) => source,
        Err(error) => return Transfer::Error(error),
    };
    let bytes = unsafe { std::slice::from_raw_parts(source, source_len) };
    if let Err(error) = access::store_heap_reference_bytes(state, ptr, *access, bytes) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute shared heap reference store.
#[inline(always)]
pub(crate) fn execute_store_shared_heap(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Store {
        pointer,
        value,
        reference,
        access,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load pointer
    let ptr = state.get(*pointer);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return Transfer::Error(error);
    }

    let value = state.get(*value);
    if let Err(error) = access::store_shared_heap_reference(state, ptr, *access, value) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute shared heap reference byte store.
#[inline(always)]
pub(crate) fn execute_store_shared_heap_bytes(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Store {
        pointer,
        value,
        reference,
        access,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load pointer
    let ptr = state.get(*pointer);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return Transfer::Error(error);
    }

    let (source, source_len) = match checked_frame_value_byte_range(state, *value, access.byte_len)
    {
        Ok(source) => source,
        Err(error) => return Transfer::Error(error),
    };
    let bytes = unsafe { std::slice::from_raw_parts(source, source_len) };
    if let Err(error) = access::store_shared_heap_reference_bytes(state, ptr, *access, bytes) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute raw pointer store.
#[inline(always)]
pub(crate) fn execute_store_raw(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Store {
        pointer,
        value,
        reference,
        access,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load pointer
    let ptr = state.get(*pointer);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return Transfer::Error(error);
    }
    // write through raw pointer
    if access.is_scalar {
        let value = state.get(*value);
        if let Err(error) = access::store_raw_pointer(state, ptr, *access, value) {
            return Transfer::Error(error);
        }
    } else {
        let (source, source_len) =
            match checked_frame_value_byte_range(state, *value, access.byte_len) {
                Ok(source) => source,
                Err(error) => return Transfer::Error(error),
            };
        let bytes = unsafe { std::slice::from_raw_parts(source, source_len) };
        if let Err(error) = access::store_raw_pointer_bytes(state, ptr, *access, bytes) {
            return Transfer::Error(error);
        }
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute shared raw pointer store.
#[inline(always)]
pub(crate) fn execute_store_shared_raw(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Store {
        pointer,
        value,
        reference,
        access,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load pointer
    let ptr = state.get(*pointer);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return Transfer::Error(error);
    }

    // write through shared raw pointer
    if access.is_scalar {
        let value = state.get(*value);
        if let Err(error) = access::store_shared_raw_pointer(state, ptr, *access, value) {
            return Transfer::Error(error);
        }
    } else {
        let (source, source_len) =
            match checked_frame_value_byte_range(state, *value, access.byte_len) {
                Ok(source) => source,
                Err(error) => return Transfer::Error(error),
            };
        let bytes = unsafe { std::slice::from_raw_parts(source, source_len) };
        if let Err(error) = access::store_shared_raw_pointer_bytes(state, ptr, *access, bytes) {
            return Transfer::Error(error);
        }
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute stack pointer store.
#[inline(always)]
pub(crate) fn execute_store_stack(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Store {
        pointer,
        value,
        reference,
        access,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load pointer
    let ptr = state.get(*pointer);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return Transfer::Error(error);
    }

    // write through stack pointer
    let pointer = ptr.as_stack_pointer();
    if access.is_scalar {
        let value = state.get(*value);
        if let Err(error) = access::store_stack_pointer(state, pointer, *access, value) {
            return Transfer::Error(error);
        }
    } else {
        let (source, source_len) =
            match checked_frame_value_byte_range(state, *value, access.byte_len) {
                Ok(source) => source,
                Err(error) => return Transfer::Error(error),
            };
        let bytes = unsafe { std::slice::from_raw_parts(source, source_len) };
        if let Err(error) = access::store_stack_pointer_bytes(state, pointer, *access, bytes) {
            return Transfer::Error(error);
        }
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute frame pointer store.
#[inline(always)]
pub(crate) fn execute_store_frame(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Store {
        pointer,
        value,
        reference,
        access,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load pointer
    let ptr = state.get(*pointer);
    let pointer = ptr.as_frame_pointer();

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return Transfer::Error(error);
    }

    // write through frame pointer
    if access.is_scalar {
        let value = state.get(*value);
        if let Err(error) = access::store_frame_pointer(state, pointer, *access, value) {
            return Transfer::Error(error);
        }
    } else {
        let (source, source_len) =
            match checked_frame_value_byte_range(state, *value, access.byte_len) {
                Ok(source) => source,
                Err(error) => return Transfer::Error(error),
            };
        let bytes = unsafe { std::slice::from_raw_parts(source, source_len) };
        if let Err(error) = access::store_frame_pointer_bytes(state, pointer, *access, bytes) {
            return Transfer::Error(error);
        }
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute static pointer store.
#[inline(always)]
pub(crate) fn execute_store_static(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Store {
        pointer,
        value,
        reference,
        access,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load pointer
    let ptr = state.get(*pointer);
    let pointer = ptr.as_static_pointer();

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return Transfer::Error(error);
    }
    if let Some(global) = immutable_static_region_for_pointer(state, pointer) {
        return Transfer::Error(Error::ImmutableGlobalWrite { global });
    }

    // write through static pointer
    if access.is_scalar {
        let value = state.get(*value);
        if let Err(error) = access::store_static_pointer(state, pointer, *access, value) {
            return Transfer::Error(error);
        }
    } else {
        let (source, source_len) =
            match checked_frame_value_byte_range(state, *value, access.byte_len) {
                Ok(source) => source,
                Err(error) => return Transfer::Error(error),
            };
        let bytes = unsafe { std::slice::from_raw_parts(source, source_len) };
        if let Err(error) = access::store_static_pointer_bytes(state, pointer, *access, bytes) {
            return Transfer::Error(error);
        }
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute heap allocation.
pub(crate) fn execute_new(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::New {
        dest,
        reference,
        layout_id,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // allocate heap allocation
    let heap_reference = state.allocate_heap_layout(*layout_id, Payload::Zeroed);
    let heap_reference = match heap_reference {
        Ok(reference) => reference,
        Err(error) => return Transfer::Error(error),
    };
    let value = Word::heap_reference(heap_reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return Transfer::Error(error);
    }

    state.set_word(*dest, value);

    // continue to next instruction
    Transfer::Continue
}

/// Execute slice allocation.
pub(crate) fn execute_new_slice(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::NewSlice {
        dest,
        length,
        element_layout_id,
        element_alignment,
    } = &block[pc].operands
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
        let element_layout = match state.program.allocation_layout(*element_layout_id) {
            Ok(layout) => layout,
            Err(error) => return Transfer::Error(error),
        };
        let (byte_len, reference_map) = match destack_heap::repeated_layout(
            element_layout.byte_len,
            *element_alignment,
            element_layout.reference_map,
            length,
        ) {
            Ok(layout) => layout,
            Err(error) => return Transfer::Error(Error::from(error)),
        };
        let layout = destack_heap::AllocationLayout::new(byte_len, &reference_map);

        state.allocate_heap(layout, Payload::Zeroed)
    };
    let heap_reference = match heap_reference {
        Ok(reference) => reference,
        Err(error) => return Transfer::Error(error),
    };
    let result =
        super::bytes::write_frame_fields(state, *dest, |state, index, value_type| match index {
            0 => {
                let reference = match state.tree().get(value_type) {
                    mir::Type::Reference {
                        kind,
                        address_space,
                        mutability,
                        is_nullable,
                        ..
                    } => {
                        ReferenceMeta::new(*kind, address_space.clone(), *mutability, *is_nullable)
                    }
                    _ => ReferenceMeta::NONE,
                };
                let value = Word::heap_reference(heap_reference);

                check_reference_kind(state, reference, value)?;

                Ok(value)
            }
            1 => match state.tree().get(value_type) {
                mir::Type::Usize => Ok(Word::uint(length as u64, usize::BITS as u8)),
                _ => Err(Error::TypeMismatch {
                    expected: "slice length usize".to_string(),
                    actual: format!("{value_type:?}"),
                }),
            },
            _ => Err(Error::InvalidFieldAccess {
                index,
                field_count: 2,
            }),
        });
    match result {
        Ok(()) => {}
        Err(error) => return Transfer::Error(error),
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute raw allocation.
pub(crate) fn execute_raw_alloc(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::RawAlloc {
        dest,
        reference,
        byte_len,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // allocate raw heap bytes
    let ptr = state.heap_mut().allocate_raw(*byte_len, Payload::Zeroed);
    let ptr = match ptr {
        Ok(ptr) => ptr,
        Err(error) => return Transfer::Error(Error::from(error)),
    };
    let value = Word::raw_pointer(ptr);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return Transfer::Error(error);
    }

    state.set_word(*dest, value);

    // continue to next instruction
    Transfer::Continue
}

/// Execute raw free.
pub(crate) fn execute_raw_free(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::RawFree { pointer } = &block[pc].operands else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    let pointer = RawPointer::from_bits(ptr.bits() as usize);
    let heap = state.heap_mut();
    match heap.free_raw(pointer) {
        Ok(true) => {}
        Ok(false) => return Transfer::Error(Error::InvalidRawPointer),
        Err(HeapError::InvalidRawPointer { .. }) => {
            return Transfer::Error(Error::InvalidRawPointer);
        }
        Err(error) => return Transfer::Error(Error::from(error)),
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute explicit synchronous cleanup.
pub(crate) fn execute_dispose(
    _state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Dispose = &block[pc].operands else {
        unreachable!()
    };

    Transfer::Continue
}

/// Execute explicit asynchronous cleanup.
pub(crate) fn execute_async_dispose(
    _state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::AsyncDispose = &block[pc].operands else {
        unreachable!()
    };

    Transfer::Continue
}

/// Execute local heap pin.
pub(crate) fn execute_pin(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Pin { value } = &block[pc].operands else {
        unreachable!()
    };

    // load the pinned value
    let pinned_value = state.get(*value);

    let value_type = match state.value_type(*value) {
        Ok(value_type) => value_type,
        Err(error) => return Transfer::Error(error),
    };
    let repr = value_repr_from_type(state.tree(), value_type);
    if !matches!(
        repr,
        ValueRepr::Pointer {
            pointer_class: PointerClass::Heap,
            ..
        }
    ) {
        return Transfer::Error(Error::TypeMismatch {
            expected: "heap_reference".to_string(),
            actual: format!("{pinned_value:?}"),
        });
    }

    let reference = HeapReference::from_bits(pinned_value.bits() as usize);
    let heap = state.heap_mut();
    match heap.pin_heap(reference) {
        Ok(reference) => state.set_word(*value, Word::heap_reference(reference)),
        Err(HeapError::InvalidHeapReference { .. }) => {
            return Transfer::Error(Error::InvalidHeapReference);
        }
        Err(error) => return Transfer::Error(Error::from(error)),
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute local heap unpin.
pub(crate) fn execute_unpin(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Unpin { value } = &block[pc].operands else {
        unreachable!()
    };

    // load the pinned value
    let pinned_value = state.get(*value);

    let value_type = match state.value_type(*value) {
        Ok(value_type) => value_type,
        Err(error) => return Transfer::Error(error),
    };
    let repr = value_repr_from_type(state.tree(), value_type);
    if !matches!(
        repr,
        ValueRepr::Pointer {
            pointer_class: PointerClass::Heap,
            ..
        }
    ) {
        return Transfer::Error(Error::TypeMismatch {
            expected: "heap_reference".to_string(),
            actual: format!("{pinned_value:?}"),
        });
    }

    let reference = HeapReference::from_bits(pinned_value.bits() as usize);
    let heap = state.heap_mut();
    match heap.unpin_heap(reference) {
        Ok(()) => {}
        Err(HeapError::InvalidHeapReference { .. }) => {
            return Transfer::Error(Error::InvalidHeapReference);
        }
        Err(error) => return Transfer::Error(Error::from(error)),
    }

    // continue to next instruction
    Transfer::Continue
}

/// Drop one runtime value according to its MIR type.
fn drop_value(
    state: &mut DispatchState<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    value: Word,
) -> Result<(), Error> {
    let repr = value_repr_from_type(state.tree(), ty);
    let ValueRepr::Pointer { pointer_class, .. } = repr else {
        return Err(Error::TypeMismatch {
            expected: "droppable reference".to_string(),
            actual: format!("{value:?}"),
        });
    };

    match pointer_class {
        PointerClass::Heap | PointerClass::SharedHeap => Ok(()),
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
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Drop { value } = &block[pc].operands else {
        unreachable!()
    };

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
pub(crate) fn execute_stack_alloc(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::StackAlloc {
        dest,
        reference,
        allocation_type,
    } = &block[pc].operands
    else {
        unreachable!()
    };

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
    let value = match stack_pointer_value(sp) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return Transfer::Error(error);
    }

    state.set_word(*dest, value);

    Transfer::Continue
}
