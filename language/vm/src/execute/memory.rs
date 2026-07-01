use super::access;
use destack_mir as mir;

use crate::diagnostic::Error;
use crate::machine::Activation;
use crate::{Cell, FramePointer};
use destack_program::vm::{Instruction, Projection, ProjectionId};

/// Execute frame cell move.
#[inline(always)]
pub(crate) fn execute_move_cell(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let destination_offset = instruction.a;
    let source_offset = instruction.b;

    let value = activation.load_cell_at(source_offset);
    activation.store_cell_at(destination_offset, value);

    Ok(())
}

/// Execute aggregate move.
#[inline(always)]
pub(crate) fn execute_move_aggregate(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let destination_offset = instruction.a;
    let byte_len = instruction.b as usize;
    let source_offset = instruction.c;

    activation.copy_frame_bytes(source_offset, destination_offset, byte_len);

    Ok(())
}

/// Execute one aggregate load from local heap memory.
#[inline(always)]
pub(crate) fn execute_load_heap_aggregate(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, destination, destination_len) =
        decode_aggregate_load(activation, instruction);

    access::load_heap_bytes(activation, address, access, destination, destination_len)?;

    Ok(())
}

/// Execute one aggregate load from shared heap memory.
#[inline(always)]
pub(crate) fn execute_load_shared_heap_aggregate(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, destination, destination_len) =
        decode_aggregate_load(activation, instruction);

    access::load_shared_heap_bytes(activation, address, access, destination, destination_len)?;

    Ok(())
}

/// Execute one aggregate load from local raw memory.
#[inline(always)]
pub(crate) fn execute_load_raw_aggregate(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, destination, destination_len) =
        decode_aggregate_load(activation, instruction);

    access::load_raw_bytes(activation, address, access, destination, destination_len)?;

    Ok(())
}

/// Execute one aggregate load from stack memory.
#[inline(always)]
pub(crate) fn execute_load_stack_aggregate(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, destination, destination_len) =
        decode_aggregate_load(activation, instruction);

    access::load_stack_bytes(
        activation,
        address.as_stack_pointer(),
        access,
        destination,
        destination_len,
    )?;

    Ok(())
}

/// Execute one aggregate load from frame memory.
#[inline(always)]
pub(crate) fn execute_load_frame_aggregate(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, destination, destination_len) =
        decode_aggregate_load(activation, instruction);

    access::load_frame_bytes(
        activation,
        address.as_frame_pointer(),
        access,
        destination,
        destination_len,
    )?;

    Ok(())
}

/// Execute one aggregate load from static memory.
#[inline(always)]
pub(crate) fn execute_load_static_aggregate(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, destination, destination_len) =
        decode_aggregate_load(activation, instruction);

    access::load_static_bytes(
        activation,
        address.as_global_address(),
        access,
        destination,
        destination_len,
    )?;

    Ok(())
}

/// Execute one aggregate store into local heap memory.
#[inline(always)]
pub(crate) fn execute_store_heap_aggregate(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, source, byte_len) = decode_aggregate_store(activation, instruction);

    activation.with_frame_bytes_at(source, byte_len, |activation, source| {
        access::store_heap_bytes(activation, address, access, source)
    })?;

    Ok(())
}

/// Execute one aggregate store into shared heap memory.
#[inline(always)]
pub(crate) fn execute_store_shared_heap_aggregate(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, source, byte_len) = decode_aggregate_store(activation, instruction);

    activation.with_frame_bytes_at(source, byte_len, |activation, source| {
        access::store_shared_heap_bytes(activation, address, access, source)
    })?;

    Ok(())
}

/// Execute one aggregate store into local raw memory.
#[inline(always)]
pub(crate) fn execute_store_raw_aggregate(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, source, byte_len) = decode_aggregate_store(activation, instruction);

    activation.with_frame_bytes_at(source, byte_len, |activation, source| {
        access::store_raw_bytes(activation, address, access, source)
    })?;

    Ok(())
}

/// Execute one aggregate store into stack memory.
#[inline(always)]
pub(crate) fn execute_store_stack_aggregate(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, source, byte_len) = decode_aggregate_store(activation, instruction);

    activation.with_frame_bytes_at(source, byte_len, |activation, source| {
        access::store_stack_bytes(activation, address.as_stack_pointer(), access, source)
    })?;

    Ok(())
}

/// Execute one aggregate store into frame memory.
#[inline(always)]
pub(crate) fn execute_store_frame_aggregate(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, source, byte_len) = decode_aggregate_store(activation, instruction);
    let destination = address.as_frame_pointer().add_bytes(access.byte_offset());

    activation.copy_frame_bytes_to_address(source, destination.address(), byte_len);

    Ok(())
}

/// Execute one aggregate store into static memory.
#[inline(always)]
pub(crate) fn execute_store_static_aggregate(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (address, access, source, byte_len) = decode_aggregate_store(activation, instruction);

    activation.with_frame_bytes_at(source, byte_len, |activation, source| {
        access::store_static_bytes(activation, address.as_global_address(), access, source)
    })?;

    Ok(())
}

/// Decode aggregate load fields from one instruction.
fn decode_aggregate_load(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> (Cell, Projection, *mut u8, usize) {
    let destination = instruction.a;
    let address = instruction.b;
    let access = ProjectionId(instruction.c);
    let access = activation.projection(access);

    let address = activation.load_cell_at(address);
    let destination = activation.frame_pointer_at(destination).address() as *mut u8;
    let destination_len = access.byte_len();

    (address, access, destination, destination_len)
}

/// Decode aggregate store fields from one instruction.
fn decode_aggregate_store(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> (Cell, Projection, u32, usize) {
    let address = instruction.a;
    let source = instruction.b;
    let access = ProjectionId(instruction.c);
    let access = activation.projection(access);

    let address = activation.load_cell_at(address);
    let byte_len = access.byte_len();

    (address, access, source, byte_len)
}

/// Execute local address.
#[inline(always)]
pub(crate) fn execute_local_address(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let local = instruction.b;

    let local = mir::LocalNodeId::new(local);
    let address = activation.active_frame().local_address(
        activation.program,
        activation.frame_layout(),
        local,
    )?;
    let pointer = FramePointer::from_address(address);
    let value = Cell::frame_pointer(pointer);

    activation.store_cell_at(dest, value);

    Ok(())
}

/// Execute global address.
pub(crate) fn execute_global_address(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;

    let global = instruction.b.into();
    let address = match activation.global_address(global) {
        Some(address) => address,
        None => return Err(Error::undefined_global(global)),
    };
    let pointer = Cell::global_address(address);

    activation.store_cell_at(dest, pointer);

    Ok(())
}

/// Load scalar access instruction fields.
#[inline(always)]
fn load_fields(activation: &Activation<'_>, instruction: &Instruction) -> (u32, Cell, usize) {
    let dest = instruction.a;
    let pointer = activation.load_cell_at(instruction.b);
    let byte_offset = instruction.c as usize;

    (dest, pointer, byte_offset)
}

/// Store scalar instruction fields.
#[inline(always)]
fn store_fields(activation: &Activation<'_>, instruction: &Instruction) -> (Cell, Cell, usize) {
    let pointer = activation.load_cell_at(instruction.a);
    let value = activation.load_cell_at(instruction.b);
    let byte_offset = instruction.c as usize;

    (pointer, value, byte_offset)
}

/// Execute local heap scalar load.
#[inline(always)]
pub(crate) fn execute_load_heap_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, pointer, byte_offset) = load_fields(activation, instruction);

    let value = access::load_heap_scalar::<BYTE_LEN, IS_SIGNED>(activation, pointer, byte_offset);
    activation.store_cell_at(dest, value);

    Ok(())
}

/// Execute shared heap scalar load.
#[inline(always)]
pub(crate) fn execute_load_shared_heap_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, pointer, byte_offset) = load_fields(activation, instruction);

    let value =
        access::load_shared_heap_scalar::<BYTE_LEN, IS_SIGNED>(activation, pointer, byte_offset);
    activation.store_cell_at(dest, value);

    Ok(())
}

/// Execute local raw scalar load.
#[inline(always)]
pub(crate) fn execute_load_raw_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, pointer, byte_offset) = load_fields(activation, instruction);

    let value = access::load_raw_scalar::<BYTE_LEN, IS_SIGNED>(activation, pointer, byte_offset);
    activation.store_cell_at(dest, value);

    Ok(())
}

/// Execute stack scalar load.
#[inline(always)]
pub(crate) fn execute_load_stack_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, pointer, byte_offset) = load_fields(activation, instruction);

    let value = access::load_stack_scalar::<BYTE_LEN, IS_SIGNED>(
        activation,
        pointer.as_stack_pointer(),
        byte_offset,
    );
    activation.store_cell_at(dest, value);

    Ok(())
}

/// Execute static scalar load.
#[inline(always)]
pub(crate) fn execute_load_static_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, pointer, byte_offset) = load_fields(activation, instruction);

    let value = access::load_static_scalar::<BYTE_LEN, IS_SIGNED>(
        activation,
        pointer.as_global_address(),
        byte_offset,
    )?;
    activation.store_cell_at(dest, value);

    Ok(())
}

/// Execute local heap scalar store.
#[inline(always)]
pub(crate) fn execute_store_heap_scalar<const BYTE_LEN: usize>(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (pointer, value, byte_offset) = store_fields(activation, instruction);
    access::store_heap_scalar::<BYTE_LEN>(activation, pointer, byte_offset, value);

    Ok(())
}

/// Execute shared heap scalar store.
#[inline(always)]
pub(crate) fn execute_store_shared_heap_scalar<const BYTE_LEN: usize>(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (pointer, value, byte_offset) = store_fields(activation, instruction);
    access::store_shared_heap_scalar::<BYTE_LEN>(activation, pointer, byte_offset, value);

    Ok(())
}

/// Execute local raw scalar store.
#[inline(always)]
pub(crate) fn execute_store_raw_scalar<const BYTE_LEN: usize>(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (pointer, value, byte_offset) = store_fields(activation, instruction);
    access::store_raw_scalar::<BYTE_LEN>(activation, pointer, byte_offset, value);

    Ok(())
}

/// Execute stack scalar store.
#[inline(always)]
pub(crate) fn execute_store_stack_scalar<const BYTE_LEN: usize>(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (pointer, value, byte_offset) = store_fields(activation, instruction);
    let pointer = pointer.as_stack_pointer();
    access::store_stack_scalar::<BYTE_LEN>(activation, pointer, byte_offset, value);

    Ok(())
}

/// Execute static scalar store.
#[inline(always)]
pub(crate) fn execute_store_static_scalar<const BYTE_LEN: usize>(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (pointer, value, byte_offset) = store_fields(activation, instruction);
    access::store_static_scalar::<BYTE_LEN>(
        activation,
        pointer.as_global_address(),
        byte_offset,
        value,
    )?;

    Ok(())
}
