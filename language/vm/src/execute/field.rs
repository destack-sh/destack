use super::address;
use super::reference::check_reference_address_space;
use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{Instruction, Transfer};
use crate::{ReferenceMeta, Word};

/// Return one directly encoded byte offset.
#[inline(always)]
fn instruction_byte_offset(instruction: &Instruction) -> usize {
    instruction.d as usize
}

/// Publish one checked offset address.
#[inline(always)]
fn publish_offset_address(
    machine: &mut Machine<'_, '_>,
    dest: u32,
    reference: ReferenceMeta,
    value: Word,
) -> Result<(), Error> {
    check_reference_address_space(machine, reference)?;
    machine.set_word_at(dest, value);

    Ok(())
}

/// Execute fixed-offset address on heap references.
pub(crate) fn execute_address_heap_offset(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let base = machine.get_word_at(instruction.b);
    let reference = ReferenceMeta::from_bits(instruction.c as u8);
    let byte_offset = instruction_byte_offset(instruction);
    let value = match address::offset_heap(machine, base.as_heap_reference(), byte_offset) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_offset_address(machine, dest, reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute fixed-offset address on shared heap references.
pub(crate) fn execute_address_shared_heap_offset(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let base = machine.get_word_at(instruction.b);
    let reference = ReferenceMeta::from_bits(instruction.c as u8);
    let byte_offset = instruction_byte_offset(instruction);

    // compute concrete address
    let value =
        match address::offset_shared_heap(machine, base.as_shared_heap_reference(), byte_offset) {
            Ok(value) => value,
            Err(error) => return Transfer::Error(error),
        };
    if let Err(error) = publish_offset_address(machine, dest, reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute fixed-offset address on raw pointers.
pub(crate) fn execute_address_raw_offset(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let base = machine.get_word_at(instruction.b);
    let reference = ReferenceMeta::from_bits(instruction.c as u8);
    let byte_offset = instruction_byte_offset(instruction);
    let value = match address::offset_raw(machine, base.as_raw_pointer(), byte_offset) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_offset_address(machine, dest, reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute fixed-offset address on shared raw pointers.
pub(crate) fn execute_address_shared_raw_offset(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let base = machine.get_word_at(instruction.b);
    let reference = ReferenceMeta::from_bits(instruction.c as u8);
    let byte_offset = instruction_byte_offset(instruction);

    // compute concrete address
    let value = match address::offset_shared_raw(machine, base.as_shared_raw_pointer(), byte_offset)
    {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_offset_address(machine, dest, reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute fixed-offset address on stack pointers.
pub(crate) fn execute_address_stack_offset(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let base = machine.get_word_at(instruction.b);
    let reference = ReferenceMeta::from_bits(instruction.c as u8);
    let byte_offset = instruction_byte_offset(instruction);
    let value = address::offset_stack(base.as_stack_pointer(), byte_offset);
    if let Err(error) = publish_offset_address(machine, dest, reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute fixed-offset address on static pointers.
pub(crate) fn execute_address_static_offset(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let base = machine.get_word_at(instruction.b);
    let reference = ReferenceMeta::from_bits(instruction.c as u8);
    let byte_offset = instruction_byte_offset(instruction);
    let value = address::offset_static(base.as_static_pointer(), byte_offset);
    if let Err(error) = publish_offset_address(machine, dest, reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}
