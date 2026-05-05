use destack_mir as mir;

use crate::program::{Instruction, Op, PointeeAccess};
use crate::{Error, Result};

use super::access::pointee_access;
use super::frame::{value_offset, word_offset};
use super::lower::BlockLowerer;
use super::op::{select_load_op, select_store_op};
use super::pool::Pool;
use super::value::{
    heap_pointee_type_for_value, heap_pointee_type_for_value_layout, pointer_class_for_value,
    raw_pointee_type_for_value, raw_pointee_type_for_value_layout, reference_meta_for_value,
};

/// Encode one fixed byte offset into an instruction lane.
fn instruction_byte_offset(byte_offset: usize) -> Result<u32> {
    u32::try_from(byte_offset).map_err(|_| Error::InvalidInstruction)
}

impl<'a> BlockLowerer<'a> {
    /// Lower one local get.
    pub(super) fn lower_local_get(
        &self,
        destination: mir::ValueReference,
        local: mir::LocalReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "local get destination".to_string(),
            })?;
        let local = local.local().ok_or_else(|| Error::MissingRepresentation {
            context: "local get source".to_string(),
        })?;
        let local = self.local_index(local)?;

        Ok(Instruction::new(
            Op::LoadLocal,
            value_offset(self, destination)?,
            local,
            0,
            0,
        ))
    }

    /// Lower one local address.
    pub(super) fn lower_local_addr(
        &self,
        destination: mir::ValueReference,
        local: mir::LocalReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "local address destination".to_string(),
            })?;
        let local = local.local().ok_or_else(|| Error::MissingRepresentation {
            context: "local address local".to_string(),
        })?;
        let local = self.local_index(local)?;
        let reference = reference_meta_for_value(self.value_layout_map(), destination);

        Ok(Instruction::new(
            Op::AddressLocal,
            word_offset(self, destination)?,
            local,
            reference.bits() as u32,
            0,
        ))
    }

    /// Lower one local set.
    pub(super) fn lower_local_set(
        &self,
        local: mir::LocalReference,
        value: mir::ValueReference,
    ) -> Result<Instruction> {
        let local = local.local().ok_or_else(|| Error::MissingRepresentation {
            context: "local set destination".to_string(),
        })?;
        let value = value.value().ok_or_else(|| Error::MissingRepresentation {
            context: "local set value".to_string(),
        })?;
        let local = self.local_index(local)?;

        Ok(Instruction::new(
            Op::StoreLocal,
            local,
            value_offset(self, value)?,
            0,
            0,
        ))
    }

    /// Lower one static address.
    pub(super) fn lower_static_addr(
        &self,
        destination: mir::ValueReference,
        global: mir::GlobalReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "static address destination".to_string(),
            })?;
        let global = global
            .global()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "static address global".to_string(),
            })?;
        let reference = reference_meta_for_value(self.value_layout_map(), destination);

        Ok(Instruction::new(
            Op::AddressStatic,
            word_offset(self, destination)?,
            global.id,
            reference.bits() as u32,
            0,
        ))
    }

    /// Lower one function address.
    pub(super) fn lower_function_addr(
        &self,
        destination: mir::ValueReference,
        function: mir::FunctionReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "function address destination".to_string(),
            })?;
        let function = function
            .function()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "function address callee".to_string(),
            })?;

        Ok(Instruction::new(
            Op::AddressFunction,
            word_offset(self, destination)?,
            function.id,
            0,
            0,
        ))
    }

    /// Lower one load.
    pub(super) fn lower_load(
        &self,
        pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        pointer: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "load destination".to_string(),
            })?;
        let pointer = pointer
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "load pointer".to_string(),
            })?;
        let access = self.pointee_access_for_value(pointer)?;
        let op = select_load_op(access)?;
        if !access.is_word() {
            let access = pool.pointee_access(access);

            return Ok(Instruction::new(
                op,
                value_offset(self, destination)?,
                word_offset(self, pointer)?,
                access.0,
                0,
            ));
        }

        Ok(Instruction::new(
            op,
            word_offset(self, destination)?,
            word_offset(self, pointer)?,
            instruction_byte_offset(access.byte_offset)?,
            0,
        ))
    }

    /// Lower one store.
    pub(super) fn lower_store(
        &self,
        pool: &mut Pool<'_>,
        pointer: mir::ValueReference,
        value: mir::ValueReference,
    ) -> Result<Instruction> {
        let pointer = pointer
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "store pointer".to_string(),
            })?;
        let value = value.value().ok_or_else(|| Error::MissingRepresentation {
            context: "store value".to_string(),
        })?;
        let access = self.pointee_access_for_value(pointer)?;
        let op = select_store_op(access)?;
        if !access.is_word() {
            let access = pool.pointee_access(access);

            return Ok(Instruction::new(
                op,
                word_offset(self, pointer)?,
                value_offset(self, value)?,
                access.0,
                0,
            ));
        }

        let reference = if is_frame_store_op(op) {
            reference_meta_for_value(self.value_layout_map(), pointer).bits() as u32
        } else {
            0
        };

        Ok(Instruction::new(
            op,
            word_offset(self, pointer)?,
            word_offset(self, value)?,
            instruction_byte_offset(access.byte_offset)?,
            reference,
        ))
    }

    /// Return the lowered access for a pointer value.
    fn pointee_access_for_value(&self, pointer: mir::Value) -> Result<PointeeAccess> {
        let pointee_type = heap_pointee_type_for_value_layout(self.value_layout_map(), pointer)
            .or_else(|| raw_pointee_type_for_value_layout(self.value_layout_map(), pointer))
            .or_else(|| heap_pointee_type_for_value(self.tree, self.value_type(), pointer))
            .or_else(|| raw_pointee_type_for_value(self.tree, self.value_type(), pointer));
        let pointer_class = pointer_class_for_value(self.value_layout_map(), pointer);

        pointee_type
            .and_then(|pointee_type| {
                pointee_access(self.tree, self.layouts(), pointee_type, pointer_class)
            })
            .ok_or(Error::InvalidInstruction)
    }
}

/// Return whether one op stores through frame memory.
fn is_frame_store_op(op: Op) -> bool {
    matches!(
        op,
        Op::StoreFrame8 | Op::StoreFrame16 | Op::StoreFrame32 | Op::StoreFrame64
    )
}
