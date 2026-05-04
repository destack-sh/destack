use super::access;
use super::reference::{check_reference_address_space, check_reference_mutability};
use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{
    ElementAccess, ElementAccessId, Instruction, PointerClass, Transfer, word_layout_from_type,
};
use crate::{ReferenceMeta, Word};
use destack_mir as mir;

/// Load one array index operand as an unsigned value.
#[inline(always)]
pub(super) fn load_array_index(machine: &Machine<'_, '_>, index: mir::Value) -> u64 {
    machine.get_word(index).as_u64()
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

/// Return one lowered element access for an already known indexed type.
#[inline(always)]
pub(crate) fn element_access_for_type(
    machine: &Machine<'_, '_>,
    indexed_type: mir::LocalNodeId<mir::Type>,
    index: u64,
) -> Result<(ElementAccess, u64), Error> {
    let layout = machine.layout(indexed_type)?;
    let element_count = layout.element_count().ok_or(Error::InvalidInstruction)? as u64;
    let element = layout.element().ok_or(Error::InvalidArrayAccess {
        index,
        length: element_count,
    })?;

    if index >= element_count {
        return Err(Error::InvalidArrayAccess {
            index,
            length: element_count,
        });
    }

    let access = ElementAccess {
        pointer_class: PointerClass::Frame,
        reference: ReferenceMeta::NONE,
        value_type: element.ty,
        length: element_count,
        byte_stride: element.stride,
        byte_len: element.byte_len,
        word_layout: word_layout_from_type(machine.tree(), element.ty),
    };

    Ok((access, element_count))
}

/// Publish one checked element address.
#[inline(always)]
fn publish_element_address(
    machine: &mut Machine<'_, '_>,
    dest: mir::Value,
    reference: ReferenceMeta,
    value: Word,
) -> Result<(), Error> {
    check_reference_address_space(machine, reference)?;
    machine.set_word(dest, value);

    Ok(())
}

/// Check one element store destination.
#[inline(always)]
fn check_element_store(machine: &Machine<'_, '_>, reference: ReferenceMeta) -> Result<(), Error> {
    check_reference_address_space(machine, reference)?;
    check_reference_mutability(machine, reference)
}

/// Execute element addr on heap references.
pub(crate) fn execute_address_heap_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let array = machine.get_word(mir::Value::new(instruction.b));
    let index = load_array_index(machine, mir::Value::new(instruction.c));
    let element = instruction_element(machine, instruction.d);
    let value = match access::address_element_heap(
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
    let dest = mir::Value::new(instruction.a);
    let array = machine.get_word(mir::Value::new(instruction.b));
    let index = load_array_index(machine, mir::Value::new(instruction.c));
    let element = instruction_element(machine, instruction.d);

    // compute concrete address
    let value = match access::address_element_shared_heap(
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
    let dest = mir::Value::new(instruction.a);
    let array = machine.get_word(mir::Value::new(instruction.b));
    let index = load_array_index(machine, mir::Value::new(instruction.c));
    let element = instruction_element(machine, instruction.d);
    let value = match access::address_element_raw(
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
    let dest = mir::Value::new(instruction.a);
    let array = machine.get_word(mir::Value::new(instruction.b));
    let index = load_array_index(machine, mir::Value::new(instruction.c));
    let element = instruction_element(machine, instruction.d);

    // compute concrete address
    let value = match access::address_element_shared_raw(
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
    let dest = mir::Value::new(instruction.a);
    let array = machine.get_word(mir::Value::new(instruction.b));
    let index = load_array_index(machine, mir::Value::new(instruction.c));
    let element = instruction_element(machine, instruction.d);
    let value = match access::address_element_stack(
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
    let dest = mir::Value::new(instruction.a);
    let array = machine.get_word(mir::Value::new(instruction.b));
    let index = load_array_index(machine, mir::Value::new(instruction.c));
    let element = instruction_element(machine, instruction.d);
    let value = match access::address_element_static(
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

/// Execute element load on heap references.
pub(crate) fn execute_load_heap_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let array = machine.get_word(mir::Value::new(instruction.b));
    let index = load_array_index(machine, mir::Value::new(instruction.c));
    let element = instruction_element(machine, instruction.d);
    let value = match access::load_element_heap(
        machine,
        array.as_heap_reference(),
        element,
        index,
        element.length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    machine.set_word(dest, value);

    Transfer::Continue
}

/// Execute element load on shared heap references.
pub(crate) fn execute_load_shared_heap_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let array = machine.get_word(mir::Value::new(instruction.b));
    let index = load_array_index(machine, mir::Value::new(instruction.c));
    let element = instruction_element(machine, instruction.d);

    // load element word
    let value = match access::load_element_shared_heap(
        machine,
        array.as_shared_heap_reference(),
        element,
        index,
        element.length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // publish result
    machine.set_word(dest, value);

    Transfer::Continue
}

/// Execute element load on raw pointers.
pub(crate) fn execute_load_raw_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let array = machine.get_word(mir::Value::new(instruction.b));
    let index = load_array_index(machine, mir::Value::new(instruction.c));
    let element = instruction_element(machine, instruction.d);
    let value = match access::load_element_raw(
        machine,
        array.as_raw_pointer(),
        element,
        index,
        element.length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    machine.set_word(dest, value);

    Transfer::Continue
}

/// Execute element load on shared raw pointers.
pub(crate) fn execute_load_shared_raw_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let array = machine.get_word(mir::Value::new(instruction.b));
    let index = load_array_index(machine, mir::Value::new(instruction.c));
    let element = instruction_element(machine, instruction.d);

    // load element word
    let value = match access::load_element_shared_raw(
        machine,
        array.as_shared_raw_pointer(),
        element,
        index,
        element.length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // publish result
    machine.set_word(dest, value);

    Transfer::Continue
}

/// Execute element load on stack pointers.
pub(crate) fn execute_load_stack_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let array = machine.get_word(mir::Value::new(instruction.b));
    let index = load_array_index(machine, mir::Value::new(instruction.c));
    let element = instruction_element(machine, instruction.d);
    let value = match access::load_element_stack(
        machine,
        array.as_stack_pointer(),
        element,
        index,
        element.length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    machine.set_word(dest, value);

    Transfer::Continue
}

/// Execute element load on static pointers.
pub(crate) fn execute_load_static_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let array = machine.get_word(mir::Value::new(instruction.b));
    let index = load_array_index(machine, mir::Value::new(instruction.c));
    let element = instruction_element(machine, instruction.d);
    let value = match access::load_element_static(
        machine,
        array.as_static_pointer(),
        element,
        index,
        element.length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    machine.set_word(dest, value);

    Transfer::Continue
}

/// Execute element store on heap references.
pub(crate) fn execute_store_heap_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let array = machine.get_word(mir::Value::new(instruction.a));
    let index = load_array_index(machine, mir::Value::new(instruction.b));
    let value = machine.get_word(mir::Value::new(instruction.c));
    let element = instruction_element(machine, instruction.d);

    // validate store destination
    if let Err(error) = check_element_store(machine, element.reference) {
        return Transfer::Error(error);
    }

    if let Err(error) = access::store_element_heap(
        machine,
        array.as_heap_reference(),
        element,
        index,
        element.length,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element store on shared heap references.
pub(crate) fn execute_store_shared_heap_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let array = machine.get_word(mir::Value::new(instruction.a));
    let index = load_array_index(machine, mir::Value::new(instruction.b));
    let value = machine.get_word(mir::Value::new(instruction.c));
    let element = instruction_element(machine, instruction.d);
    if let Err(error) = check_element_store(machine, element.reference) {
        return Transfer::Error(error);
    }

    // store element word
    if let Err(error) = access::store_element_shared_heap(
        machine,
        array.as_shared_heap_reference(),
        element,
        index,
        element.length,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element store on raw pointers.
pub(crate) fn execute_store_raw_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let array = machine.get_word(mir::Value::new(instruction.a));
    let index = load_array_index(machine, mir::Value::new(instruction.b));
    let value = machine.get_word(mir::Value::new(instruction.c));
    let element = instruction_element(machine, instruction.d);

    // validate store destination
    if let Err(error) = check_element_store(machine, element.reference) {
        return Transfer::Error(error);
    }

    if let Err(error) = access::store_element_raw(
        machine,
        array.as_raw_pointer(),
        element,
        index,
        element.length,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element store on shared raw pointers.
pub(crate) fn execute_store_shared_raw_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let array = machine.get_word(mir::Value::new(instruction.a));
    let index = load_array_index(machine, mir::Value::new(instruction.b));
    let value = machine.get_word(mir::Value::new(instruction.c));
    let element = instruction_element(machine, instruction.d);
    if let Err(error) = check_element_store(machine, element.reference) {
        return Transfer::Error(error);
    }

    // store element word
    if let Err(error) = access::store_element_shared_raw(
        machine,
        array.as_shared_raw_pointer(),
        element,
        index,
        element.length,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element store on stack pointers.
pub(crate) fn execute_store_stack_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let array = machine.get_word(mir::Value::new(instruction.a));
    let index = load_array_index(machine, mir::Value::new(instruction.b));
    let value = machine.get_word(mir::Value::new(instruction.c));
    let element = instruction_element(machine, instruction.d);
    if let Err(error) = check_element_store(machine, element.reference) {
        return Transfer::Error(error);
    }

    if let Err(error) = access::store_element_stack(
        machine,
        array.as_stack_pointer(),
        element,
        index,
        element.length,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element store on static pointers.
pub(crate) fn execute_store_static_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let array = machine.get_word(mir::Value::new(instruction.a));
    let index = load_array_index(machine, mir::Value::new(instruction.b));
    let value = machine.get_word(mir::Value::new(instruction.c));
    let element = instruction_element(machine, instruction.d);
    if let Err(error) = check_element_store(machine, element.reference) {
        return Transfer::Error(error);
    }

    if let Err(error) = access::store_element_static(
        machine,
        array.as_static_pointer(),
        element,
        index,
        element.length,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}
