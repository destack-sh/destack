use super::slice::{load_slice_length_at, store_slice_at};
use crate::diagnostic::{Error, ReferenceKind, ResourceError};
use crate::interpreter::Machine;
use crate::program::{
    AllocationBranch, AllocationSiteId, Edge, Instruction, SliceAllocationBranch,
    SliceProjectionId, SmallAllocationSiteId, Transfer,
};
use crate::{StackPointer, Word};
use destack_heap::{HeapError, HeapReferenceKind};

/// Decode one power-of-two alignment from an instruction field.
fn decode_alignment(alignment_log2: u32) -> usize {
    1usize << alignment_log2
}

/// Return whether one VM error is a catchable allocation failure.
#[inline]
fn is_allocation_failure(error: &Error) -> bool {
    matches!(
        error,
        Error::Resource {
            reason: ResourceError::AllocationFailed | ResourceError::HeapLimitExceeded { .. },
        }
    )
}

/// Record one allocation failure and return the failure edge.
#[cold]
fn record_allocation_failure(machine: &mut Machine<'_, '_>, error: Error, edge: Edge) -> Transfer {
    machine.set_allocation_failure(error);

    Transfer::Jump {
        block: edge.target,
        moves: edge.moves,
    }
}

/// Execute local heap allocation.
#[inline(always)]
pub(crate) fn execute_allocate_heap_zeroed(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let allocation = AllocationSiteId(instruction.b);

    // allocate from the allocation site
    let reference = machine.allocate_zeroed_heap(allocation)?;

    // store result
    machine.store_word_at(dest, Word::heap_reference(reference));

    Ok(())
}

/// Execute local uninitialized heap allocation.
#[inline(always)]
pub(crate) fn execute_allocate_heap_uninit(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let allocation = AllocationSiteId(instruction.b);

    let reference = machine.allocate_uninit_heap(allocation)?;
    machine.store_word_at(dest, Word::heap_reference(reference));

    Ok(())
}

/// Execute local noscan small heap allocation.
#[inline(always)]
pub(crate) fn execute_allocate_heap_small_noscan_zeroed(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let allocation = SmallAllocationSiteId(instruction.b);

    // allocate from the small allocation site
    let reference = machine.allocate_zeroed_heap_small_noscan(allocation)?;

    // store result
    machine.store_word_at(dest, Word::heap_reference(reference));

    Ok(())
}

/// Execute local uninitialized noscan small heap allocation.
#[inline(always)]
pub(crate) fn execute_allocate_heap_small_noscan_uninit(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let allocation = SmallAllocationSiteId(instruction.b);

    let reference = machine.allocate_uninit_heap_small_noscan(allocation)?;
    machine.store_word_at(dest, Word::heap_reference(reference));

    Ok(())
}

/// Execute local scanned small heap allocation.
#[inline(always)]
pub(crate) fn execute_allocate_heap_small_scan_zeroed(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let allocation = SmallAllocationSiteId(instruction.b);

    // allocate from the small allocation site
    let reference = machine.allocate_zeroed_heap_small_scan(allocation)?;

    // store result
    machine.store_word_at(dest, Word::heap_reference(reference));

    Ok(())
}

/// Execute local uninitialized scanned small heap allocation.
#[inline(always)]
pub(crate) fn execute_allocate_heap_small_scan_uninit(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let allocation = SmallAllocationSiteId(instruction.b);

    let reference = machine.allocate_uninit_heap_small_scan(allocation)?;
    machine.store_word_at(dest, Word::heap_reference(reference));

    Ok(())
}

/// Execute local shared-edge small heap allocation.
#[inline(always)]
pub(crate) fn execute_allocate_heap_small_shared_edge_zeroed(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let allocation = SmallAllocationSiteId(instruction.b);

    // allocate from the small allocation site
    let reference = machine.allocate_zeroed_heap_small_shared_edge(allocation)?;

    // store result
    machine.store_word_at(dest, Word::heap_reference(reference));

    Ok(())
}

/// Execute local uninitialized shared-edge small heap allocation.
#[inline(always)]
pub(crate) fn execute_allocate_heap_small_shared_edge_uninit(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let allocation = SmallAllocationSiteId(instruction.b);

    let reference = machine.allocate_uninit_heap_small_shared_edge(allocation)?;
    machine.store_word_at(dest, Word::heap_reference(reference));

    Ok(())
}

/// Execute shared heap allocation.
pub(crate) fn execute_allocate_shared_heap_zeroed(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let allocation = AllocationSiteId(instruction.b);

    // allocate from the allocation site
    let reference = machine.allocate_zeroed_shared_heap(allocation)?;

    // store result
    machine.store_word_at(dest, Word::shared_heap_reference(reference));

    Ok(())
}

/// Execute shared uninitialized heap allocation.
pub(crate) fn execute_allocate_shared_heap_uninit(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let allocation = AllocationSiteId(instruction.b);

    let reference = machine.allocate_uninit_shared_heap(allocation)?;
    machine.store_word_at(dest, Word::shared_heap_reference(reference));

    Ok(())
}

/// Execute shared small heap allocation.
pub(crate) fn execute_allocate_shared_heap_small_zeroed(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let allocation = SmallAllocationSiteId(instruction.b);

    // allocate from the small allocation site
    let reference = machine.allocate_zeroed_shared_heap_small(allocation)?;

    // store result
    machine.store_word_at(dest, Word::shared_heap_reference(reference));

    Ok(())
}

/// Execute shared uninitialized small heap allocation.
pub(crate) fn execute_allocate_shared_heap_small_uninit(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let allocation = SmallAllocationSiteId(instruction.b);

    let reference = machine.allocate_uninit_shared_heap_small(allocation)?;
    machine.store_word_at(dest, Word::shared_heap_reference(reference));

    Ok(())
}

/// Execute one fallible zeroed local heap allocation branch.
pub(crate) fn execute_allocate_heap_zeroed_branch(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_allocate_branch::<false, true>(machine, instruction)
}

/// Execute one fallible uninitialized local heap allocation branch.
pub(crate) fn execute_allocate_heap_uninit_branch(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_allocate_branch::<false, false>(machine, instruction)
}

/// Execute one fallible zeroed shared heap allocation branch.
pub(crate) fn execute_allocate_shared_heap_zeroed_branch(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_allocate_branch::<true, true>(machine, instruction)
}

/// Execute one fallible uninitialized shared heap allocation branch.
pub(crate) fn execute_allocate_shared_heap_uninit_branch(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_allocate_branch::<true, false>(machine, instruction)
}

/// Execute one fallible allocation branch.
#[inline(always)]
fn execute_allocate_branch<const IS_SHARED: bool, const IS_ZEROED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let branch = *machine.side::<AllocationBranch>(instruction);
    machine.clear_allocation_failure();
    let reference = allocate_branch_payload::<IS_SHARED, IS_ZEROED>(machine, branch.allocation);

    match reference {
        Ok(reference) => {
            machine.store_word_at(branch.destination, reference);
            Transfer::Jump {
                block: branch.success.target,
                moves: branch.success.moves,
            }
        }
        Err(error) if is_allocation_failure(&error) => {
            record_allocation_failure(machine, error, branch.failure)
        }
        Err(error) => Transfer::Error(error),
    }
}

/// Allocate one fallible heap payload.
#[inline(always)]
fn allocate_branch_payload<const IS_SHARED: bool, const IS_ZEROED: bool>(
    machine: &mut Machine<'_, '_>,
    allocation: AllocationSiteId,
) -> Result<Word, Error> {
    if IS_SHARED && IS_ZEROED {
        machine
            .allocate_zeroed_shared_heap(allocation)
            .map(Word::shared_heap_reference)
    } else if IS_SHARED {
        machine
            .allocate_uninit_shared_heap(allocation)
            .map(Word::shared_heap_reference)
    } else if IS_ZEROED {
        machine
            .allocate_zeroed_heap(allocation)
            .map(Word::heap_reference)
    } else {
        machine
            .allocate_uninit_heap(allocation)
            .map(Word::heap_reference)
    }
}

/// Execute local slice allocation.
pub(crate) fn execute_allocate_slice_zeroed(
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

/// Execute local uninitialized slice allocation.
pub(crate) fn execute_allocate_slice_uninit(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let length = instruction.b;
    let element = AllocationSiteId(instruction.c);
    let access = SliceProjectionId(instruction.d);
    let access = machine.slice_projection(access);

    let length = load_slice_length_at(machine, length)?;
    let backing_reference = machine
        .allocate_uninit_heap_slice(element, length)
        .map(Word::heap_reference)?;

    store_slice_at(machine, dest, access, backing_reference, length)?;

    Ok(())
}

/// Execute shared slice allocation.
pub(crate) fn execute_allocate_shared_slice_zeroed(
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

/// Execute shared uninitialized slice allocation.
pub(crate) fn execute_allocate_shared_slice_uninit(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let length = instruction.b;
    let element = AllocationSiteId(instruction.c);
    let access = SliceProjectionId(instruction.d);
    let access = machine.slice_projection(access);

    let length = load_slice_length_at(machine, length)?;
    let backing_reference = machine
        .allocate_uninit_shared_heap_slice(element, length)
        .map(Word::shared_heap_reference)?;

    store_slice_at(machine, dest, access, backing_reference, length)?;

    Ok(())
}

/// Execute one fallible zeroed local slice allocation branch.
pub(crate) fn execute_allocate_slice_zeroed_branch(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_allocate_slice_branch::<false, true>(machine, instruction)
}

/// Execute one fallible uninitialized local slice allocation branch.
pub(crate) fn execute_allocate_slice_uninit_branch(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_allocate_slice_branch::<false, false>(machine, instruction)
}

/// Execute one fallible zeroed shared slice allocation branch.
pub(crate) fn execute_allocate_shared_slice_zeroed_branch(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_allocate_slice_branch::<true, true>(machine, instruction)
}

/// Execute one fallible uninitialized shared slice allocation branch.
pub(crate) fn execute_allocate_shared_slice_uninit_branch(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_allocate_slice_branch::<true, false>(machine, instruction)
}

/// Execute one fallible slice allocation branch.
#[inline(always)]
fn execute_allocate_slice_branch<const IS_SHARED: bool, const IS_ZEROED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let branch = *machine.side::<SliceAllocationBranch>(instruction);
    machine.clear_allocation_failure();

    let length = match load_slice_length_at(machine, branch.length) {
        Ok(length) => length,
        Err(error) if is_allocation_failure(&error) => {
            return record_allocation_failure(machine, error, branch.failure);
        }
        Err(error) => return Transfer::Error(error),
    };
    let reference =
        allocate_slice_branch_payload::<IS_SHARED, IS_ZEROED>(machine, branch.element, length);

    match reference {
        Ok(backing_reference) => {
            let access = machine.slice_projection(branch.access);
            if let Err(error) = store_slice_at(
                machine,
                branch.destination,
                access,
                backing_reference,
                length,
            ) {
                return Transfer::Error(error);
            }

            Transfer::Jump {
                block: branch.success.target,
                moves: branch.success.moves,
            }
        }
        Err(error) if is_allocation_failure(&error) => {
            record_allocation_failure(machine, error, branch.failure)
        }
        Err(error) => Transfer::Error(error),
    }
}

/// Allocate one fallible slice backing payload.
#[inline(always)]
fn allocate_slice_branch_payload<const IS_SHARED: bool, const IS_ZEROED: bool>(
    machine: &mut Machine<'_, '_>,
    allocation: AllocationSiteId,
    length: usize,
) -> Result<Word, Error> {
    if IS_SHARED && IS_ZEROED {
        machine
            .allocate_zeroed_shared_heap_slice(allocation, length)
            .map(Word::shared_heap_reference)
    } else if IS_SHARED {
        machine
            .allocate_uninit_shared_heap_slice(allocation, length)
            .map(Word::shared_heap_reference)
    } else if IS_ZEROED {
        machine
            .allocate_zeroed_heap_slice(allocation, length)
            .map(Word::heap_reference)
    } else {
        machine
            .allocate_uninit_heap_slice(allocation, length)
            .map(Word::heap_reference)
    }
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
        Err(HeapError::InvalidReference {
            kind: HeapReferenceKind::Heap,
            ..
        }) => {
            return Err(Error::invalid_reference(ReferenceKind::Heap));
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
        Err(HeapError::InvalidReference {
            kind: HeapReferenceKind::SharedHeap,
            ..
        }) => {
            return Err(Error::invalid_reference(ReferenceKind::SharedHeap));
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
        Err(HeapError::InvalidReference {
            kind: HeapReferenceKind::Heap,
            ..
        }) => {
            return Err(Error::invalid_reference(ReferenceKind::Heap));
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
        Err(HeapError::InvalidReference {
            kind: HeapReferenceKind::Heap,
            ..
        }) => {
            return Err(Error::invalid_reference(ReferenceKind::Heap));
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

/// Execute stack allocation.
pub(crate) fn execute_allocate_stack_zeroed(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let byte_len = instruction.b as u64 | ((instruction.c as u64) << 32);
    let byte_len = byte_len as usize;
    let alignment = decode_alignment(instruction.d);

    // allocate stack bytes from the lowered layout
    let address = machine.allocate_stack_zeroed(byte_len, alignment)?;
    let sp = StackPointer::from_address(address);
    let value = Word::stack_pointer(sp);

    machine.store_word_at(dest, value);

    Ok(())
}

/// Execute uninitialized stack allocation.
pub(crate) fn execute_allocate_stack_uninit(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let byte_len = instruction.b as u64 | ((instruction.c as u64) << 32);
    let byte_len = byte_len as usize;
    let alignment = decode_alignment(instruction.d);

    let address = machine.allocate_stack_uninit(byte_len, alignment)?;
    let sp = StackPointer::from_address(address);
    let value = Word::stack_pointer(sp);

    machine.store_word_at(dest, value);

    Ok(())
}
