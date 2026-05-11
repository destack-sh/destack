use super::slice::{load_slice_length_at, store_slice_at};
use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{AllocationLayoutId, Instruction, SliceProjectionId};
use crate::{StackPointer, Word};
use destack_heap::{AllocationShape, HeapError, Payload, RawAllocationShape, repeated_layout};

/// Decode one power-of-two alignment from an instruction field.
fn decode_alignment(alignment_log2: u32) -> usize {
    1usize << alignment_log2
}

/// Execute local heap allocation.
#[inline(always)]
pub(crate) fn execute_allocate_heap_small_noscan(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let allocation = AllocationLayoutId(instruction.b);
    let slot_bytes = instruction.c as usize;

    // reserve from the active young run, refill on capacity failure
    let reference = match machine.reserve_young(slot_bytes) {
        Some(reference) => reference,
        None => {
            let table = machine.side_table_ptr();
            let allocation = unsafe { (*table).allocation_layout(allocation) };
            let reference_map = unsafe { (*table).reference_map(allocation.reference_map) };
            let class = unsafe { (*table).allocation_class(allocation.class) };

            machine.allocate_zeroed_heap_layout(&allocation.heap_layout(reference_map, class))?
        }
    };

    // store result
    machine.store_word_at(dest, Word::heap_reference(reference));

    Ok(())
}

/// Execute local heap allocation.
pub(crate) fn execute_allocate_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let allocation = AllocationLayoutId(instruction.b);
    let table = machine.side_table_ptr();
    let allocation = unsafe { (*table).allocation_layout(allocation) };
    let reference_map = unsafe { (*table).reference_map(allocation.reference_map) };
    let class = unsafe { (*table).allocation_class(allocation.class) };

    // allocate through the compiled heap layout
    let reference =
        machine.allocate_zeroed_heap_layout(&allocation.heap_layout(reference_map, class))?;

    // store result
    machine.store_word_at(dest, Word::heap_reference(reference));

    Ok(())
}

/// Execute shared heap allocation.
pub(crate) fn execute_allocate_shared_heap_small_noscan(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let allocation = AllocationLayoutId(instruction.b);
    let slot_bytes = instruction.c as usize;
    let bucket_index = instruction.d as usize;

    // reserve from the active worker run, refill on capacity failure
    let reference = match machine.reserve_shared_small(bucket_index, slot_bytes) {
        Some(reference) => reference,
        None => {
            let table = machine.side_table_ptr();
            let allocation = unsafe { (*table).allocation_layout(allocation) };
            let reference_map = unsafe { (*table).reference_map(allocation.reference_map) };
            let class = unsafe { (*table).allocation_class(allocation.class) };

            machine
                .allocate_zeroed_shared_heap_layout(&allocation.heap_layout(reference_map, class))?
        }
    };

    // store result
    machine.store_word_at(dest, Word::shared_heap_reference(reference));

    Ok(())
}

/// Execute shared heap allocation.
pub(crate) fn execute_allocate_shared_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let allocation = AllocationLayoutId(instruction.b);
    let table = machine.side_table_ptr();
    let allocation = unsafe { (*table).allocation_layout(allocation) };
    let reference_map = unsafe { (*table).reference_map(allocation.reference_map) };
    let class = unsafe { (*table).allocation_class(allocation.class) };

    // allocate through the compiled shared heap layout
    let reference = machine
        .allocate_zeroed_shared_heap_layout(&allocation.heap_layout(reference_map, class))?;

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
    let element = AllocationLayoutId(instruction.c);
    let access = SliceProjectionId(instruction.d);
    let table = machine.side_table_ptr();
    let element = unsafe { (*table).allocation_layout(element) };
    let element_reference_map = unsafe { (*table).reference_map(element.reference_map) };
    let access = unsafe { *(*table).slice_projection(access) };

    // load slice length
    let length = load_slice_length_at(machine, length)?;

    // build the backing array allocation shape
    let backing_reference = {
        let element_shape = element.shape(element_reference_map);
        let (byte_len, reference_map) = match repeated_layout(
            element_shape.byte_len,
            element.alignment,
            element_shape.reference_map,
            length,
        ) {
            Ok(layout) => layout,
            Err(error) => return Err(Error::from(error)),
        };
        let shape = AllocationShape::new(byte_len, element.alignment, &reference_map);

        machine
            .allocate_zeroed_heap_shape(shape)
            .map(Word::heap_reference)
    };
    let backing_reference = backing_reference?;

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
    let element = AllocationLayoutId(instruction.c);
    let access = SliceProjectionId(instruction.d);
    let table = machine.side_table_ptr();
    let element = unsafe { (*table).allocation_layout(element) };
    let element_reference_map = unsafe { (*table).reference_map(element.reference_map) };
    let access = unsafe { *(*table).slice_projection(access) };

    // load slice length
    let length = load_slice_length_at(machine, length)?;

    // build the backing array allocation shape
    let backing_reference = {
        let element_shape = element.shape(element_reference_map);
        let (byte_len, reference_map) = match repeated_layout(
            element_shape.byte_len,
            element.alignment,
            element_shape.reference_map,
            length,
        ) {
            Ok(layout) => layout,
            Err(error) => return Err(Error::from(error)),
        };
        let shape = AllocationShape::new(byte_len, element.alignment, &reference_map);

        machine
            .allocate_zeroed_shared_heap_shape(shape)
            .map(Word::shared_heap_reference)
    };
    let backing_reference = backing_reference?;

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
    let pointer = machine.heap_mut().allocate_raw(shape, Payload::Zeroed);
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
    let pointer = machine.shared().allocate_raw(shape, Payload::Zeroed);
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
    let heap = machine.heap_mut();
    match heap.free_raw(pointer) {
        Ok(()) => {}
        Err(HeapError::InvalidRawPointer { .. }) => {
            return Err(Error::InvalidRawPointer);
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
    let heap = machine.heap_mut();
    match heap.pin_heap(reference) {
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
    let heap = machine.heap_mut();
    match heap.unpin_heap(reference) {
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
    match machine.shared().free_raw(pointer) {
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
