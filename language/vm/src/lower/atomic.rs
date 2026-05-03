use destack_mir as mir;

use crate::program::{
    AtomicCompareExchange, AtomicLoad, AtomicRmw, AtomicStore, Instruction, Opcode,
};
use crate::{Error, Result};

use super::lower::BlockLowerer;
use super::value::raw_pointee_type_for_value;

impl<'a> BlockLowerer<'a> {
    /// Lower one atomic load.
    pub(super) fn lower_atomic_load(
        &self,
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

        Ok(Instruction::new(
            Opcode::AtomicLoad,
            AtomicLoad {
                dest: destination,
                pointer,
                raw_pointee: raw_pointee_type_for_value(self.tree, self.value_type(), pointer),
            },
        ))
    }

    /// Lower one atomic store.
    pub(super) fn lower_atomic_store(
        &self,
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

        Ok(Instruction::new(
            Opcode::AtomicStore,
            AtomicStore {
                pointer,
                value,
                raw_pointee: raw_pointee_type_for_value(self.tree, self.value_type(), pointer),
            },
        ))
    }

    /// Lower one atomic compare exchange.
    pub(super) fn lower_atomic_compare_exchange(
        &self,
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

        Ok(Instruction::new(
            Opcode::AtomicCompareExchange,
            AtomicCompareExchange {
                dest: destination,
                pointer,
                expected,
                new_value,
                raw_pointee: raw_pointee_type_for_value(self.tree, self.value_type(), pointer),
            },
        ))
    }

    /// Lower one atomic read-modify-write.
    pub(super) fn lower_atomic_rmw(
        &self,
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

        Ok(Instruction::new(
            Opcode::AtomicRmw,
            AtomicRmw {
                dest: destination,
                operator,
                pointer,
                value,
                raw_pointee: raw_pointee_type_for_value(self.tree, self.value_type(), pointer),
            },
        ))
    }
}
