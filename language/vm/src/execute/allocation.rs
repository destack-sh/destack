use super::slice::{load_slice_length_at, store_slice_at};
use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{AllocationSiteId, Instruction, SliceProjectionId};
use crate::{StackPointer, Word};
use destack_heap::{HeapError, RawAllocationShape};

/// Decode one power-of-two alignment from an instruction field.
fn decode_alignment(alignment_log2: u32) -> usize {
    1usize << alignment_log2
}

/// Execute local heap allocation.
#[inline(always)]
pub(crate) fn execute_allocate_heap_site(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let allocation = AllocationSiteId(instruction.b);

    // allocate through the compiled heap layout
    let reference = machine.allocate_zeroed_heap_site(allocation)?;

    // store result
    machine.store_word_at(dest, Word::heap_reference(reference));

    Ok(())
}

/// Execute shared heap allocation.
pub(crate) fn execute_allocate_shared_heap_site(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let allocation = AllocationSiteId(instruction.b);

    // allocate through the compiled shared heap layout
    let reference = machine.allocate_zeroed_shared_heap_site(allocation)?;

    // store result
    machine.store_word_at(dest, Word::shared_heap_reference(reference));

    Ok(())
}

/// Execute local slice allocation.
pub(crate) fn execute_allocate_slice(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // decode fixed fields
    let dest = instruction.a;
    let length = instruction.b;
    let element = AllocationSiteId(instruction.c);
    let access = SliceProjectionId(instruction.d);
    let access = machine.slice_projection(access);

    // load slice length
    let length = load_slice_length_at(machine, length)?;

    // build the backing array allocation shape
    let backing_reference = machine
        .allocate_zeroed_heap_slice(element, length)
        .map(Word::heap_reference)?;

    // write the slice descriptor
    store_slice_at(machine, dest, access, backing_reference, length)?;

    Ok(())
}

/// Execute shared slice allocation.
pub(crate) fn execute_allocate_shared_slice(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // decode fixed fields
    let dest = instruction.a;
    let length = instruction.b;
    let element = AllocationSiteId(instruction.c);
    let access = SliceProjectionId(instruction.d);
    let access = machine.slice_projection(access);

    // load slice length
    let length = load_slice_length_at(machine, length)?;

    // build the backing array allocation shape
    let backing_reference = machine
        .allocate_zeroed_shared_heap_slice(element, length)
        .map(Word::shared_heap_reference)?;

    // write the slice descriptor
    store_slice_at(machine, dest, access, backing_reference, length)?;

    Ok(())
}

/// Execute raw allocation.
pub(crate) fn execute_allocate_raw(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let byte_len = instruction.b as u64 | ((instruction.c as u64) << 32);
    let byte_len = byte_len as usize;
    let alignment = decode_alignment(instruction.d);
    let shape = RawAllocationShape::new(byte_len, alignment);

    // allocate raw heap bytes
    let pointer = machine.allocate_raw_zeroed(shape);
    let pointer = match pointer {
        Ok(pointer) => pointer,
        Err(error) => return Err(Error::from(error)),
    };
    let value = Word::raw_pointer(pointer);

    machine.store_word_at(dest, value);

    Ok(())
}

/// Execute shared raw allocation.
pub(crate) fn execute_allocate_shared_raw(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let byte_len = instruction.b as u64 | ((instruction.c as u64) << 32);
    let byte_len = byte_len as usize;
    let alignment = decode_alignment(instruction.d);
    let shape = RawAllocationShape::new(byte_len, alignment);

    // allocate shared raw heap bytes
    let pointer = machine.allocate_shared_raw_zeroed(shape);
    let pointer = match pointer {
        Ok(pointer) => pointer,
        Err(error) => return Err(Error::from(error)),
    };
    let value = Word::shared_raw_pointer(pointer);

    machine.store_word_at(dest, value);

    Ok(())
}

/// Execute raw free.
pub(crate) fn execute_free_raw(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let pointer = instruction.a;

    // free the pointed raw allocation
    let pointer = machine.load_word_at(pointer).as_raw_pointer();
    match machine.free_raw(pointer) {
        Ok(()) => {}
        Err(HeapError::InvalidRawPointer { .. }) => {
            return Err(Error::InvalidRawPointer);
        }
        Err(error) => return Err(Error::from(error)),
    }

    Ok(())
}

/// Execute unique heap free.
pub(crate) fn execute_free_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let reference = instruction.a;

    // free the unique heap allocation
    let reference = machine.load_word_at(reference).as_heap_reference();
    match machine.free_heap(reference) {
        Ok(()) => {}
        Err(HeapError::InvalidHeapReference { .. }) => {
            return Err(Error::InvalidHeapReference);
        }
        Err(error) => return Err(Error::from(error)),
    }

    Ok(())
}

/// Execute unique shared heap free.
pub(crate) fn execute_free_shared_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let reference = instruction.a;

    // free the unique shared heap allocation
    let reference = machine.load_word_at(reference).as_shared_heap_reference();
    match machine.free_shared_heap(reference) {
        Ok(()) => {}
        Err(HeapError::InvalidSharedHeapReference { .. }) => {
            return Err(Error::InvalidSharedHeapReference);
        }
        Err(error) => return Err(Error::from(error)),
    }

    Ok(())
}

/// Execute local heap pin.
pub(crate) fn execute_pin_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let value = instruction.b;

    // pin the local heap reference
    let reference = machine.load_word_at(value).as_heap_reference();
    match machine.pin_heap(reference) {
        Ok(reference) => machine.store_word_at(dest, Word::heap_reference(reference)),
        Err(HeapError::InvalidHeapReference { .. }) => {
            return Err(Error::InvalidHeapReference);
        }
        Err(error) => return Err(Error::from(error)),
    }

    Ok(())
}

/// Execute shared heap pin.
pub(crate) fn execute_pin_shared_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let value = instruction.b;

    // shared heap references are already stable
    let reference = machine.load_word_at(value).as_shared_heap_reference();
    machine.store_word_at(dest, Word::shared_heap_reference(reference));

    Ok(())
}

/// Execute local heap unpin.
pub(crate) fn execute_unpin_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let value = instruction.a;

    // release the local heap pin
    let reference = machine.load_word_at(value).as_heap_reference();
    match machine.unpin_heap(reference) {
        Ok(()) => {}
        Err(HeapError::InvalidHeapReference { .. }) => {
            return Err(Error::InvalidHeapReference);
        }
        Err(error) => return Err(Error::from(error)),
    }

    Ok(())
}

/// Execute shared heap unpin.
pub(crate) fn execute_unpin_shared_heap(
    _machine: &mut Machine<'_, '_>,
    _instruction: &Instruction,
) -> Result<(), Error> {
    // shared heap pins do not need worker-local release
    Ok(())
}

/// Execute shared raw free.
pub(crate) fn execute_free_shared_raw(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let pointer = instruction.a;

    // free the pointed shared raw allocation
    let pointer = machine.load_word_at(pointer).as_shared_raw_pointer();
    match machine.free_shared_raw(pointer) {
        Ok(()) => {}
        Err(HeapError::InvalidSharedRawPointer { .. }) => {
            return Err(Error::InvalidSharedRawPointer);
        }
        Err(error) => return Err(Error::from(error)),
    }

    Ok(())
}

/// Execute stack allocation.
pub(crate) fn execute_allocate_stack(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let byte_len = instruction.b as u64 | ((instruction.c as u64) << 32);
    let byte_len = byte_len as usize;
    let alignment = decode_alignment(instruction.d);

    // allocate stack bytes from the lowered layout
    let address = machine.allocate_stack(byte_len, alignment)?;
    let sp = StackPointer::from_address(address);
    let value = Word::stack_pointer(sp);

    machine.store_word_at(dest, value);

    Ok(())
}
