use super::element::{element_byte_offset, load_array_index_at};
use super::reference::check_reference_address_space;
use super::{access, address};
use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{Instruction, SliceElementAccess, SliceElementAccessId, Transfer};
use crate::{ReferenceMeta, Word};

/// Load one slice length value as a host usize.
#[inline(always)]
pub(crate) fn load_slice_length_at(
    machine: &Machine<'_, '_>,
    value_offset: u32,
) -> Result<usize, Error> {
    let value = machine.get_word_at(value_offset);
    let length = value.as_uint();

    usize::try_from(length).map_err(|_| Error::AllocationFailed)
}

/// Store one slice descriptor into a frame value.
pub(crate) fn store_slice_at(
    machine: &mut Machine<'_, '_>,
    dest: u32,
    access: SliceElementAccess,
    data: Word,
    length: usize,
) -> Result<(), Error> {
    // validate the backing pointer against the descriptor type
    check_reference_address_space(machine, access.reference)?;

    // write the two descriptor fields through their lowered layouts
    let length = Word::uint(length as u64, usize::BITS as u8);
    let pointer = machine.frame_pointer_at(dest);
    access::store_frame_scalar_by_layout(machine, pointer, access.data.into(), data)?;
    access::store_frame_scalar_by_layout(machine, pointer, access.length.into(), length)?;

    Ok(())
}

/// Load the data pointer and length from one slice descriptor.
#[inline(always)]
fn load_slice_descriptor(
    machine: &mut Machine<'_, '_>,
    slice: Word,
    access: SliceElementAccess,
) -> Result<(Word, u64), Error> {
    let slice = slice.as_frame_pointer();
    let data = access::load_frame_scalar_by_layout(machine, slice, access.data.into())?;
    let length =
        access::load_frame_scalar_by_layout(machine, slice, access.length.into())?.as_uint();

    Ok((data, length))
}

/// Store a computed slice element address.
#[inline(always)]
fn store_slice_element_address(
    machine: &mut Machine<'_, '_>,
    dest: u32,
    reference: ReferenceMeta,
    value: Word,
) -> Result<(), Error> {
    check_reference_address_space(machine, reference)?;
    machine.set_word_at(dest, value);

    Ok(())
}

/// Return one lowered slice element access.
#[inline(always)]
fn instruction_slice_element(machine: &Machine<'_, '_>, access: u32) -> SliceElementAccess {
    machine.slice_element_access(SliceElementAccessId(access))
}

/// Load slice address instruction fields.
#[inline(always)]
fn slice_address_fields(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> (u32, Word, u64, SliceElementAccess) {
    let dest = instruction.a;
    let slice = Word::frame_pointer(machine.frame_pointer_at(instruction.b));
    let index = instruction.c;
    let access = instruction_slice_element(machine, instruction.d);

    let index = load_array_index_at(machine, index);

    (dest, slice, index, access)
}

/// Execute slice element addr on local heap references.
pub(crate) fn execute_address_heap_slice_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, slice, index, access) = slice_address_fields(machine, instruction);

    let (data, length) = match load_slice_descriptor(machine, slice, access) {
        Ok(descriptor) => descriptor,
        Err(error) => return Transfer::Error(error),
    };
    let value = match address::element_heap(
        machine,
        data.as_heap_reference(),
        access.element,
        index,
        length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    if let Err(error) = store_slice_element_address(machine, dest, access.reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute slice element addr on shared heap references.
pub(crate) fn execute_address_shared_heap_slice_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, slice, index, access) = slice_address_fields(machine, instruction);

    let (data, length) = match load_slice_descriptor(machine, slice, access) {
        Ok(descriptor) => descriptor,
        Err(error) => return Transfer::Error(error),
    };
    let value = match address::element_shared_heap(
        machine,
        data.as_shared_heap_reference(),
        access.element,
        index,
        length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    if let Err(error) = store_slice_element_address(machine, dest, access.reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute slice element addr on local raw pointers.
pub(crate) fn execute_address_raw_slice_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, slice, index, access) = slice_address_fields(machine, instruction);

    let (data, length) = match load_slice_descriptor(machine, slice, access) {
        Ok(descriptor) => descriptor,
        Err(error) => return Transfer::Error(error),
    };
    let value = match address::element_raw(
        machine,
        data.as_raw_pointer(),
        access.element,
        index,
        length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    if let Err(error) = store_slice_element_address(machine, dest, access.reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute slice element addr on shared raw pointers.
pub(crate) fn execute_address_shared_raw_slice_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, slice, index, access) = slice_address_fields(machine, instruction);

    let (data, length) = match load_slice_descriptor(machine, slice, access) {
        Ok(descriptor) => descriptor,
        Err(error) => return Transfer::Error(error),
    };
    let value = match address::element_shared_raw(
        machine,
        data.as_shared_raw_pointer(),
        access.element,
        index,
        length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    if let Err(error) = store_slice_element_address(machine, dest, access.reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute slice element addr on stack pointers.
pub(crate) fn execute_address_stack_slice_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, slice, index, access) = slice_address_fields(machine, instruction);

    let (data, length) = match load_slice_descriptor(machine, slice, access) {
        Ok(descriptor) => descriptor,
        Err(error) => return Transfer::Error(error),
    };
    let value = match address::element_stack(
        machine,
        data.as_stack_pointer(),
        access.element,
        index,
        length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    if let Err(error) = store_slice_element_address(machine, dest, access.reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute slice element addr on frame pointers.
pub(crate) fn execute_address_frame_slice_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, slice, index, access) = slice_address_fields(machine, instruction);

    let (data, length) = match load_slice_descriptor(machine, slice, access) {
        Ok(descriptor) => descriptor,
        Err(error) => return Transfer::Error(error),
    };
    if machine.bounds_checks && index >= length {
        return Transfer::Error(Error::InvalidArrayAccess { index, length });
    }

    let offset = element_byte_offset(index, access.element.byte_stride);
    let pointer = data.as_frame_pointer().add_bytes(offset);
    let value = Word::frame_pointer(pointer);

    if let Err(error) = store_slice_element_address(machine, dest, access.reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute slice element addr on static pointers.
pub(crate) fn execute_address_static_slice_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, slice, index, access) = slice_address_fields(machine, instruction);

    let (data, length) = match load_slice_descriptor(machine, slice, access) {
        Ok(descriptor) => descriptor,
        Err(error) => return Transfer::Error(error),
    };
    let value = match address::element_static(
        machine,
        data.as_static_pointer(),
        access.element,
        index,
        length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    if let Err(error) = store_slice_element_address(machine, dest, access.reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}
