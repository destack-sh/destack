use std::slice;

use super::access;
use super::frame::frame_value_bytes_for_access;
use super::reference::check_reference_address_space;
use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{
    FrameAccess, FrameAccessId, Instruction, PointeeAccess, PointeeAccessId, PointerClass,
    Transfer, decode_word_bytes, word_layout_from_type,
};
use crate::{FramePointer, ReferenceMeta, StaticPointer, Word};
use {destack_engine as engine, destack_mir as mir};

/// Load one scalar static directly from isolate bytes.
#[inline(always)]
fn load_static_word(
    machine: &mut Machine<'_, '_>,
    dest: mir::Value,
    global: u32,
) -> Result<(), Error> {
    let global_id: mir::LocalNodeId<mir::Global> = mir::LocalNodeId::new(global);
    let global = machine.program.tree.get(global_id);
    let ty = global.ty.ty().ok_or_else(|| Error::MissingRepresentation {
        context: "static load type".to_string(),
    })?;
    let bytes = machine
        .static_bytes(global_id)
        .ok_or(Error::UndefinedGlobal { global: global_id })?;
    let value = decode_word_bytes(machine.tree(), ty, bytes)?;
    machine.set_word(dest, value);

    Ok(())
}

/// Execute local variable load.
#[inline(always)]
pub(crate) fn execute_load_local(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let local = instruction.b;

    // move local bytes into the destination region
    if let Err(error) = machine.move_local_to_value(local, dest) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute local variable store.
#[inline(always)]
pub(crate) fn execute_store_local(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let local = instruction.a;
    let value = mir::Value::new(instruction.b);

    // move the value bytes into the local region
    if let Err(error) = machine.move_value_to_local(value, local) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute frame byte move.
#[inline(always)]
pub(crate) fn execute_move_frame(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let destination = mir::Value::new(instruction.a);
    let destination_access = FrameAccessId(instruction.b);
    let source = mir::Value::new(instruction.c);
    let source_access = FrameAccessId(instruction.d);

    let result = move_frame_range(
        machine,
        destination,
        machine.frame_access(destination_access),
        source,
        machine.frame_access(source_access),
    );
    if let Err(error) = result {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte load from local heap memory.
#[inline(always)]
pub(crate) fn execute_load_heap_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, destination, destination_len) =
        match load_bytes_operands(machine, instruction) {
            Ok(operands) => operands,
            Err(error) => return Transfer::Error(error),
        };

    if let Err(error) =
        access::load_heap_bytes(machine, address, access, destination, destination_len)
    {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte load from shared heap memory.
#[inline(always)]
pub(crate) fn execute_load_shared_heap_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, destination, destination_len) =
        match load_bytes_operands(machine, instruction) {
            Ok(operands) => operands,
            Err(error) => return Transfer::Error(error),
        };

    if let Err(error) =
        access::load_shared_heap_bytes(machine, address, access, destination, destination_len)
    {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte load from local raw memory.
#[inline(always)]
pub(crate) fn execute_load_raw_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, destination, destination_len) =
        match load_bytes_operands(machine, instruction) {
            Ok(operands) => operands,
            Err(error) => return Transfer::Error(error),
        };

    if let Err(error) =
        access::load_raw_bytes(machine, address, access, destination, destination_len)
    {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte load from shared raw memory.
#[inline(always)]
pub(crate) fn execute_load_shared_raw_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, destination, destination_len) =
        match load_bytes_operands(machine, instruction) {
            Ok(operands) => operands,
            Err(error) => return Transfer::Error(error),
        };

    if let Err(error) =
        access::load_shared_raw_bytes(machine, address, access, destination, destination_len)
    {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte load from stack memory.
#[inline(always)]
pub(crate) fn execute_load_stack_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, destination, destination_len) =
        match load_bytes_operands(machine, instruction) {
            Ok(operands) => operands,
            Err(error) => return Transfer::Error(error),
        };

    if let Err(error) = access::load_stack_bytes(
        machine,
        address.as_stack_pointer(),
        access,
        destination,
        destination_len,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte load from frame memory.
#[inline(always)]
pub(crate) fn execute_load_frame_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, destination, destination_len) =
        match load_bytes_operands(machine, instruction) {
            Ok(operands) => operands,
            Err(error) => return Transfer::Error(error),
        };

    if let Err(error) = access::load_frame_bytes(
        machine,
        address.as_frame_pointer(),
        access,
        destination,
        destination_len,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte load from static memory.
#[inline(always)]
pub(crate) fn execute_load_static_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, destination, destination_len) =
        match load_bytes_operands(machine, instruction) {
            Ok(operands) => operands,
            Err(error) => return Transfer::Error(error),
        };

    if let Err(error) = access::load_static_bytes(
        machine,
        address.as_static_pointer(),
        access,
        destination,
        destination_len,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte store into local heap memory.
#[inline(always)]
pub(crate) fn execute_store_heap_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, source, source_len) = match store_bytes_operands(machine, instruction) {
        Ok(operands) => operands,
        Err(error) => return Transfer::Error(error),
    };
    let source = unsafe { slice::from_raw_parts(source, source_len) };

    if let Err(error) = access::store_heap_bytes(machine, address, access, source) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte store into shared heap memory.
#[inline(always)]
pub(crate) fn execute_store_shared_heap_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, source, source_len) = match store_bytes_operands(machine, instruction) {
        Ok(operands) => operands,
        Err(error) => return Transfer::Error(error),
    };
    let source = unsafe { slice::from_raw_parts(source, source_len) };

    if let Err(error) = access::store_shared_heap_bytes(machine, address, access, source) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte store into local raw memory.
#[inline(always)]
pub(crate) fn execute_store_raw_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, source, source_len) = match store_bytes_operands(machine, instruction) {
        Ok(operands) => operands,
        Err(error) => return Transfer::Error(error),
    };
    let source = unsafe { slice::from_raw_parts(source, source_len) };

    if let Err(error) = access::store_raw_bytes(machine, address, access, source) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte store into shared raw memory.
#[inline(always)]
pub(crate) fn execute_store_shared_raw_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, source, source_len) = match store_bytes_operands(machine, instruction) {
        Ok(operands) => operands,
        Err(error) => return Transfer::Error(error),
    };
    let source = unsafe { slice::from_raw_parts(source, source_len) };

    if let Err(error) = access::store_shared_raw_bytes(machine, address, access, source) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte store into stack memory.
#[inline(always)]
pub(crate) fn execute_store_stack_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, source, source_len) = match store_bytes_operands(machine, instruction) {
        Ok(operands) => operands,
        Err(error) => return Transfer::Error(error),
    };
    let source = unsafe { slice::from_raw_parts(source, source_len) };

    if let Err(error) =
        access::store_stack_bytes(machine, address.as_stack_pointer(), access, source)
    {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte store into frame memory.
#[inline(always)]
pub(crate) fn execute_store_frame_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, source, source_len) = match store_bytes_operands(machine, instruction) {
        Ok(operands) => operands,
        Err(error) => return Transfer::Error(error),
    };
    let source = unsafe { slice::from_raw_parts(source, source_len) };

    if let Err(error) =
        access::store_frame_bytes(machine, address.as_frame_pointer(), access, source)
    {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte store into static memory.
#[inline(always)]
pub(crate) fn execute_store_static_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, source, source_len) = match store_bytes_operands(machine, instruction) {
        Ok(operands) => operands,
        Err(error) => return Transfer::Error(error),
    };
    let source = unsafe { slice::from_raw_parts(source, source_len) };

    if let Err(error) =
        access::store_static_bytes(machine, address.as_static_pointer(), access, source)
    {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Move one frame byte range into another frame byte range.
fn move_frame_range(
    machine: &mut Machine<'_, '_>,
    destination: mir::Value,
    destination_access: FrameAccess,
    source: mir::Value,
    source_access: FrameAccess,
) -> Result<(), Error> {
    move_frame_range_at(
        machine,
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
    machine: &mut Machine<'_, '_>,
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

    machine.move_value_range(
        destination,
        destination_offset,
        source,
        source_offset,
        source_access.byte_len,
    )
}

/// Load byte operands from one instruction.
fn load_bytes_operands(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(Word, PointeeAccess, *mut u8, usize), Error> {
    let destination = mir::Value::new(instruction.a);
    let address = mir::Value::new(instruction.b);
    let access = PointeeAccessId(instruction.c);
    let access = machine.pointee_access(access);

    let address = machine.get(address);
    let destination = machine.value_bytes_mut(destination)?;
    let destination_len = destination.len();
    let destination = destination.as_mut_ptr();

    Ok((address, access, destination, destination_len))
}

/// Load byte store operands from one instruction.
fn store_bytes_operands(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(Word, PointeeAccess, *const u8, usize), Error> {
    let address = mir::Value::new(instruction.a);
    let source = mir::Value::new(instruction.b);
    let access = PointeeAccessId(instruction.c);
    let access = machine.pointee_access(access);

    let address = machine.get(address);
    let (source, source_len) = frame_value_bytes_for_access(machine, source, access.byte_len)?;

    Ok((address, access, source, source_len))
}

/// Execute local address.
#[inline(always)]
pub(crate) fn execute_address_local(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let local = instruction.b;
    let reference = ReferenceMeta::from_bits(instruction.c as u8);

    // take the address of the local value
    let local = mir::LocalNodeId::new(local);
    let frame_layout = machine.frame_layout() as *const engine::FrameLayout;
    let address = match machine
        .current_frame_mut()
        .local_address(unsafe { &*frame_layout }, local)
    {
        Ok(address) => address,
        Err(error) => return Transfer::Error(error),
    };
    let pointer = FramePointer::from_address(address);
    let value = Word::frame_pointer(pointer);

    // validate reference address space
    if let Err(error) = check_reference_address_space(machine, reference) {
        return Transfer::Error(error);
    }

    // store result
    machine.set_word(dest, value);

    // continue to next instruction
    Transfer::Continue
}

/// Execute static address.
pub(crate) fn execute_address_static(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let reference = ReferenceMeta::from_bits(instruction.c as u8);

    let global: mir::LocalNodeId<mir::Global> = mir::LocalNodeId::new(instruction.b);
    let pointer = match machine.static_pointer(global) {
        Some(pointer) => pointer,
        None => return Transfer::Error(Error::UndefinedGlobal { global }),
    };
    let pointer = Word::static_pointer(pointer);

    // validate reference address space
    if let Err(error) = check_reference_address_space(machine, reference) {
        return Transfer::Error(error);
    }

    // store result
    machine.set_word(dest, pointer);

    // continue to next instruction
    Transfer::Continue
}

/// Execute fused static address and load.
#[inline(always)]
pub(crate) fn execute_load_static_id(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let global = instruction.b;

    // load static word directly
    match load_static_word(machine, dest, global) {
        Ok(()) => {}
        Err(error) => return Transfer::Error(error),
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute fused static address and word store.
#[inline(always)]
pub(crate) fn execute_store_static_id(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let global = instruction.a;
    let value = mir::Value::new(instruction.b);
    let reference = ReferenceMeta::from_bits(instruction.c as u8);

    // check mutability via reference metadata
    let global_id: mir::LocalNodeId<mir::Global> = mir::LocalNodeId::new(global);
    let global_def = machine.program.tree.get(global_id);
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
    let layout = match machine.layout(ty) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let access = PointeeAccess {
        pointer_class: PointerClass::Static,
        value_type: ty,
        byte_offset: 0,
        byte_len: layout.byte_len,
        word_layout: word_layout_from_type(machine.tree(), ty),
    };

    // store scalar static without allocating a byte vector
    let value = machine.get(value);
    let static_id = machine.program.static_id(global_id);
    let Some(pointer) = machine.statics.pointer(static_id) else {
        return Transfer::Error(Error::UndefinedGlobal { global: global_id });
    };
    if let Err(error) = access::store_static_word(machine, pointer, access, value) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Return the immutable static region for one static address.
#[inline(always)]
fn immutable_static_region_for_pointer(
    machine: &Machine<'_, '_>,
    pointer: StaticPointer,
) -> Option<mir::LocalNodeId<mir::Global>> {
    let region = machine
        .statics
        .region_for_pointer(pointer)
        .or_else(|| machine.program.statics.region_for_pointer(pointer))?;
    if region.is_mutable {
        return None;
    }

    Some(mir::LocalNodeId::new(region.id.0))
}

/// Load direct word access operands.
#[inline(always)]
fn load_operands(
    machine: &Machine<'_, '_>,
    instruction: &Instruction,
) -> (mir::Value, Word, PointeeAccess) {
    let dest = mir::Value::new(instruction.a);
    let pointer = machine.get(mir::Value::new(instruction.b));
    let access = machine.pointee_access(PointeeAccessId(instruction.c));

    (dest, pointer, access)
}

/// Load direct word store operands.
#[inline(always)]
fn store_operands(
    machine: &Machine<'_, '_>,
    instruction: &Instruction,
) -> (Word, Word, PointeeAccess) {
    let pointer = machine.get(mir::Value::new(instruction.a));
    let value = machine.get(mir::Value::new(instruction.b));
    let access = machine.pointee_access(PointeeAccessId(instruction.c));

    (pointer, value, access)
}

/// Execute local heap word load.
#[inline(always)]
pub(crate) fn execute_load_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, pointer, access) = load_operands(machine, instruction);

    let value = match access::load_heap_word(machine, pointer, access) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    machine.set_word(dest, value);

    Transfer::Continue
}

/// Execute shared heap word load.
#[inline(always)]
pub(crate) fn execute_load_shared_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, pointer, access) = load_operands(machine, instruction);

    let value = match access::load_shared_heap_word(machine, pointer, access) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    machine.set_word(dest, value);

    Transfer::Continue
}

/// Execute local raw word load.
#[inline(always)]
pub(crate) fn execute_load_raw(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, pointer, access) = load_operands(machine, instruction);

    let value = match access::load_raw_word(machine, pointer, access) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    machine.set_word(dest, value);

    Transfer::Continue
}

/// Execute shared raw word load.
#[inline(always)]
pub(crate) fn execute_load_shared_raw(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, pointer, access) = load_operands(machine, instruction);

    let value = match access::load_shared_raw_word(machine, pointer, access) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    machine.set_word(dest, value);

    Transfer::Continue
}

/// Execute stack word load.
#[inline(always)]
pub(crate) fn execute_load_stack(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, pointer, access) = load_operands(machine, instruction);

    let value = match access::load_stack_word(machine, pointer.as_stack_pointer(), access) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    machine.set_word(dest, value);

    Transfer::Continue
}

/// Execute static word load.
#[inline(always)]
pub(crate) fn execute_load_static(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, pointer, access) = load_operands(machine, instruction);

    let value = match access::load_static_word(machine, pointer.as_static_pointer(), access) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    machine.set_word(dest, value);

    Transfer::Continue
}

/// Execute local heap word store.
#[inline(always)]
pub(crate) fn execute_store_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (pointer, value, access) = store_operands(machine, instruction);
    if let Err(error) = access::store_heap_word(machine, pointer, access, value) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute shared heap word store.
#[inline(always)]
pub(crate) fn execute_store_shared_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (pointer, value, access) = store_operands(machine, instruction);
    if let Err(error) = access::store_shared_heap_word(machine, pointer, access, value) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute local raw word store.
#[inline(always)]
pub(crate) fn execute_store_raw(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (pointer, value, access) = store_operands(machine, instruction);
    if let Err(error) = access::store_raw_word(machine, pointer, access, value) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute shared raw word store.
#[inline(always)]
pub(crate) fn execute_store_shared_raw(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (pointer, value, access) = store_operands(machine, instruction);
    if let Err(error) = access::store_shared_raw_word(machine, pointer, access, value) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute stack word store.
#[inline(always)]
pub(crate) fn execute_store_stack(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (pointer, value, access) = store_operands(machine, instruction);
    let pointer = pointer.as_stack_pointer();
    if let Err(error) = access::store_stack_word(machine, pointer, access, value) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute static word store.
#[inline(always)]
pub(crate) fn execute_store_static(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (pointer, value, access) = store_operands(machine, instruction);
    let pointer = pointer.as_static_pointer();

    if let Some(global) = immutable_static_region_for_pointer(machine, pointer) {
        return Transfer::Error(Error::ImmutableGlobalWrite { global });
    }

    if let Err(error) = access::store_static_word(machine, pointer, access, value) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}
