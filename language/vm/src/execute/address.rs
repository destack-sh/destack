use crate::interpreter::Machine;
use crate::program::Projection;
use crate::{
    HeapReference, RawPointer, SharedHeapReference, SharedRawPointer, StackPointer, StaticPointer,
    Word,
};

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
    pointer: RawPointer,
    byte_offset: usize,
) -> Word {
    let pointer = pointer.add_bytes(byte_offset);

    Word::raw_pointer(pointer)
}

/// Compute a fixed-offset address from a shared raw pointer.
#[inline(always)]
pub(crate) fn offset_shared_raw(
    _machine: &mut Machine<'_, '_>,
    pointer: SharedRawPointer,
    byte_offset: usize,
) -> Word {
    let pointer = pointer.add_bytes(byte_offset);

    Word::shared_raw_pointer(pointer)
}

/// Compute a fixed-offset address from a stack pointer.
#[inline(always)]
pub(crate) fn offset_stack(pointer: StackPointer, byte_offset: usize) -> Word {
    let pointer = pointer.add_bytes(byte_offset);

    Word::stack_pointer(pointer)
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
    pointer: RawPointer,
    element: Projection,
    index: u64,
) -> Word {
    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = pointer.add_bytes(element_offset);

    Word::raw_pointer(pointer)
}

/// Compute an element address from a shared raw pointer.
#[inline(always)]
pub(crate) fn element_shared_raw(
    _machine: &mut Machine<'_, '_>,
    pointer: SharedRawPointer,
    element: Projection,
    index: u64,
) -> Word {
    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = pointer.add_bytes(element_offset);

    Word::shared_raw_pointer(pointer)
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
