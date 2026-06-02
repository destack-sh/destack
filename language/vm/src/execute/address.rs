use destack_engine::StaticAddress;
use destack_heap::{HeapReference, SharedHeapReference};

use crate::diagnostic::Error;
use crate::machine::Activation;
use crate::program::Projection;
use crate::{Cell, FramePointer, StackPointer};

/// Return one element byte offset.
#[inline(always)]
fn element_byte_offset(index: u64, stride: usize) -> usize {
    index as usize * stride
}

/// Compute a fixed-offset address from a heap reference.
#[inline(always)]
pub(crate) fn offset_heap(
    _machine: &mut Activation<'_>,
    reference: HeapReference,
    byte_offset: usize,
) -> Cell {
    let reference = reference.add_bytes(byte_offset);

    Cell::heap_reference(reference)
}

/// Compute a fixed-offset address from a shared heap reference.
#[inline(always)]
pub(crate) fn offset_shared_heap(
    _machine: &mut Activation<'_>,
    reference: SharedHeapReference,
    byte_offset: usize,
) -> Cell {
    let reference = reference.add_bytes(byte_offset);

    Cell::shared_heap_reference(reference)
}

/// Compute a fixed-offset address from a raw pointer.
#[inline(always)]
pub(crate) fn offset_raw(
    _machine: &mut Activation<'_>,
    address: usize,
    byte_offset: usize,
) -> Cell {
    let address = address + byte_offset;

    Cell::address(address)
}

/// Compute a fixed-offset address from a stack pointer.
#[inline(always)]
pub(crate) fn offset_stack(pointer: StackPointer, byte_offset: usize) -> Cell {
    let pointer = pointer.add_bytes(byte_offset);

    Cell::stack_pointer(pointer)
}

/// Compute a fixed-offset address from a frame pointer.
#[inline(always)]
pub(crate) fn offset_frame(pointer: FramePointer, byte_offset: usize) -> Cell {
    let pointer = pointer.add_bytes(byte_offset);

    Cell::frame_pointer(pointer)
}

/// Compute a fixed-offset address from a static address.
#[inline(always)]
pub(crate) fn offset_static(address: StaticAddress, byte_offset: usize) -> Result<Cell, Error> {
    let address = address
        .add_bytes(byte_offset)
        .ok_or(Error::invalid_instruction())?;

    Ok(Cell::static_address(address))
}

/// Compute an element address from a heap reference.
#[inline(always)]
pub(crate) fn element_heap(
    _machine: &mut Activation<'_>,
    reference: HeapReference,
    element: Projection,
    index: u64,
) -> Cell {
    let element_offset = element_byte_offset(index, element.byte_stride);
    let reference = reference.add_bytes(element_offset);

    Cell::heap_reference(reference)
}

/// Compute an element address from a shared heap reference.
#[inline(always)]
pub(crate) fn element_shared_heap(
    _machine: &mut Activation<'_>,
    reference: SharedHeapReference,
    element: Projection,
    index: u64,
) -> Cell {
    let element_offset = element_byte_offset(index, element.byte_stride);
    let reference = reference.add_bytes(element_offset);

    Cell::shared_heap_reference(reference)
}

/// Compute an element address from a raw pointer.
#[inline(always)]
pub(crate) fn element_raw(
    _machine: &mut Activation<'_>,
    address: usize,
    element: Projection,
    index: u64,
) -> Cell {
    let element_offset = element_byte_offset(index, element.byte_stride);
    let address = address + element_offset;

    Cell::address(address)
}

/// Compute an element address from a stack pointer.
#[inline(always)]
pub(crate) fn element_stack(
    _machine: &mut Activation<'_>,
    pointer: StackPointer,
    element: Projection,
    index: u64,
) -> Cell {
    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = pointer.add_bytes(element_offset);

    Cell::stack_pointer(pointer)
}

/// Compute an element address from a frame pointer.
#[inline(always)]
pub(crate) fn element_frame(
    _machine: &mut Activation<'_>,
    pointer: FramePointer,
    element: Projection,
    index: u64,
) -> Cell {
    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = pointer.add_bytes(element_offset);

    Cell::frame_pointer(pointer)
}

/// Compute an element address from a static address.
#[inline(always)]
pub(crate) fn element_static(
    _machine: &mut Activation<'_>,
    address: StaticAddress,
    element: Projection,
    index: u64,
) -> Result<Cell, Error> {
    let element_offset = element_byte_offset(index, element.byte_stride);
    let address = address
        .add_bytes(element_offset)
        .ok_or(Error::invalid_instruction())?;

    Ok(Cell::static_address(address))
}
