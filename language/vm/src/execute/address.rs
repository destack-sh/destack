use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::ElementAccess;
use crate::{
    HeapReference, RawPointer, SharedHeapReference, SharedRawPointer, StackPointer, StaticPointer,
    Word,
};

/// Ensure one shared heap reference is visible to global heap metadata.
#[inline(always)]
fn ensure_shared_heap_live(
    machine: &mut Machine<'_, '_>,
    reference: SharedHeapReference,
) -> Result<(), Error> {
    if machine.shared().is_heap_live(reference) {
        return Ok(());
    }

    machine.flush_shared_allocator();
    if machine.shared().is_heap_live(reference) {
        return Ok(());
    }

    Err(Error::InvalidHeapReference)
}

/// Validate an array index against a known length.
#[inline(always)]
fn check_array_index(
    machine: &Machine<'_, '_>,
    index: u64,
    array_length: u64,
) -> Result<(), Error> {
    // skip checks when bounds are disabled
    if !machine.bounds_checks {
        return Ok(());
    }

    // reject out of bounds indices
    if index >= array_length {
        return Err(Error::InvalidArrayAccess {
            index,
            length: array_length,
        });
    }

    Ok(())
}

/// Return one element byte offset.
#[inline(always)]
fn element_byte_offset(index: u64, stride: usize) -> usize {
    index as usize * stride
}

/// Compute a fixed-offset address from a heap reference.
#[inline(always)]
pub(crate) fn offset_heap(
    machine: &mut Machine<'_, '_>,
    reference: HeapReference,
    byte_offset: usize,
) -> Result<Word, Error> {
    if machine.bounds_checks && !machine.heap().is_heap_live(reference) {
        return Err(Error::InvalidHeapReference);
    }

    if machine.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let reference = reference.add_bytes(byte_offset);

    Ok(Word::heap_reference(reference))
}

/// Compute a fixed-offset address from a shared heap reference.
#[inline(always)]
pub(crate) fn offset_shared_heap(
    machine: &mut Machine<'_, '_>,
    reference: SharedHeapReference,
    byte_offset: usize,
) -> Result<Word, Error> {
    if machine.bounds_checks {
        ensure_shared_heap_live(machine, reference)?;
    }

    if machine.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let reference = reference.add_bytes(byte_offset);

    Ok(Word::shared_heap_reference(reference))
}

/// Compute a fixed-offset address from a raw pointer.
#[inline(always)]
pub(crate) fn offset_raw(
    machine: &mut Machine<'_, '_>,
    pointer: RawPointer,
    byte_offset: usize,
) -> Result<Word, Error> {
    // reject null pointers when enabled
    if machine.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let pointer = pointer.add_bytes(byte_offset);

    Ok(Word::raw_pointer(pointer))
}

/// Compute a fixed-offset address from a shared raw pointer.
#[inline(always)]
pub(crate) fn offset_shared_raw(
    machine: &mut Machine<'_, '_>,
    pointer: SharedRawPointer,
    byte_offset: usize,
) -> Result<Word, Error> {
    // reject null pointers when enabled
    if machine.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let pointer = pointer.add_bytes(byte_offset);

    Ok(Word::shared_raw_pointer(pointer))
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
    machine: &mut Machine<'_, '_>,
    reference: HeapReference,
    element: ElementAccess,
    index: u64,
    array_length: u64,
) -> Result<Word, Error> {
    check_array_index(machine, index, array_length)?;

    if machine.bounds_checks && !machine.heap().is_heap_live(reference) {
        return Err(Error::InvalidHeapReference);
    }

    if machine.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = element_byte_offset(index, element.byte_stride);
    let reference = reference.add_bytes(element_offset);

    Ok(Word::heap_reference(reference))
}

/// Compute an element address from a shared heap reference.
#[inline(always)]
pub(crate) fn element_shared_heap(
    machine: &mut Machine<'_, '_>,
    reference: SharedHeapReference,
    element: ElementAccess,
    index: u64,
    array_length: u64,
) -> Result<Word, Error> {
    check_array_index(machine, index, array_length)?;

    if machine.bounds_checks {
        ensure_shared_heap_live(machine, reference)?;
    }

    if machine.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = element_byte_offset(index, element.byte_stride);
    let reference = reference.add_bytes(element_offset);

    Ok(Word::shared_heap_reference(reference))
}

/// Compute an element address from a raw pointer.
#[inline(always)]
pub(crate) fn element_raw(
    machine: &mut Machine<'_, '_>,
    pointer: RawPointer,
    element: ElementAccess,
    index: u64,
    array_length: u64,
) -> Result<Word, Error> {
    check_array_index(machine, index, array_length)?;

    if machine.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = pointer.add_bytes(element_offset);

    Ok(Word::raw_pointer(pointer))
}

/// Compute an element address from a shared raw pointer.
#[inline(always)]
pub(crate) fn element_shared_raw(
    machine: &mut Machine<'_, '_>,
    pointer: SharedRawPointer,
    element: ElementAccess,
    index: u64,
    array_length: u64,
) -> Result<Word, Error> {
    check_array_index(machine, index, array_length)?;

    if machine.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = pointer.add_bytes(element_offset);

    Ok(Word::shared_raw_pointer(pointer))
}

/// Compute an element address from a stack pointer.
#[inline(always)]
pub(crate) fn element_stack(
    machine: &mut Machine<'_, '_>,
    pointer: StackPointer,
    element: ElementAccess,
    index: u64,
    array_length: u64,
) -> Result<Word, Error> {
    check_array_index(machine, index, array_length)?;

    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = pointer.add_bytes(element_offset);

    Ok(Word::stack_pointer(pointer))
}

/// Compute an element address from a static pointer.
#[inline(always)]
pub(crate) fn element_static(
    machine: &mut Machine<'_, '_>,
    pointer: StaticPointer,
    element: ElementAccess,
    index: u64,
    array_length: u64,
) -> Result<Word, Error> {
    check_array_index(machine, index, array_length)?;

    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = pointer.add_bytes(element_offset);

    Ok(Word::static_pointer(pointer))
}
