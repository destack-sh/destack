use std::slice;

use super::access;
use super::frame::frame_value_bytes_for_access;
use super::reference::check_reference_address_space;
use super::slice::{load_slice_length, store_slice};
use crate::diagnostic::Error;
use crate::interpreter::DispatchState;
use crate::program::{
    AsyncDispose, AtomicCompareExchange, AtomicFence, AtomicLoad, AtomicRmw, AtomicStore,
    BarrierWrite, Dispose, DropValue, FrameAccess, Instruction, Load, LoadFrameBytes, LocalAddr,
    LocalGet, LocalSet, MoveFrame, New, NewSlice, Pin, PointeeAccess, PointerClass, RawAlloc,
    RawFree, StackAlloc, StaticAddr, StaticLoad, StaticStore, Store, StoreFrameBytes, Transfer,
    UnpinValue, ValueLayout, decode_word_bytes, value_layout_from_type, word_layout_from_type,
};
use crate::{FramePointer, HeapReference, RawPointer, StackPointer, StaticPointer, Word};
use destack_heap::{AllocationPlan, HeapError, Payload, SharedRawPointer, repeated_layout};
use {destack_engine as engine, destack_mir as mir};

/// Load one scalar static directly from isolate bytes.
#[inline(always)]
fn load_static_word(
    state: &mut DispatchState<'_, '_>,
    dest: mir::Value,
    global: u32,
) -> Result<(), Error> {
    let global_id: mir::LocalNodeId<mir::Global> = mir::LocalNodeId::new(global);
    let global = state.program.tree.get(global_id);
    let ty = global.ty.ty().ok_or_else(|| Error::MissingRepresentation {
        context: "static load type".to_string(),
    })?;
    let bytes = state
        .static_bytes(global_id)
        .ok_or(Error::UndefinedGlobal { global: global_id })?;
    let value = decode_word_bytes(state.tree(), ty, bytes)?;
    state.set_word(dest, value);

    Ok(())
}

/// Execute local variable load.
#[inline(always)]
pub(crate) fn execute_load_local(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let LocalGet { dest, local } = instruction.payload_as::<LocalGet>();

    // move local bytes into the destination region
    if let Err(error) = state.move_local_to_value(*local, *dest) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute local variable store.
#[inline(always)]
pub(crate) fn execute_store_local(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let LocalSet { local, value } = instruction.payload_as::<LocalSet>();

    // move the value bytes into the local region
    if let Err(error) = state.move_value_to_local(*value, *local) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute frame byte move.
#[inline(always)]
pub(crate) fn execute_move_frame(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let MoveFrame {
        destination,
        destination_access,
        source,
        source_access,
    } = instruction.payload_as::<MoveFrame>();

    let result = move_frame_range(
        state,
        *destination,
        state.frame_access(*destination_access),
        *source,
        state.frame_access(*source_access),
    );
    if let Err(error) = result {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte load from memory into a frame value.
#[inline(always)]
pub(crate) fn execute_load_frame_bytes(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let LoadFrameBytes {
        destination,
        address,
        access,
    } = instruction.payload_as::<LoadFrameBytes>();
    let access = state.pointee_access(*access);

    if let Err(error) = load_frame_bytes(state, *destination, *address, access) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte store from a frame value into memory.
#[inline(always)]
pub(crate) fn execute_store_frame_bytes(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let StoreFrameBytes {
        address,
        source,
        access,
    } = instruction.payload_as::<StoreFrameBytes>();
    let access = state.pointee_access(*access);

    if let Err(error) = store_frame_bytes(state, *address, *source, access) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Move one frame byte range into another frame byte range.
fn move_frame_range(
    state: &mut DispatchState<'_, '_>,
    destination: mir::Value,
    destination_access: FrameAccess,
    source: mir::Value,
    source_access: FrameAccess,
) -> Result<(), Error> {
    move_frame_range_at(
        state,
        destination,
        destination_access.byte_offset,
        source,
        source_access.byte_offset,
        destination_access,
        source_access,
    )
}

/// Move one frame byte range at concrete offsets.
fn move_frame_range_at(
    state: &mut DispatchState<'_, '_>,
    destination: mir::Value,
    destination_offset: usize,
    source: mir::Value,
    source_offset: usize,
    destination_access: FrameAccess,
    source_access: FrameAccess,
) -> Result<(), Error> {
    if destination_access.byte_len != source_access.byte_len {
        return Err(Error::InvalidInstruction);
    }

    state.move_value_range(
        destination,
        destination_offset,
        source,
        source_offset,
        source_access.byte_len,
    )
}

/// Load bytes through one computed address into a frame value.
fn load_frame_bytes(
    state: &mut DispatchState<'_, '_>,
    destination: mir::Value,
    address: mir::Value,
    access: PointeeAccess,
) -> Result<(), Error> {
    // expose the destination region as raw bytes
    let address = state.get(address);
    let destination = state.value_bytes_mut(destination)?;
    let destination_len = destination.len();
    let destination = destination.as_mut_ptr();

    access::load_pointer_bytes(state, address, access, destination, destination_len)
}

/// Store bytes from a frame value through one computed address.
fn store_frame_bytes(
    state: &mut DispatchState<'_, '_>,
    address: mir::Value,
    source: mir::Value,
    access: PointeeAccess,
) -> Result<(), Error> {
    // borrow the bytes before loading the address
    let address = state.get(address);
    let (source, source_len) = frame_value_bytes_for_access(state, source, access.byte_len)?;
    let source = unsafe { slice::from_raw_parts(source, source_len) };

    access::store_pointer_bytes(state, address, access, source)
}

/// Execute local address.
#[inline(always)]
pub(crate) fn execute_address_local(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let LocalAddr {
        dest,
        local,
        reference,
    } = instruction.payload_as::<LocalAddr>();

    // take the address of the local value
    let local = mir::LocalNodeId::new(*local);
    let frame_layout = state.frame_layout() as *const engine::FrameLayout;
    let address = match state
        .current_frame_mut()
        .local_address(unsafe { &*frame_layout }, local)
    {
        Ok(address) => address,
        Err(error) => return Transfer::Error(error),
    };
    let pointer = FramePointer::from_address(address);
    let value = Word::frame_pointer(pointer);

    // validate reference address space
    if let Err(error) = check_reference_address_space(state, *reference) {
        return Transfer::Error(error);
    }

    // store result
    state.set_word(*dest, value);

    // continue to next instruction
    Transfer::Continue
}

/// Execute static address.
pub(crate) fn execute_address_static(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let StaticAddr {
        dest,
        global,
        reference,
    } = instruction.payload_as::<StaticAddr>();

    let global: mir::LocalNodeId<mir::Global> = mir::LocalNodeId::new(*global);
    let pointer = match state.static_pointer(global) {
        Some(pointer) => pointer,
        None => return Transfer::Error(Error::UndefinedGlobal { global }),
    };
    let pointer = Word::static_pointer(pointer);

    // validate reference address space
    if let Err(error) = check_reference_address_space(state, *reference) {
        return Transfer::Error(error);
    }

    // store result
    state.set_word(*dest, pointer);

    // continue to next instruction
    Transfer::Continue
}

/// Execute fused static address and load.
#[inline(always)]
pub(crate) fn execute_load_static_id(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let StaticLoad { dest, global } = instruction.payload_as::<StaticLoad>();

    // load static word directly
    match load_static_word(state, *dest, *global) {
        Ok(()) => {}
        Err(error) => return Transfer::Error(error),
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute fused static address and word store.
#[inline(always)]
pub(crate) fn execute_store_static_id(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let StaticStore {
        global,
        value,
        reference,
    } = instruction.payload_as::<StaticStore>();

    // check mutability via reference metadata
    let global_id: mir::LocalNodeId<mir::Global> = mir::LocalNodeId::new(*global);
    let global_def = state.program.tree.get(global_id);
    if !global_def.is_mutable() || reference.mutability() != Some(mir::Mutability::Mutable) {
        return Transfer::Error(Error::ImmutableGlobalWrite { global: global_id });
    }

    // store to static directly
    let ty = global_def
        .ty
        .ty()
        .ok_or_else(|| Error::MissingRepresentation {
            context: "static store type".to_string(),
        });
    let ty = match ty {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let layout = match state.layout(ty) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let access = PointeeAccess {
        pointer_class: PointerClass::Static,
        value_type: ty,
        byte_offset: 0,
        byte_len: layout.byte_len,
        word_layout: word_layout_from_type(state.tree(), ty),
    };

    // store scalar static without allocating a byte vector
    let value = state.get(*value);
    let static_id = state.program.static_id(global_id);
    let Some(pointer) = state.statics.pointer(static_id) else {
        return Transfer::Error(Error::UndefinedGlobal { global: global_id });
    };
    if let Err(error) = access::store_static_word(state, pointer, access, value) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
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
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let AtomicLoad {
        dest,
        pointer,
        raw_pointee,
        ..
    } = instruction.payload_as::<AtomicLoad>();

    // load pointer
    let pointer = state.get(*pointer);

    // execute the load
    let value = match state.atomic_load_value(
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
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let AtomicStore {
        pointer,
        value,
        raw_pointee,
        ..
    } = instruction.payload_as::<AtomicStore>();

    // load values
    let pointer = state.get(*pointer);
    let value = state.get(*value);

    // execute the store
    if let Err(error) = state.atomic_store_value(
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
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let AtomicCompareExchange {
        dest,
        pointer,
        expected,
        new_value,
        raw_pointee,
        ..
    } = instruction.payload_as::<AtomicCompareExchange>();

    // load values
    let pointer = state.get(*pointer);
    let expected = state.get(*expected);
    let new_value = state.get(*new_value);

    // execute the compare exchange
    if let Err(error) = state.atomic_compare_exchange_value(
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
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let AtomicRmw {
        dest,
        operator,
        pointer,
        value,
        raw_pointee,
        ..
    } = instruction.payload_as::<AtomicRmw>();

    // load values
    let pointer = state.get(*pointer);
    let value = state.get(*value);

    // execute the read modify write
    let result = match state.atomic_rmw_value(
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
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let _ = instruction.payload_as::<AtomicFence>();

    // execute the fence
    if let Err(error) = state.atomic_fence(
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

/// Execute managed barrier write.
pub(crate) fn execute_barrier_write(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let BarrierWrite {
        object,
        offset,
        byte_len,
        pointer_class,
    } = instruction.payload_as::<BarrierWrite>();

    // load barrier range
    let object = state.get(*object);
    let offset = state.get(*offset).as_uint() as usize;
    let byte_len = state.get(*byte_len).as_uint() as usize;

    // publish to the owning collector
    let result = match pointer_class {
        PointerClass::Heap => {
            state
                .heap_mut()
                .write_barrier(object.as_heap_reference(), offset, byte_len)
        }
        PointerClass::SharedHeap => {
            state
                .shared()
                .write_barrier(object.as_shared_heap_reference(), offset, byte_len)
        }
        _ => {
            return Transfer::Error(Error::TypeMismatch {
                expected: "managed barrier reference".to_string(),
                actual: format!("{pointer_class:?}"),
            });
        }
    };

    // report invalid heap ranges
    if let Err(error) = result {
        return Transfer::Error(Error::from(error));
    }

    Transfer::Continue
}

/// Execute local heap word load.
#[inline(always)]
pub(crate) fn execute_load_heap(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let Load {
        dest,
        pointer,
        access,
    } = instruction.payload_as::<Load>();

    // load pointer word
    let pointer = state.get(*pointer);
    let access = state.pointee_access(*access);

    let value = match access::load_heap_word(state, pointer, access) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute shared heap word load.
#[inline(always)]
pub(crate) fn execute_load_shared_heap(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let Load {
        dest,
        pointer,
        access,
    } = instruction.payload_as::<Load>();

    // load pointer word
    let pointer = state.get(*pointer);
    let access = state.pointee_access(*access);

    let value = match access::load_shared_heap_word(state, pointer, access) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute local raw word load.
#[inline(always)]
pub(crate) fn execute_load_raw(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let Load {
        dest,
        pointer,
        access,
    } = instruction.payload_as::<Load>();

    // load pointer word
    let pointer = state.get(*pointer);
    let access = state.pointee_access(*access);

    let value = match access::load_raw_word(state, pointer, access) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute shared raw word load.
#[inline(always)]
pub(crate) fn execute_load_shared_raw(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let Load {
        dest,
        pointer,
        access,
    } = instruction.payload_as::<Load>();

    // load pointer word
    let pointer = state.get(*pointer);
    let access = state.pointee_access(*access);

    let value = match access::load_shared_raw_word(state, pointer, access) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute stack word load.
#[inline(always)]
pub(crate) fn execute_load_stack(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let Load {
        dest,
        pointer,
        access,
    } = instruction.payload_as::<Load>();

    // load pointer word
    let pointer = state.get(*pointer);
    let access = state.pointee_access(*access);

    let value = match access::load_stack_word(state, pointer.as_stack_pointer(), access) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute static word load.
#[inline(always)]
pub(crate) fn execute_load_static(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let Load {
        dest,
        pointer,
        access,
    } = instruction.payload_as::<Load>();

    // load pointer word
    let pointer = state.get(*pointer);
    let access = state.pointee_access(*access);

    let value = match access::load_static_word(state, pointer.as_static_pointer(), access) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute local heap word store.
#[inline(always)]
pub(crate) fn execute_store_heap(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let Store {
        pointer,
        value,
        access,
    } = instruction.payload_as::<Store>();

    // load pointer word
    let pointer = state.get(*pointer);
    let access = state.pointee_access(*access);

    let value = state.get(*value);
    if let Err(error) = access::store_heap_word(state, pointer, access, value) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute shared heap word store.
#[inline(always)]
pub(crate) fn execute_store_shared_heap(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let Store {
        pointer,
        value,
        access,
    } = instruction.payload_as::<Store>();

    // load pointer word
    let pointer = state.get(*pointer);
    let access = state.pointee_access(*access);

    let value = state.get(*value);
    if let Err(error) = access::store_shared_heap_word(state, pointer, access, value) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute local raw word store.
#[inline(always)]
pub(crate) fn execute_store_raw(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let Store {
        pointer,
        value,
        access,
    } = instruction.payload_as::<Store>();

    // load pointer word
    let pointer = state.get(*pointer);
    let access = state.pointee_access(*access);

    let value = state.get(*value);
    if let Err(error) = access::store_raw_word(state, pointer, access, value) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute shared raw word store.
#[inline(always)]
pub(crate) fn execute_store_shared_raw(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let Store {
        pointer,
        value,
        access,
    } = instruction.payload_as::<Store>();

    // load pointer word
    let pointer = state.get(*pointer);
    let access = state.pointee_access(*access);

    let value = state.get(*value);
    if let Err(error) = access::store_shared_raw_word(state, pointer, access, value) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute stack word store.
#[inline(always)]
pub(crate) fn execute_store_stack(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let Store {
        pointer,
        value,
        access,
    } = instruction.payload_as::<Store>();

    // load pointer word
    let pointer = state.get(*pointer);
    let access = state.pointee_access(*access);

    let pointer = pointer.as_stack_pointer();
    let value = state.get(*value);
    if let Err(error) = access::store_stack_word(state, pointer, access, value) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute static word store.
#[inline(always)]
pub(crate) fn execute_store_static(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let Store {
        pointer,
        value,
        access,
    } = instruction.payload_as::<Store>();

    // load pointer word
    let pointer = state.get(*pointer);
    let pointer = pointer.as_static_pointer();
    let access = state.pointee_access(*access);

    if let Some(global) = immutable_static_region_for_pointer(state, pointer) {
        return Transfer::Error(Error::ImmutableGlobalWrite { global });
    }

    let value = state.get(*value);
    if let Err(error) = access::store_static_word(state, pointer, access, value) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

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

    if let Err(error) = store_slice(state, *dest, backing_reference, length) {
        return Transfer::Error(error);
    }

    // continue to next instruction
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

    // continue to next instruction
    Transfer::Continue
}

/// Execute raw free.
pub(crate) fn execute_free_raw(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let RawFree { pointer } = instruction.payload_as::<RawFree>();

    // load pointer word
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

    // continue to next instruction
    Transfer::Continue
}

/// Execute explicit synchronous cleanup.
pub(crate) fn execute_dispose(
    _state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let _ = instruction.payload_as::<Dispose>();

    Transfer::Continue
}

/// Execute explicit asynchronous cleanup.
pub(crate) fn execute_async_dispose(
    _state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
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

    // load the pinned value
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
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let UnpinValue { value } = instruction.payload_as::<UnpinValue>();

    // load the pinned value
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
