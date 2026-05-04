use destack_mir as mir;

use crate::program::{Instruction, Op};
use crate::{Error, Result};

use super::lower::BlockLowerer;
use super::pool::Pool;
use super::value::raw_pointee_type_for_value;

/// Return one atomic read modify write operator operand.
fn atomic_rmw_operator_operand(operator: mir::AtomicRmwOperator) -> u32 {
    match operator {
        mir::AtomicRmwOperator::Exchange => 0,
        mir::AtomicRmwOperator::Add => 1,
        mir::AtomicRmwOperator::Sub => 2,
        mir::AtomicRmwOperator::And => 3,
        mir::AtomicRmwOperator::Or => 4,
        mir::AtomicRmwOperator::Xor => 5,
        mir::AtomicRmwOperator::Min => 6,
        mir::AtomicRmwOperator::Max => 7,
        mir::AtomicRmwOperator::Umin => 8,
        mir::AtomicRmwOperator::Umax => 9,
        mir::AtomicRmwOperator::Fadd => 10,
        mir::AtomicRmwOperator::Fmin => 11,
        mir::AtomicRmwOperator::Fmax => 12,
    }
}

/// Require an atomic pointer with a concrete raw pointee type.
fn require_atomic_pointee(
    tree: &mir::Tree,
    value_types: &[mir::LocalNodeId<mir::Type>],
    pointer: mir::Value,
) -> Result<()> {
    // atomics operate over raw memory in the VM interpreter
    if raw_pointee_type_for_value(tree, value_types, pointer).is_none() {
        return Err(Error::InvalidPointerType {
            actual: format!("{pointer:?}"),
        });
    }

    Ok(())
}

impl<'a> BlockLowerer<'a> {
    /// Lower one atomic load.
    pub(super) fn lower_atomic_load(
        &self,
        _pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        pointer: mir::ValueReference,
    ) -> Result<Instruction> {
        // require SSA values
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic load destination".to_string(),
            })?;
        let pointer = pointer
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic load pointer".to_string(),
            })?;

        require_atomic_pointee(self.tree, self.value_type(), pointer)?;

        Ok(Instruction::new(
            Op::AtomicLoad,
            destination.id(),
            pointer.id(),
            0,
            0,
        ))
    }

    /// Lower one atomic store.
    pub(super) fn lower_atomic_store(
        &self,
        _pool: &mut Pool<'_>,
        pointer: mir::ValueReference,
        value: mir::ValueReference,
    ) -> Result<Instruction> {
        // require SSA values
        let pointer = pointer
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic store pointer".to_string(),
            })?;
        let value = value.value().ok_or_else(|| Error::MissingRepresentation {
            context: "atomic store value".to_string(),
        })?;

        require_atomic_pointee(self.tree, self.value_type(), pointer)?;

        Ok(Instruction::new(
            Op::AtomicStore,
            pointer.id(),
            value.id(),
            0,
            0,
        ))
    }

    /// Lower one atomic compare exchange.
    pub(super) fn lower_atomic_compare_exchange(
        &self,
        _pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        pointer: mir::ValueReference,
        expected: mir::ValueReference,
        new_value: mir::ValueReference,
    ) -> Result<Instruction> {
        // require SSA values
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic compare exchange destination".to_string(),
            })?;
        let pointer = pointer
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic compare exchange pointer".to_string(),
            })?;
        let expected = expected
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic compare exchange expected".to_string(),
            })?;
        let new_value = new_value
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic compare exchange new value".to_string(),
            })?;

        require_atomic_pointee(self.tree, self.value_type(), pointer)?;

        Ok(Instruction::new(
            Op::AtomicCompareExchange,
            destination.id(),
            pointer.id(),
            expected.id(),
            new_value.id(),
        ))
    }

    /// Lower one atomic read-modify-write.
    pub(super) fn lower_atomic_rmw(
        &self,
        _pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        operator: mir::AtomicRmwOperator,
        pointer: mir::ValueReference,
        value: mir::ValueReference,
    ) -> Result<Instruction> {
        // require SSA values
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic rmw destination".to_string(),
            })?;
        let pointer = pointer
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic rmw pointer".to_string(),
            })?;
        let value = value.value().ok_or_else(|| Error::MissingRepresentation {
            context: "atomic rmw value".to_string(),
        })?;

        require_atomic_pointee(self.tree, self.value_type(), pointer)?;

        Ok(Instruction::new(
            Op::AtomicRmw,
            destination.id(),
            atomic_rmw_operator_operand(operator),
            pointer.id(),
            value.id(),
        ))
    }
}
