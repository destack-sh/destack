use super::address;
use crate::Word;
use crate::diagnostic::Error;
use crate::machine::Activation;
use crate::program::Instruction;

/// Return one directly encoded byte offset.
#[inline(always)]
fn instruction_byte_offset(instruction: &Instruction) -> usize {
    instruction.d as usize
}

/// Publish one offset address.
#[inline(always)]
fn publish_offset_address(activation: &mut Activation<'_>, dest: u32, value: Word) {
    activation.store_word_at(dest, value);
}

/// Execute fixed-offset address on heap references.
pub(crate) fn execute_address_heap_offset(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let base = activation.load_word_at(instruction.b);
    let byte_offset = instruction_byte_offset(instruction);
    let value = address::offset_heap(activation, base.as_heap_reference(), byte_offset);
    publish_offset_address(activation, dest, value);

    Ok(())
}

/// Execute fixed-offset address on shared heap references.
pub(crate) fn execute_address_shared_heap_offset(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let base = activation.load_word_at(instruction.b);
    let byte_offset = instruction_byte_offset(instruction);

    let value =
        address::offset_shared_heap(activation, base.as_shared_heap_reference(), byte_offset);
    publish_offset_address(activation, dest, value);

    Ok(())
}

/// Execute fixed-offset address on raw pointers.
pub(crate) fn execute_address_raw_offset(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let base = activation.load_word_at(instruction.b);
    let byte_offset = instruction_byte_offset(instruction);
    let value = address::offset_raw(activation, base.as_address(), byte_offset);
    publish_offset_address(activation, dest, value);

    Ok(())
}

/// Execute fixed-offset address on stack pointers.
pub(crate) fn execute_address_stack_offset(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let base = activation.load_word_at(instruction.b);
    let byte_offset = instruction_byte_offset(instruction);
    let value = address::offset_stack(base.as_stack_pointer(), byte_offset);
    publish_offset_address(activation, dest, value);

    Ok(())
}

/// Execute fixed-offset address on frame pointers.
pub(crate) fn execute_address_frame_offset(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let base = activation.load_word_at(instruction.b);
    let byte_offset = instruction_byte_offset(instruction);
    let value = address::offset_frame(base.as_frame_pointer(), byte_offset);
    publish_offset_address(activation, dest, value);

    Ok(())
}

/// Execute fixed-offset address on static pointers.
pub(crate) fn execute_address_static_offset(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let base = activation.load_word_at(instruction.b);
    let byte_offset = instruction_byte_offset(instruction);
    let value = address::offset_static(base.as_static_pointer(), byte_offset);
    publish_offset_address(activation, dest, value);

    Ok(())
}
