use std::slice;

use super::access;
use super::frame::frame_value_bytes_for_access;
use super::reference::check_reference_address_space;
use crate::diagnostic::Error;
use crate::interpreter::DispatchState;
use crate::program::{
    FrameAccess, Instruction, Load, LoadFrameBytes, LocalAddr, LocalGet, LocalSet, MoveFrame,
    PointeeAccess, PointerClass, StaticAddr, StaticLoad, StaticStore, Store, StoreFrameBytes,
    Transfer, decode_word_bytes, word_layout_from_type,
};
use crate::{FramePointer, StaticPointer, Word};
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
