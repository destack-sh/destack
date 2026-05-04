use super::access;
use super::reference::{check_reference_address_space, check_reference_mutability};
use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{FieldAccess, FieldAccessId, Instruction, Transfer};
use crate::{ReferenceMeta, Word};
use destack_mir as mir;

/// Return one directly encoded field access.
#[inline(always)]
fn instruction_field(machine: &Machine<'_, '_>, field: u32) -> FieldAccess {
    machine.field_access(FieldAccessId(field))
}

/// Publish one checked field address.
#[inline(always)]
fn publish_field_address(
    machine: &mut Machine<'_, '_>,
    dest: mir::Value,
    reference: ReferenceMeta,
    value: Word,
) -> Result<(), Error> {
    check_reference_address_space(machine, reference)?;
    machine.set_word(dest, value);

    Ok(())
}

/// Check one field store destination.
#[inline(always)]
fn check_field_store(machine: &Machine<'_, '_>, reference: ReferenceMeta) -> Result<(), Error> {
    check_reference_address_space(machine, reference)?;
    check_reference_mutability(machine, reference)
}

/// Execute field addr on heap references.
pub(crate) fn execute_address_heap_field(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let base = machine.get_word(mir::Value::new(instruction.b));
    let reference = ReferenceMeta::from_bits(instruction.c as u8);
    let field = instruction_field(machine, instruction.d);
    let value = match access::address_field_heap(
        machine,
        base.as_heap_reference(),
        field,
        field.index,
        field.field_count,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_field_address(machine, dest, reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field addr on shared heap references.
pub(crate) fn execute_address_shared_heap_field(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let base = machine.get_word(mir::Value::new(instruction.b));
    let reference = ReferenceMeta::from_bits(instruction.c as u8);
    let field = instruction_field(machine, instruction.d);

    // compute concrete address
    let value = match access::address_field_shared_heap(
        machine,
        base.as_shared_heap_reference(),
        field,
        field.index,
        field.field_count,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_field_address(machine, dest, reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field addr on raw pointers.
pub(crate) fn execute_address_raw_field(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let base = machine.get_word(mir::Value::new(instruction.b));
    let reference = ReferenceMeta::from_bits(instruction.c as u8);
    let field = instruction_field(machine, instruction.d);
    let value = match access::address_field_raw(
        machine,
        base.as_raw_pointer(),
        field,
        field.index,
        field.field_count,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_field_address(machine, dest, reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field addr on shared raw pointers.
pub(crate) fn execute_address_shared_raw_field(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let base = machine.get_word(mir::Value::new(instruction.b));
    let reference = ReferenceMeta::from_bits(instruction.c as u8);
    let field = instruction_field(machine, instruction.d);

    // compute concrete address
    let value = match access::address_field_shared_raw(
        machine,
        base.as_shared_raw_pointer(),
        field,
        field.index,
        field.field_count,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_field_address(machine, dest, reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field addr on stack pointers.
pub(crate) fn execute_address_stack_field(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let base = machine.get_word(mir::Value::new(instruction.b));
    let reference = ReferenceMeta::from_bits(instruction.c as u8);
    let field = instruction_field(machine, instruction.d);
    let value = match access::address_field_stack(
        machine,
        base.as_stack_pointer(),
        field,
        field.index,
        field.field_count,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_field_address(machine, dest, reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field addr on static pointers.
pub(crate) fn execute_address_static_field(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let base = machine.get_word(mir::Value::new(instruction.b));
    let reference = ReferenceMeta::from_bits(instruction.c as u8);
    let field = instruction_field(machine, instruction.d);
    let value = match access::address_field_static(
        machine,
        base.as_static_pointer(),
        field,
        field.index,
        field.field_count,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_field_address(machine, dest, reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field load on heap references.
pub(crate) fn execute_load_heap_field(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let base = machine.get_word(mir::Value::new(instruction.b));
    let field = instruction_field(machine, instruction.c);
    let value = match access::load_field_heap(
        machine,
        base.as_heap_reference(),
        field,
        field.index,
        field.field_count,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    machine.set_word(dest, value);

    Transfer::Continue
}

/// Execute field load on shared heap references.
pub(crate) fn execute_load_shared_heap_field(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let base = machine.get_word(mir::Value::new(instruction.b));
    let field = instruction_field(machine, instruction.c);

    // load field word
    let value = match access::load_field_shared_heap(
        machine,
        base.as_shared_heap_reference(),
        field,
        field.index,
        field.field_count,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // publish result
    machine.set_word(dest, value);

    Transfer::Continue
}

/// Execute field load on raw pointers.
pub(crate) fn execute_load_raw_field(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let base = machine.get_word(mir::Value::new(instruction.b));
    let field = instruction_field(machine, instruction.c);
    let value = match access::load_field_raw(
        machine,
        base.as_raw_pointer(),
        field,
        field.index,
        field.field_count,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    machine.set_word(dest, value);

    Transfer::Continue
}

/// Execute field load on shared raw pointers.
pub(crate) fn execute_load_shared_raw_field(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let base = machine.get_word(mir::Value::new(instruction.b));
    let field = instruction_field(machine, instruction.c);

    // load field word
    let value = match access::load_field_shared_raw(
        machine,
        base.as_shared_raw_pointer(),
        field,
        field.index,
        field.field_count,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // publish result
    machine.set_word(dest, value);

    Transfer::Continue
}

/// Execute field load on stack pointers.
pub(crate) fn execute_load_stack_field(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let base = machine.get_word(mir::Value::new(instruction.b));
    let field = instruction_field(machine, instruction.c);
    let value = match access::load_field_stack(
        machine,
        base.as_stack_pointer(),
        field,
        field.index,
        field.field_count,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    machine.set_word(dest, value);

    Transfer::Continue
}

/// Execute field load on static pointers.
pub(crate) fn execute_load_static_field(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let base = machine.get_word(mir::Value::new(instruction.b));
    let field = instruction_field(machine, instruction.c);
    let value = match access::load_field_static(
        machine,
        base.as_static_pointer(),
        field,
        field.index,
        field.field_count,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    machine.set_word(dest, value);

    Transfer::Continue
}

/// Execute field store on heap references.
pub(crate) fn execute_store_heap_field(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let base = machine.get_word(mir::Value::new(instruction.a));
    let value = machine.get_word(mir::Value::new(instruction.b));
    let reference = ReferenceMeta::from_bits(instruction.c as u8);
    let field = instruction_field(machine, instruction.d);

    // validate store destination
    if let Err(error) = check_field_store(machine, reference) {
        return Transfer::Error(error);
    }

    if let Err(error) = access::store_field_heap(
        machine,
        base.as_heap_reference(),
        field,
        field.index,
        field.field_count,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field store on shared heap references.
pub(crate) fn execute_store_shared_heap_field(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let base = machine.get_word(mir::Value::new(instruction.a));
    let value = machine.get_word(mir::Value::new(instruction.b));
    let reference = ReferenceMeta::from_bits(instruction.c as u8);
    let field = instruction_field(machine, instruction.d);
    if let Err(error) = check_field_store(machine, reference) {
        return Transfer::Error(error);
    }

    // store field word
    if let Err(error) = access::store_field_shared_heap(
        machine,
        base.as_shared_heap_reference(),
        field,
        field.index,
        field.field_count,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field store on raw pointers.
pub(crate) fn execute_store_raw_field(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let base = machine.get_word(mir::Value::new(instruction.a));
    let value = machine.get_word(mir::Value::new(instruction.b));
    let reference = ReferenceMeta::from_bits(instruction.c as u8);
    let field = instruction_field(machine, instruction.d);

    // validate store destination
    if let Err(error) = check_field_store(machine, reference) {
        return Transfer::Error(error);
    }

    if let Err(error) = access::store_field_raw(
        machine,
        base.as_raw_pointer(),
        field,
        field.index,
        field.field_count,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field store on shared raw pointers.
pub(crate) fn execute_store_shared_raw_field(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let base = machine.get_word(mir::Value::new(instruction.a));
    let value = machine.get_word(mir::Value::new(instruction.b));
    let reference = ReferenceMeta::from_bits(instruction.c as u8);
    let field = instruction_field(machine, instruction.d);
    if let Err(error) = check_field_store(machine, reference) {
        return Transfer::Error(error);
    }

    // store field word
    if let Err(error) = access::store_field_shared_raw(
        machine,
        base.as_shared_raw_pointer(),
        field,
        field.index,
        field.field_count,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field store on stack pointers.
pub(crate) fn execute_store_stack_field(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let base = machine.get_word(mir::Value::new(instruction.a));
    let value = machine.get_word(mir::Value::new(instruction.b));
    let reference = ReferenceMeta::from_bits(instruction.c as u8);
    let field = instruction_field(machine, instruction.d);
    if let Err(error) = check_field_store(machine, reference) {
        return Transfer::Error(error);
    }

    if let Err(error) = access::store_field_stack(
        machine,
        base.as_stack_pointer(),
        field,
        field.index,
        field.field_count,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field store on static pointers.
pub(crate) fn execute_store_static_field(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let base = machine.get_word(mir::Value::new(instruction.a));
    let value = machine.get_word(mir::Value::new(instruction.b));
    let reference = ReferenceMeta::from_bits(instruction.c as u8);
    let field = instruction_field(machine, instruction.d);
    if let Err(error) = check_field_store(machine, reference) {
        return Transfer::Error(error);
    }

    if let Err(error) = access::store_field_static(
        machine,
        base.as_static_pointer(),
        field,
        field.index,
        field.field_count,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}
