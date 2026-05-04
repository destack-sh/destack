use super::access;
use super::element::{element_byte_offset, load_array_index};
use super::reference::check_reference_address_space;
use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{
    Instruction, SliceElementAccess, SliceElementAccessId, Transfer, encode_word_bits,
};
use crate::{ReferenceMeta, Word};
use destack_mir as mir;

/// Load one slice length operand as a host usize.
#[inline(always)]
pub(crate) fn load_slice_length(
    machine: &Machine<'_, '_>,
    value: mir::Value,
) -> Result<usize, Error> {
    let value = machine.get(value);
    let length = value.as_uint();

    usize::try_from(length).map_err(|_| Error::AllocationFailed)
}

/// Return the reference contract for slice data.
fn slice_data_reference(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<ReferenceMeta, Error> {
    let mir::Type::Reference {
        kind,
        address_space,
        mutability,
        is_nullable,
        ..
    } = tree.get(ty)
    else {
        return Err(Error::TypeMismatch {
            expected: "slice data reference".to_string(),
            actual: format!("{ty:?}"),
        });
    };

    Ok(ReferenceMeta::new(
        *kind,
        address_space.clone(),
        *mutability,
        *is_nullable,
    ))
}

/// Encode one slice field.
fn encode_slice_field(
    machine: &Machine<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    value: Word,
    expected_len: usize,
) -> Result<[u8; Word::BYTE_LEN], Error> {
    let (raw, byte_len) = encode_word_bits(machine.tree(), ty, value)?;
    if byte_len != expected_len {
        return Err(Error::InvalidInstruction);
    }

    Ok(raw.to_le_bytes())
}

/// Store one slice descriptor into a frame value.
pub(crate) fn store_slice(
    machine: &mut Machine<'_, '_>,
    dest: mir::Value,
    data: Word,
    length: usize,
) -> Result<(), Error> {
    let ty = machine.value_type(dest)?;
    let layout = machine.layout(ty)?.clone();
    let slice = layout.slice().ok_or(Error::InvalidInstruction)?;
    let data_reference = slice_data_reference(machine.tree(), slice.data.ty)?;

    // validate the backing pointer against the descriptor type
    check_reference_address_space(machine, data_reference)?;

    // encode fields before borrowing destination bytes
    let length = Word::uint(length as u64, usize::BITS as u8);
    let data_bytes = encode_slice_field(machine, slice.data.ty, data, slice.data.byte_len)?;
    let length_bytes = encode_slice_field(machine, slice.length.ty, length, slice.length.byte_len)?;

    // store the concrete descriptor layout
    let destination = machine.value_bytes_mut(dest)?;
    let data_end = slice.data.offset + slice.data.byte_len;
    let length_end = slice.length.offset + slice.length.byte_len;

    destination
        .get_mut(slice.data.offset..data_end)
        .ok_or(Error::InvalidInstruction)?
        .copy_from_slice(&data_bytes[..slice.data.byte_len]);
    destination
        .get_mut(slice.length.offset..length_end)
        .ok_or(Error::InvalidInstruction)?
        .copy_from_slice(&length_bytes[..slice.length.byte_len]);

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
    let data = access::load_frame_word(machine, slice, access.data.into())?;
    let length = access::load_frame_word(machine, slice, access.length.into())?.as_uint();

    Ok((data, length))
}

/// Store a computed slice element address.
#[inline(always)]
fn store_slice_element_address(
    machine: &mut Machine<'_, '_>,
    dest: mir::Value,
    reference: ReferenceMeta,
    value: Word,
) -> Result<(), Error> {
    check_reference_address_space(machine, reference)?;
    machine.set_word(dest, value);

    Ok(())
}

/// Return one lowered slice element access.
#[inline(always)]
fn instruction_slice_element(machine: &Machine<'_, '_>, access: u32) -> SliceElementAccess {
    machine.slice_element_access(SliceElementAccessId(access))
}

/// Load slice address operands.
#[inline(always)]
fn slice_address_operands(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(mir::Value, Word, u64, SliceElementAccess), Error> {
    let dest = mir::Value::new(instruction.a);
    let slice = mir::Value::new(instruction.b);
    let index = mir::Value::new(instruction.c);
    let access = instruction_slice_element(machine, instruction.d);

    let slice = machine.value_operand(slice)?;
    let index = load_array_index(machine, index);

    Ok((dest, slice, index, access))
}

/// Execute slice element addr on local heap references.
pub(crate) fn execute_address_heap_slice_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, slice, index, access) = match slice_address_operands(machine, instruction) {
        Ok(operands) => operands,
        Err(error) => return Transfer::Error(error),
    };

    let (data, length) = match load_slice_descriptor(machine, slice, access) {
        Ok(descriptor) => descriptor,
        Err(error) => return Transfer::Error(error),
    };
    let value = match access::address_element_heap(
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
    let (dest, slice, index, access) = match slice_address_operands(machine, instruction) {
        Ok(operands) => operands,
        Err(error) => return Transfer::Error(error),
    };

    let (data, length) = match load_slice_descriptor(machine, slice, access) {
        Ok(descriptor) => descriptor,
        Err(error) => return Transfer::Error(error),
    };
    let value = match access::address_element_shared_heap(
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
    let (dest, slice, index, access) = match slice_address_operands(machine, instruction) {
        Ok(operands) => operands,
        Err(error) => return Transfer::Error(error),
    };

    let (data, length) = match load_slice_descriptor(machine, slice, access) {
        Ok(descriptor) => descriptor,
        Err(error) => return Transfer::Error(error),
    };
    let value = match access::address_element_raw(
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
    let (dest, slice, index, access) = match slice_address_operands(machine, instruction) {
        Ok(operands) => operands,
        Err(error) => return Transfer::Error(error),
    };

    let (data, length) = match load_slice_descriptor(machine, slice, access) {
        Ok(descriptor) => descriptor,
        Err(error) => return Transfer::Error(error),
    };
    let value = match access::address_element_shared_raw(
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
    let (dest, slice, index, access) = match slice_address_operands(machine, instruction) {
        Ok(operands) => operands,
        Err(error) => return Transfer::Error(error),
    };

    let (data, length) = match load_slice_descriptor(machine, slice, access) {
        Ok(descriptor) => descriptor,
        Err(error) => return Transfer::Error(error),
    };
    let value = match access::address_element_stack(
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
    let (dest, slice, index, access) = match slice_address_operands(machine, instruction) {
        Ok(operands) => operands,
        Err(error) => return Transfer::Error(error),
    };

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
    let (dest, slice, index, access) = match slice_address_operands(machine, instruction) {
        Ok(operands) => operands,
        Err(error) => return Transfer::Error(error),
    };

    let (data, length) = match load_slice_descriptor(machine, slice, access) {
        Ok(descriptor) => descriptor,
        Err(error) => return Transfer::Error(error),
    };
    let value = match access::address_element_static(
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
