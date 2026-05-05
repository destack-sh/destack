use std::slice;

use super::access;
use super::reference::check_reference_address_space;
use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{
    FrameAccess, FrameAccessId, Instruction, PointeeAccess, PointeeAccessId, Transfer,
};
use crate::{FramePointer, ReferenceMeta, StaticPointer, Word};
use destack_mir as mir;

/// Execute local variable load.
#[inline(always)]
pub(crate) fn execute_load_local(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let local = instruction.b;

    // move local bytes into the destination region
    if let Err(error) = machine.move_local_to_offset(local, dest) {
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
    let value = instruction.b;

    // move the value bytes into the local region
    if let Err(error) = machine.move_offset_to_local(value, local) {
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
    let destination_offset = instruction.a;
    let destination_access = FrameAccessId(instruction.b);
    let source_offset = instruction.c;
    let source_access = FrameAccessId(instruction.d);

    move_frame_range(
        machine,
        destination_offset,
        machine.frame_access(destination_access),
        source_offset,
        machine.frame_access(source_access),
    );

    Transfer::Continue
}

/// Execute one byte range load from local heap memory.
#[inline(always)]
pub(crate) fn execute_load_heap_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, destination, destination_len) = load_bytes(machine, instruction);

    if let Err(error) =
        access::load_heap_bytes(machine, address, access, destination, destination_len)
    {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte range load from shared heap memory.
#[inline(always)]
pub(crate) fn execute_load_shared_heap_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, destination, destination_len) = load_bytes(machine, instruction);

    if let Err(error) =
        access::load_shared_heap_bytes(machine, address, access, destination, destination_len)
    {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte range load from local raw memory.
#[inline(always)]
pub(crate) fn execute_load_raw_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, destination, destination_len) = load_bytes(machine, instruction);

    if let Err(error) =
        access::load_raw_bytes(machine, address, access, destination, destination_len)
    {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte range load from shared raw memory.
#[inline(always)]
pub(crate) fn execute_load_shared_raw_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, destination, destination_len) = load_bytes(machine, instruction);

    if let Err(error) =
        access::load_shared_raw_bytes(machine, address, access, destination, destination_len)
    {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte range load from stack memory.
#[inline(always)]
pub(crate) fn execute_load_stack_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, destination, destination_len) = load_bytes(machine, instruction);

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

/// Execute one byte range load from frame memory.
#[inline(always)]
pub(crate) fn execute_load_frame_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, destination, destination_len) = load_bytes(machine, instruction);

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

/// Execute one byte range load from static memory.
#[inline(always)]
pub(crate) fn execute_load_static_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, destination, destination_len) = load_bytes(machine, instruction);

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

/// Execute one byte range store into local heap memory.
#[inline(always)]
pub(crate) fn execute_store_heap_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, source, source_len) = store_bytes(machine, instruction);
    let source = unsafe { slice::from_raw_parts(source, source_len) };

    if let Err(error) = access::store_heap_bytes(machine, address, access, source) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte range store into shared heap memory.
#[inline(always)]
pub(crate) fn execute_store_shared_heap_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, source, source_len) = store_bytes(machine, instruction);
    let source = unsafe { slice::from_raw_parts(source, source_len) };

    if let Err(error) = access::store_shared_heap_bytes(machine, address, access, source) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte range store into local raw memory.
#[inline(always)]
pub(crate) fn execute_store_raw_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, source, source_len) = store_bytes(machine, instruction);
    let source = unsafe { slice::from_raw_parts(source, source_len) };

    if let Err(error) = access::store_raw_bytes(machine, address, access, source) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte range store into shared raw memory.
#[inline(always)]
pub(crate) fn execute_store_shared_raw_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, source, source_len) = store_bytes(machine, instruction);
    let source = unsafe { slice::from_raw_parts(source, source_len) };

    if let Err(error) = access::store_shared_raw_bytes(machine, address, access, source) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte range store into stack memory.
#[inline(always)]
pub(crate) fn execute_store_stack_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, source, source_len) = store_bytes(machine, instruction);
    let source = unsafe { slice::from_raw_parts(source, source_len) };

    if let Err(error) =
        access::store_stack_bytes(machine, address.as_stack_pointer(), access, source)
    {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte range store into frame memory.
#[inline(always)]
pub(crate) fn execute_store_frame_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, source, source_len) = store_bytes(machine, instruction);
    let source = unsafe { slice::from_raw_parts(source, source_len) };

    if let Err(error) =
        access::store_frame_bytes(machine, address.as_frame_pointer(), access, source)
    {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute one byte range store into static memory.
#[inline(always)]
pub(crate) fn execute_store_static_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (address, access, source, source_len) = store_bytes(machine, instruction);
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
    destination_offset: u32,
    destination_access: FrameAccess,
    source_offset: u32,
    source_access: FrameAccess,
) {
    let destination_offset = destination_offset + destination_access.byte_offset as u32;
    let source_offset = source_offset + source_access.byte_offset as u32;

    machine.copy_frame_bytes(source_offset, destination_offset, source_access.byte_len);
}

/// Load byte range fields from one instruction.
fn load_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> (Word, PointeeAccess, *mut u8, usize) {
    let destination = instruction.a;
    let address = instruction.b;
    let access = PointeeAccessId(instruction.c);
    let access = machine.pointee_access(access);

    let address = machine.get_word_at(address);
    let destination = machine.frame_pointer_at(destination).address() as *mut u8;
    let destination_len = access.byte_len;

    (address, access, destination, destination_len)
}

/// Store byte range fields from one instruction.
fn store_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> (Word, PointeeAccess, *const u8, usize) {
    let address = instruction.a;
    let source = instruction.b;
    let access = PointeeAccessId(instruction.c);
    let access = machine.pointee_access(access);

    let address = machine.get_word_at(address);
    let source = machine.frame_pointer_at(source).address() as *const u8;
    let source_len = access.byte_len;

    (address, access, source, source_len)
}

/// Execute local address.
#[inline(always)]
pub(crate) fn execute_address_local(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let local = instruction.b;
    let reference = ReferenceMeta::from_bits(instruction.c as u8);

    // take the address of the local value
    let local = mir::LocalNodeId::new(local);
    let frame_layout = machine.frame_layout() as *const _;
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
    machine.set_word_at(dest, value);

    // continue to next instruction
    Transfer::Continue
}

/// Execute static address.
pub(crate) fn execute_address_static(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
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
    machine.set_word_at(dest, pointer);

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

/// Load scalar access instruction fields.
#[inline(always)]
fn load_fields(machine: &Machine<'_, '_>, instruction: &Instruction) -> (u32, Word, usize) {
    let dest = instruction.a;
    let pointer = machine.get_word_at(instruction.b);
    let byte_offset = instruction.c as usize;

    (dest, pointer, byte_offset)
}

/// Store scalar instruction fields.
#[inline(always)]
fn store_fields(machine: &Machine<'_, '_>, instruction: &Instruction) -> (Word, Word, usize) {
    let pointer = machine.get_word_at(instruction.a);
    let value = machine.get_word_at(instruction.b);
    let byte_offset = instruction.c as usize;

    (pointer, value, byte_offset)
}

/// Execute local heap scalar load.
#[inline(always)]
pub(crate) fn execute_load_heap_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, pointer, byte_offset) = load_fields(machine, instruction);

    let value = match access::load_heap_scalar::<BYTE_LEN, IS_SIGNED>(machine, pointer, byte_offset)
    {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    machine.set_word_at(dest, value);

    Transfer::Continue
}

/// Execute shared heap scalar load.
#[inline(always)]
pub(crate) fn execute_load_shared_heap_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, pointer, byte_offset) = load_fields(machine, instruction);

    let value =
        match access::load_shared_heap_scalar::<BYTE_LEN, IS_SIGNED>(machine, pointer, byte_offset)
        {
            Ok(value) => value,
            Err(error) => return Transfer::Error(error),
        };
    machine.set_word_at(dest, value);

    Transfer::Continue
}

/// Execute local raw scalar load.
#[inline(always)]
pub(crate) fn execute_load_raw_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, pointer, byte_offset) = load_fields(machine, instruction);

    let value = match access::load_raw_scalar::<BYTE_LEN, IS_SIGNED>(machine, pointer, byte_offset)
    {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    machine.set_word_at(dest, value);

    Transfer::Continue
}

/// Execute shared raw scalar load.
#[inline(always)]
pub(crate) fn execute_load_shared_raw_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, pointer, byte_offset) = load_fields(machine, instruction);

    let value = match access::load_shared_raw_scalar::<BYTE_LEN, IS_SIGNED>(
        machine,
        pointer,
        byte_offset,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    machine.set_word_at(dest, value);

    Transfer::Continue
}

/// Execute stack scalar load.
#[inline(always)]
pub(crate) fn execute_load_stack_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, pointer, byte_offset) = load_fields(machine, instruction);

    let value = match access::load_stack_scalar::<BYTE_LEN, IS_SIGNED>(
        machine,
        pointer.as_stack_pointer(),
        byte_offset,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    machine.set_word_at(dest, value);

    Transfer::Continue
}

/// Execute static scalar load.
#[inline(always)]
pub(crate) fn execute_load_static_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, pointer, byte_offset) = load_fields(machine, instruction);

    let value = match access::load_static_scalar::<BYTE_LEN, IS_SIGNED>(
        machine,
        pointer.as_static_pointer(),
        byte_offset,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    machine.set_word_at(dest, value);

    Transfer::Continue
}

/// Execute local heap scalar store.
#[inline(always)]
pub(crate) fn execute_store_heap_scalar<const BYTE_LEN: usize>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (pointer, value, byte_offset) = store_fields(machine, instruction);
    if let Err(error) = access::store_heap_scalar::<BYTE_LEN>(machine, pointer, byte_offset, value)
    {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute shared heap scalar store.
#[inline(always)]
pub(crate) fn execute_store_shared_heap_scalar<const BYTE_LEN: usize>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (pointer, value, byte_offset) = store_fields(machine, instruction);
    if let Err(error) =
        access::store_shared_heap_scalar::<BYTE_LEN>(machine, pointer, byte_offset, value)
    {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute local raw scalar store.
#[inline(always)]
pub(crate) fn execute_store_raw_scalar<const BYTE_LEN: usize>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (pointer, value, byte_offset) = store_fields(machine, instruction);
    if let Err(error) = access::store_raw_scalar::<BYTE_LEN>(machine, pointer, byte_offset, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute shared raw scalar store.
#[inline(always)]
pub(crate) fn execute_store_shared_raw_scalar<const BYTE_LEN: usize>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (pointer, value, byte_offset) = store_fields(machine, instruction);
    if let Err(error) =
        access::store_shared_raw_scalar::<BYTE_LEN>(machine, pointer, byte_offset, value)
    {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute stack scalar store.
#[inline(always)]
pub(crate) fn execute_store_stack_scalar<const BYTE_LEN: usize>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (pointer, value, byte_offset) = store_fields(machine, instruction);
    let pointer = pointer.as_stack_pointer();
    if let Err(error) = access::store_stack_scalar::<BYTE_LEN>(machine, pointer, byte_offset, value)
    {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute static scalar store.
#[inline(always)]
pub(crate) fn execute_store_static_scalar<const BYTE_LEN: usize>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (pointer, value, byte_offset) = store_fields(machine, instruction);
    let pointer = pointer.as_static_pointer();

    if let Some(global) = immutable_static_region_for_pointer(machine, pointer) {
        return Transfer::Error(Error::ImmutableGlobalWrite { global });
    }

    if let Err(error) =
        access::store_static_scalar::<BYTE_LEN>(machine, pointer, byte_offset, value)
    {
        return Transfer::Error(error);
    }

    Transfer::Continue
}
