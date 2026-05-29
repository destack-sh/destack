use crate::interpreter::Machine;
use crate::program::Projection;
use crate::{FramePointer, HeapReference, SharedHeapReference, StackPointer, StaticPointer, Word};

/// Return one element byte offset.
#[inline(always)]
fn element_byte_offset(index: u64, stride: usize) -> usize {
    index as usize * stride
}

/// Compute a fixed-offset address from a heap reference.
#[inline(always)]
pub(crate) fn offset_heap(
    _machine: &mut Machine<'_, '_>,
    reference: HeapReference,
    byte_offset: usize,
) -> Word {
    let reference = reference.add_bytes(byte_offset);

    Word::heap_reference(reference)
}

/// Compute a fixed-offset address from a shared heap reference.
#[inline(always)]
pub(crate) fn offset_shared_heap(
    _machine: &mut Machine<'_, '_>,
    reference: SharedHeapReference,
    byte_offset: usize,
) -> Word {
    let reference = reference.add_bytes(byte_offset);

    Word::shared_heap_reference(reference)
}

/// Compute a fixed-offset address from a raw pointer.
#[inline(always)]
pub(crate) fn offset_raw(
    _machine: &mut Machine<'_, '_>,
    address: usize,
    byte_offset: usize,
) -> Word {
    let address = address + byte_offset;

    Word::address(address)
}

/// Compute a fixed-offset address from a stack pointer.
#[inline(always)]
pub(crate) fn offset_stack(pointer: StackPointer, byte_offset: usize) -> Word {
    let pointer = pointer.add_bytes(byte_offset);

    Word::stack_pointer(pointer)
}

/// Compute a fixed-offset address from a frame pointer.
#[inline(always)]
pub(crate) fn offset_frame(pointer: FramePointer, byte_offset: usize) -> Word {
    let pointer = pointer.add_bytes(byte_offset);

    Word::frame_pointer(pointer)
}

/// Compute a fixed-offset address from a static pointer.
#[inline(always)]
pub(crate) fn offset_static(pointer: StaticPointer, byte_offset: usize) -> Word {
    let pointer = pointer.add_bytes(byte_offset);

    Word::static_pointer(pointer)
}

/// Compute an element address from a heap reference.
#[inline(always)]
pub(crate) fn element_heap(
    _machine: &mut Machine<'_, '_>,
    reference: HeapReference,
    element: Projection,
    index: u64,
) -> Word {
    let element_offset = element_byte_offset(index, element.byte_stride);
    let reference = reference.add_bytes(element_offset);

    Word::heap_reference(reference)
}

/// Compute an element address from a shared heap reference.
#[inline(always)]
pub(crate) fn element_shared_heap(
    _machine: &mut Machine<'_, '_>,
    reference: SharedHeapReference,
    element: Projection,
    index: u64,
) -> Word {
    let element_offset = element_byte_offset(index, element.byte_stride);
    let reference = reference.add_bytes(element_offset);

    Word::shared_heap_reference(reference)
}

/// Compute an element address from a raw pointer.
#[inline(always)]
pub(crate) fn element_raw(
    _machine: &mut Machine<'_, '_>,
    address: usize,
    element: Projection,
    index: u64,
) -> Word {
    let element_offset = element_byte_offset(index, element.byte_stride);
    let address = address + element_offset;

    Word::address(address)
}

/// Compute an element address from a stack pointer.
#[inline(always)]
pub(crate) fn element_stack(
    _machine: &mut Machine<'_, '_>,
    pointer: StackPointer,
    element: Projection,
    index: u64,
) -> Word {
    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = pointer.add_bytes(element_offset);

    Word::stack_pointer(pointer)
}

/// Compute an element address from a frame pointer.
#[inline(always)]
pub(crate) fn element_frame(
    _machine: &mut Machine<'_, '_>,
    pointer: FramePointer,
    element: Projection,
    index: u64,
) -> Word {
    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = pointer.add_bytes(element_offset);

    Word::frame_pointer(pointer)
}

/// Compute an element address from a static pointer.
#[inline(always)]
pub(crate) fn element_static(
    _machine: &mut Machine<'_, '_>,
    pointer: StaticPointer,
    element: Projection,
    index: u64,
) -> Word {
    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = pointer.add_bytes(element_offset);

    Word::static_pointer(pointer)
}
