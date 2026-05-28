use super::access;
use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{Instruction, Projection, ProjectionId};
use crate::{FramePointer, StaticPointer, Word};
use destack_mir as mir;

/// Execute frame word move.
#[inline(always)]
pub(crate) fn execute_move_word(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let destination_offset = instruction.a;
    let source_offset = instruction.b;

    let value = machine.load_word_at(source_offset);
    machine.store_word_at(destination_offset, value);

    Ok(())
}

/// Execute frame byte move.
#[inline(always)]
pub(crate) fn execute_move_frame(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let destination_offset = instruction.a;
    let byte_len = instruction.b as usize;
    let source_offset = instruction.c;

    machine.copy_frame_bytes(source_offset, destination_offset, byte_len);

    Ok(())
}

/// Execute one byte range load from local heap memory.
#[inline(always)]
pub(crate) fn execute_load_heap_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, destination, destination_len) = load_bytes(machine, instruction);

    access::load_heap_bytes(machine, address, access, destination, destination_len)?;

    Ok(())
}

/// Execute one byte range load from shared heap memory.
#[inline(always)]
pub(crate) fn execute_load_shared_heap_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, destination, destination_len) = load_bytes(machine, instruction);

    access::load_shared_heap_bytes(machine, address, access, destination, destination_len)?;

    Ok(())
}

/// Execute one byte range load from local raw memory.
#[inline(always)]
pub(crate) fn execute_load_raw_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, destination, destination_len) = load_bytes(machine, instruction);

    access::load_raw_bytes(machine, address, access, destination, destination_len)?;

    Ok(())
}

/// Execute one byte range load from shared raw memory.
#[inline(always)]
pub(crate) fn execute_load_shared_raw_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, destination, destination_len) = load_bytes(machine, instruction);

    access::load_shared_raw_bytes(machine, address, access, destination, destination_len)?;

    Ok(())
}

/// Execute one byte range load from stack memory.
#[inline(always)]
pub(crate) fn execute_load_stack_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, destination, destination_len) = load_bytes(machine, instruction);

    access::load_stack_bytes(
        machine,
        address.as_stack_pointer(),
        access,
        destination,
        destination_len,
    )?;

    Ok(())
}

/// Execute one byte range load from frame memory.
#[inline(always)]
pub(crate) fn execute_load_frame_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, destination, destination_len) = load_bytes(machine, instruction);

    access::load_frame_bytes(
        machine,
        address.as_frame_pointer(),
        access,
        destination,
        destination_len,
    )?;

    Ok(())
}

/// Execute one byte range load from static memory.
#[inline(always)]
pub(crate) fn execute_load_static_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, destination, destination_len) = load_bytes(machine, instruction);

    access::load_static_bytes(
        machine,
        address.as_static_pointer(),
        access,
        destination,
        destination_len,
    )?;

    Ok(())
}

/// Execute one byte range store into local heap memory.
#[inline(always)]
pub(crate) fn execute_store_heap_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, source, byte_len) = store_bytes(machine, instruction);

    machine.with_frame_bytes_at(source, byte_len, |machine, source| {
        access::store_heap_bytes(machine, address, access, source)
    })?;

    Ok(())
}

/// Execute one byte range store into shared heap memory.
#[inline(always)]
pub(crate) fn execute_store_shared_heap_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, source, byte_len) = store_bytes(machine, instruction);

    machine.with_frame_bytes_at(source, byte_len, |machine, source| {
        access::store_shared_heap_bytes(machine, address, access, source)
    })?;

    Ok(())
}

/// Execute one byte range store into local raw memory.
#[inline(always)]
pub(crate) fn execute_store_raw_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, source, byte_len) = store_bytes(machine, instruction);

    machine.with_frame_bytes_at(source, byte_len, |machine, source| {
        access::store_raw_bytes(machine, address, access, source)
    })?;

    Ok(())
}

/// Execute one byte range store into shared raw memory.
#[inline(always)]
pub(crate) fn execute_store_shared_raw_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, source, byte_len) = store_bytes(machine, instruction);

    machine.with_frame_bytes_at(source, byte_len, |machine, source| {
        access::store_shared_raw_bytes(machine, address, access, source)
    })?;

    Ok(())
}

/// Execute one byte range store into stack memory.
#[inline(always)]
pub(crate) fn execute_store_stack_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, source, byte_len) = store_bytes(machine, instruction);

    machine.with_frame_bytes_at(source, byte_len, |machine, source| {
        access::store_stack_bytes(machine, address.as_stack_pointer(), access, source)
    })?;

    Ok(())
}

/// Execute one byte range store into frame memory.
#[inline(always)]
pub(crate) fn execute_store_frame_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, source, byte_len) = store_bytes(machine, instruction);
    let destination = address.as_frame_pointer().add_bytes(access.byte_offset);

    machine.copy_frame_bytes_to_address(source, destination.address(), byte_len);

    Ok(())
}

/// Execute one byte range store into static memory.
#[inline(always)]
pub(crate) fn execute_store_static_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, source, byte_len) = store_bytes(machine, instruction);

    machine.with_frame_bytes_at(source, byte_len, |machine, source| {
        access::store_static_bytes(machine, address.as_static_pointer(), access, source)
    })?;

    Ok(())
}

/// Load byte range fields from one instruction.
fn load_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> (Word, Projection, *mut u8, usize) {
    let destination = instruction.a;
    let address = instruction.b;
    let access = ProjectionId(instruction.c);
    let access = machine.projection(access);

    let address = machine.load_word_at(address);
    let destination = machine.frame_pointer_at(destination).address() as *mut u8;
    let destination_len = access.byte_len;

    (address, access, destination, destination_len)
}

/// Store byte range fields from one instruction.
fn store_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> (Word, Projection, u32, usize) {
    let address = instruction.a;
    let source = instruction.b;
    let access = ProjectionId(instruction.c);
    let access = machine.projection(access);

    let address = machine.load_word_at(address);
    let byte_len = access.byte_len;

    (address, access, source, byte_len)
}

/// Execute local address.
#[inline(always)]
pub(crate) fn execute_address_local(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let local = instruction.b;

    let local = mir::LocalNodeId::new(local);
    let address = machine
        .active_frame()
        .local_address(machine.frame_layout(), local)?;
    let pointer = FramePointer::from_address(address);
    let value = Word::frame_pointer(pointer);

    machine.store_word_at(dest, value);

    Ok(())
}

/// Execute static address.
pub(crate) fn execute_address_static(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;

    let global: mir::LocalNodeId<mir::Global> = mir::LocalNodeId::new(instruction.b);
    let pointer = match machine.static_pointer(global) {
        Some(pointer) => pointer,
        None => return Err(Error::undefined_global(global)),
    };
    let pointer = Word::static_pointer(pointer);

    machine.store_word_at(dest, pointer);

    Ok(())
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
    let pointer = machine.load_word_at(instruction.b);
    let byte_offset = instruction.c as usize;

    (dest, pointer, byte_offset)
}

/// Store scalar instruction fields.
#[inline(always)]
fn store_fields(machine: &Machine<'_, '_>, instruction: &Instruction) -> (Word, Word, usize) {
    let pointer = machine.load_word_at(instruction.a);
    let value = machine.load_word_at(instruction.b);
    let byte_offset = instruction.c as usize;

    (pointer, value, byte_offset)
}

/// Execute local heap scalar load.
#[inline(always)]
pub(crate) fn execute_load_heap_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, pointer, byte_offset) = load_fields(machine, instruction);

    let value = access::load_heap_scalar::<BYTE_LEN, IS_SIGNED>(machine, pointer, byte_offset);
    machine.store_word_at(dest, value);

    Ok(())
}

/// Execute shared heap scalar load.
#[inline(always)]
pub(crate) fn execute_load_shared_heap_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, pointer, byte_offset) = load_fields(machine, instruction);

    let value =
        access::load_shared_heap_scalar::<BYTE_LEN, IS_SIGNED>(machine, pointer, byte_offset);
    machine.store_word_at(dest, value);

    Ok(())
}

/// Execute local raw scalar load.
#[inline(always)]
pub(crate) fn execute_load_raw_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, pointer, byte_offset) = load_fields(machine, instruction);

    let value = access::load_raw_scalar::<BYTE_LEN, IS_SIGNED>(machine, pointer, byte_offset);
    machine.store_word_at(dest, value);

    Ok(())
}

/// Execute shared raw scalar load.
#[inline(always)]
pub(crate) fn execute_load_shared_raw_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, pointer, byte_offset) = load_fields(machine, instruction);

    let value =
        access::load_shared_raw_scalar::<BYTE_LEN, IS_SIGNED>(machine, pointer, byte_offset);
    machine.store_word_at(dest, value);

    Ok(())
}

/// Execute stack scalar load.
#[inline(always)]
pub(crate) fn execute_load_stack_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, pointer, byte_offset) = load_fields(machine, instruction);

    let value = access::load_stack_scalar::<BYTE_LEN, IS_SIGNED>(
        machine,
        pointer.as_stack_pointer(),
        byte_offset,
    );
    machine.store_word_at(dest, value);

    Ok(())
}

/// Execute static scalar load.
#[inline(always)]
pub(crate) fn execute_load_static_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, pointer, byte_offset) = load_fields(machine, instruction);

    let value = access::load_static_scalar::<BYTE_LEN, IS_SIGNED>(
        machine,
        pointer.as_static_pointer(),
        byte_offset,
    );
    machine.store_word_at(dest, value);

    Ok(())
}

/// Execute local heap scalar store.
#[inline(always)]
pub(crate) fn execute_store_heap_scalar<const BYTE_LEN: usize>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (pointer, value, byte_offset) = store_fields(machine, instruction);
    access::store_heap_scalar::<BYTE_LEN>(machine, pointer, byte_offset, value);

    Ok(())
}

/// Execute shared heap scalar store.
#[inline(always)]
pub(crate) fn execute_store_shared_heap_scalar<const BYTE_LEN: usize>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (pointer, value, byte_offset) = store_fields(machine, instruction);
    access::store_shared_heap_scalar::<BYTE_LEN>(machine, pointer, byte_offset, value);

    Ok(())
}

/// Execute local raw scalar store.
#[inline(always)]
pub(crate) fn execute_store_raw_scalar<const BYTE_LEN: usize>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (pointer, value, byte_offset) = store_fields(machine, instruction);
    access::store_raw_scalar::<BYTE_LEN>(machine, pointer, byte_offset, value);

    Ok(())
}

/// Execute shared raw scalar store.
#[inline(always)]
pub(crate) fn execute_store_shared_raw_scalar<const BYTE_LEN: usize>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (pointer, value, byte_offset) = store_fields(machine, instruction);
    access::store_shared_raw_scalar::<BYTE_LEN>(machine, pointer, byte_offset, value);

    Ok(())
}

/// Execute stack scalar store.
#[inline(always)]
pub(crate) fn execute_store_stack_scalar<const BYTE_LEN: usize>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (pointer, value, byte_offset) = store_fields(machine, instruction);
    let pointer = pointer.as_stack_pointer();
    access::store_stack_scalar::<BYTE_LEN>(machine, pointer, byte_offset, value);

    Ok(())
}

/// Execute static scalar store.
#[inline(always)]
pub(crate) fn execute_store_static_scalar<const BYTE_LEN: usize>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (pointer, value, byte_offset) = store_fields(machine, instruction);
    let pointer = pointer.as_static_pointer();

    if let Some(global) = immutable_static_region_for_pointer(machine, pointer) {
        return Err(Error::immutable_global_write(global));
    }

    access::store_static_scalar::<BYTE_LEN>(machine, pointer, byte_offset, value);

    Ok(())
}
