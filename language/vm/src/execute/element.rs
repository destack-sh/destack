use super::address;
use crate::Word;
use crate::diagnostic::Error;
use crate::machine::Activation;
use crate::program::{Instruction, Projection, ProjectionId};

/// Load one array index value from a frame byte offset.
#[inline(always)]
pub(super) fn load_array_index_at(activation: &Activation<'_>, index: u32) -> u64 {
    activation.load_word_at(index).as_u64()
}

/// Return one element byte offset.
#[inline(always)]
pub(crate) fn element_byte_offset(index: u64, stride: usize) -> usize {
    index as usize * stride
}

/// Return one directly encoded element projection.
#[inline(always)]
fn instruction_element(activation: &Activation<'_>, element: u32) -> Projection {
    activation.projection(ProjectionId(element))
}

/// Publish one element address.
#[inline(always)]
fn publish_element_address(activation: &mut Activation<'_>, dest: u32, value: Word) {
    activation.store_word_at(dest, value);
}

/// Execute element addr on heap references.
pub(crate) fn execute_address_heap_element(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let array = activation.load_word_at(instruction.b);
    let index = load_array_index_at(activation, instruction.c);
    let element = instruction_element(activation, instruction.d);
    let value = address::element_heap(activation, array.as_heap_reference(), element, index);
    publish_element_address(activation, dest, value);

    Ok(())
}

/// Execute element addr on shared heap references.
pub(crate) fn execute_address_shared_heap_element(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let array = activation.load_word_at(instruction.b);
    let index = load_array_index_at(activation, instruction.c);
    let element = instruction_element(activation, instruction.d);

    let value =
        address::element_shared_heap(activation, array.as_shared_heap_reference(), element, index);
    publish_element_address(activation, dest, value);

    Ok(())
}

/// Execute element addr on raw pointers.
pub(crate) fn execute_address_raw_element(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let array = activation.load_word_at(instruction.b);
    let index = load_array_index_at(activation, instruction.c);
    let element = instruction_element(activation, instruction.d);
    let value = address::element_raw(activation, array.as_address(), element, index);
    publish_element_address(activation, dest, value);

    Ok(())
}

/// Execute element addr on stack pointers.
pub(crate) fn execute_address_stack_element(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let array = activation.load_word_at(instruction.b);
    let index = load_array_index_at(activation, instruction.c);
    let element = instruction_element(activation, instruction.d);
    let value = address::element_stack(activation, array.as_stack_pointer(), element, index);
    publish_element_address(activation, dest, value);

    Ok(())
}

/// Execute element addr on frame pointers.
pub(crate) fn execute_address_frame_element(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let array = activation.load_word_at(instruction.b);
    let index = load_array_index_at(activation, instruction.c);
    let element = instruction_element(activation, instruction.d);
    let value = address::element_frame(activation, array.as_frame_pointer(), element, index);
    publish_element_address(activation, dest, value);

    Ok(())
}

/// Execute element addr on static pointers.
pub(crate) fn execute_address_static_element(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let array = activation.load_word_at(instruction.b);
    let index = load_array_index_at(activation, instruction.c);
    let element = instruction_element(activation, instruction.d);
    let value = address::element_static(activation, array.as_static_pointer(), element, index);
    publish_element_address(activation, dest, value);

    Ok(())
}
