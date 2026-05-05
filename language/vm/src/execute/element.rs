use super::address;
use super::reference::check_reference_address_space;
use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{ElementAccess, ElementAccessId, Instruction, Transfer};
use crate::{ReferenceMeta, Word};

/// Load one array index value from a frame byte offset.
#[inline(always)]
pub(super) fn load_array_index_at(machine: &Machine<'_, '_>, index: u32) -> u64 {
    machine.get_word_at(index).as_u64()
}

/// Return one element byte offset.
#[inline(always)]
pub(crate) fn element_byte_offset(index: u64, stride: usize) -> usize {
    index as usize * stride
}

/// Return one directly encoded element access.
#[inline(always)]
fn instruction_element(machine: &Machine<'_, '_>, element: u32) -> ElementAccess {
    machine.element_access(ElementAccessId(element))
}

/// Publish one checked element address.
#[inline(always)]
fn publish_element_address(
    machine: &mut Machine<'_, '_>,
    dest: u32,
    reference: ReferenceMeta,
    value: Word,
) -> Result<(), Error> {
    check_reference_address_space(machine, reference)?;
    machine.set_word_at(dest, value);

    Ok(())
}

/// Execute element addr on heap references.
pub(crate) fn execute_address_heap_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let array = machine.get_word_at(instruction.b);
    let index = load_array_index_at(machine, instruction.c);
    let element = instruction_element(machine, instruction.d);
    let value = match address::element_heap(
        machine,
        array.as_heap_reference(),
        element,
        index,
        element.length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_element_address(machine, dest, element.reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element addr on shared heap references.
pub(crate) fn execute_address_shared_heap_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let array = machine.get_word_at(instruction.b);
    let index = load_array_index_at(machine, instruction.c);
    let element = instruction_element(machine, instruction.d);

    // compute concrete address
    let value = match address::element_shared_heap(
        machine,
        array.as_shared_heap_reference(),
        element,
        index,
        element.length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_element_address(machine, dest, element.reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element addr on raw pointers.
pub(crate) fn execute_address_raw_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let array = machine.get_word_at(instruction.b);
    let index = load_array_index_at(machine, instruction.c);
    let element = instruction_element(machine, instruction.d);
    let value = match address::element_raw(
        machine,
        array.as_raw_pointer(),
        element,
        index,
        element.length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_element_address(machine, dest, element.reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element addr on shared raw pointers.
pub(crate) fn execute_address_shared_raw_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let array = machine.get_word_at(instruction.b);
    let index = load_array_index_at(machine, instruction.c);
    let element = instruction_element(machine, instruction.d);

    // compute concrete address
    let value = match address::element_shared_raw(
        machine,
        array.as_shared_raw_pointer(),
        element,
        index,
        element.length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_element_address(machine, dest, element.reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element addr on stack pointers.
pub(crate) fn execute_address_stack_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let array = machine.get_word_at(instruction.b);
    let index = load_array_index_at(machine, instruction.c);
    let element = instruction_element(machine, instruction.d);
    let value = match address::element_stack(
        machine,
        array.as_stack_pointer(),
        element,
        index,
        element.length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_element_address(machine, dest, element.reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element addr on static pointers.
pub(crate) fn execute_address_static_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let array = machine.get_word_at(instruction.b);
    let index = load_array_index_at(machine, instruction.c);
    let element = instruction_element(machine, instruction.d);
    let value = match address::element_static(
        machine,
        array.as_static_pointer(),
        element,
        index,
        element.length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_element_address(machine, dest, element.reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}
