use super::address;
use crate::Word;
use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{Instruction, Projection, ProjectionId};

/// Load one array index value from a frame byte offset.
#[inline(always)]
pub(super) fn load_array_index_at(machine: &Machine<'_, '_>, index: u32) -> u64 {
    machine.load_word_at(index).as_u64()
}

/// Return one element byte offset.
#[inline(always)]
pub(crate) fn element_byte_offset(index: u64, stride: usize) -> usize {
    index as usize * stride
}

/// Return one directly encoded element projection.
#[inline(always)]
fn instruction_element(machine: &Machine<'_, '_>, element: u32) -> Projection {
    machine.projection(ProjectionId(element))
}

/// Publish one element address.
#[inline(always)]
fn publish_element_address(machine: &mut Machine<'_, '_>, dest: u32, value: Word) {
    machine.store_word_at(dest, value);
}

/// Execute element addr on heap references.
pub(crate) fn execute_address_heap_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let array = machine.load_word_at(instruction.b);
    let index = load_array_index_at(machine, instruction.c);
    let element = instruction_element(machine, instruction.d);
    let value = address::element_heap(machine, array.as_heap_reference(), element, index);
    publish_element_address(machine, dest, value);

    Ok(())
}

/// Execute element addr on shared heap references.
pub(crate) fn execute_address_shared_heap_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let array = machine.load_word_at(instruction.b);
    let index = load_array_index_at(machine, instruction.c);
    let element = instruction_element(machine, instruction.d);

    let value =
        address::element_shared_heap(machine, array.as_shared_heap_reference(), element, index);
    publish_element_address(machine, dest, value);

    Ok(())
}

/// Execute element addr on raw pointers.
pub(crate) fn execute_address_raw_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let array = machine.load_word_at(instruction.b);
    let index = load_array_index_at(machine, instruction.c);
    let element = instruction_element(machine, instruction.d);
    let value = address::element_raw(machine, array.as_raw_pointer(), element, index);
    publish_element_address(machine, dest, value);

    Ok(())
}

/// Execute element addr on shared raw pointers.
pub(crate) fn execute_address_shared_raw_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let array = machine.load_word_at(instruction.b);
    let index = load_array_index_at(machine, instruction.c);
    let element = instruction_element(machine, instruction.d);

    let value = address::element_shared_raw(machine, array.as_shared_raw_pointer(), element, index);
    publish_element_address(machine, dest, value);

    Ok(())
}

/// Execute element addr on stack pointers.
pub(crate) fn execute_address_stack_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let array = machine.load_word_at(instruction.b);
    let index = load_array_index_at(machine, instruction.c);
    let element = instruction_element(machine, instruction.d);
    let value = address::element_stack(machine, array.as_stack_pointer(), element, index);
    publish_element_address(machine, dest, value);

    Ok(())
}

/// Execute element addr on static pointers.
pub(crate) fn execute_address_static_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let array = machine.load_word_at(instruction.b);
    let index = load_array_index_at(machine, instruction.c);
    let element = instruction_element(machine, instruction.d);
    let value = address::element_static(machine, array.as_static_pointer(), element, index);
    publish_element_address(machine, dest, value);

    Ok(())
}
