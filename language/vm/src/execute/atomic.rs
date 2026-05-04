use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{
    Instruction, PointerClass, Transfer, pointer_class_from_reference, repr_type,
};
use destack_mir as mir;

/// Return one atomic read modify write operator from an side record.
fn atomic_rmw_operator_from_operand(operand: u32) -> Result<mir::AtomicRmwOperator, Error> {
    match operand {
        0 => Ok(mir::AtomicRmwOperator::Exchange),
        1 => Ok(mir::AtomicRmwOperator::Add),
        2 => Ok(mir::AtomicRmwOperator::Sub),
        3 => Ok(mir::AtomicRmwOperator::And),
        4 => Ok(mir::AtomicRmwOperator::Or),
        5 => Ok(mir::AtomicRmwOperator::Xor),
        6 => Ok(mir::AtomicRmwOperator::Min),
        7 => Ok(mir::AtomicRmwOperator::Max),
        8 => Ok(mir::AtomicRmwOperator::Umin),
        9 => Ok(mir::AtomicRmwOperator::Umax),
        10 => Ok(mir::AtomicRmwOperator::Fadd),
        11 => Ok(mir::AtomicRmwOperator::Fmin),
        12 => Ok(mir::AtomicRmwOperator::Fmax),
        _ => Err(Error::InvalidInstruction),
    }
}

/// Return the raw pointee type for one atomic pointer.
fn atomic_pointee_type(
    machine: &Machine<'_, '_>,
    pointer: mir::Value,
) -> Result<mir::LocalNodeId<mir::Type>, Error> {
    let pointer_type = machine.value_type(pointer)?;
    let pointer_type = repr_type(machine.tree(), pointer_type);

    match machine.tree().get(pointer_type) {
        mir::Type::Reference {
            kind,
            address_space,
            pointee,
            ..
        } if matches!(
            pointer_class_from_reference(address_space.clone(), *kind),
            PointerClass::Raw | PointerClass::Stack | PointerClass::Frame
        ) =>
        {
            pointee.ty().ok_or_else(|| Error::MissingRepresentation {
                context: "atomic pointee type".to_string(),
            })
        }
        mir::Type::TensorView {
            kind,
            address_space,
            element,
            ..
        } if matches!(
            pointer_class_from_reference(address_space.clone(), *kind),
            PointerClass::Raw | PointerClass::Stack | PointerClass::Frame
        ) =>
        {
            element.ty().ok_or_else(|| Error::MissingRepresentation {
                context: "atomic pointee type".to_string(),
            })
        }
        _ => Err(Error::InvalidPointerType {
            actual: format!("{pointer_type:?}"),
        }),
    }
}

/// Execute atomic load.
pub(crate) fn execute_atomic_load(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let dest = mir::Value::new(instruction.a);
    let pointer = mir::Value::new(instruction.b);
    let raw_pointee = match atomic_pointee_type(machine, pointer) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };

    // execute the load
    let pointer = machine.get(pointer);
    let value = match machine.atomic_load_value(
        pointer,
        Some(raw_pointee),
        mir::MemoryOrdering::SequentiallyConsistent,
        mir::AtomicScope::Device,
        mir::MemoryScope::Device,
        mir::MemorySemantics::default(),
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error.error),
    };

    // store the result
    machine.set_word(dest, value);

    Transfer::Continue
}

/// Execute atomic store.
pub(crate) fn execute_atomic_store(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let pointer = mir::Value::new(instruction.a);
    let value = mir::Value::new(instruction.b);
    let raw_pointee = match atomic_pointee_type(machine, pointer) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };

    // execute the store
    let pointer = machine.get(pointer);
    let value = machine.get(value);
    if let Err(error) = machine.atomic_store_value(
        pointer,
        value,
        Some(raw_pointee),
        mir::MemoryOrdering::SequentiallyConsistent,
        mir::AtomicScope::Device,
        mir::MemoryScope::Device,
        mir::MemorySemantics::default(),
    ) {
        return Transfer::Error(error.error);
    }

    Transfer::Continue
}

/// Execute atomic compare exchange.
pub(crate) fn execute_atomic_compare_exchange(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let dest = mir::Value::new(instruction.a);
    let pointer = mir::Value::new(instruction.b);
    let expected = mir::Value::new(instruction.c);
    let new_value = mir::Value::new(instruction.d);
    let raw_pointee = match atomic_pointee_type(machine, pointer) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };

    // execute the compare exchange
    let pointer = machine.get(pointer);
    let expected = machine.get(expected);
    let new_value = machine.get(new_value);
    if let Err(error) = machine.atomic_compare_exchange_value(
        dest,
        pointer,
        expected,
        new_value,
        Some(raw_pointee),
        false,
        mir::MemoryOrdering::SequentiallyConsistent,
        mir::AtomicScope::Device,
        mir::MemoryScope::Device,
        mir::MemorySemantics::default(),
    ) {
        return Transfer::Error(error.error);
    }

    Transfer::Continue
}

/// Execute atomic read modify write.
pub(crate) fn execute_atomic_rmw(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let dest = mir::Value::new(instruction.a);
    let operator = match atomic_rmw_operator_from_operand(instruction.b) {
        Ok(operator) => operator,
        Err(error) => return Transfer::Error(error),
    };
    let pointer = mir::Value::new(instruction.c);
    let value = mir::Value::new(instruction.d);
    let raw_pointee = match atomic_pointee_type(machine, pointer) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };

    // execute the read modify write
    let pointer = machine.get(pointer);
    let value = machine.get(value);
    let result = match machine.atomic_rmw_value(
        operator,
        pointer,
        value,
        Some(raw_pointee),
        mir::MemoryOrdering::SequentiallyConsistent,
        mir::AtomicScope::Device,
        mir::MemoryScope::Device,
        mir::MemorySemantics::default(),
    ) {
        Ok(result) => result,
        Err(error) => return Transfer::Error(error.error),
    };

    // store the result
    machine.set_word(dest, result);

    Transfer::Continue
}

/// Execute atomic fence.
pub(crate) fn execute_atomic_fence(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let _ = instruction;

    // execute the fence
    if let Err(error) = machine.atomic_fence(
        mir::MemoryOrdering::SequentiallyConsistent,
        mir::AtomicScope::Device,
        mir::MemoryScope::Device,
        mir::MemorySemantics::default(),
    ) {
        return Transfer::Error(error.error);
    }

    Transfer::Continue
}
