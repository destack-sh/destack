use destack_mir as mir;

use destack_program::vm::{Instruction, Op, Projection};

use crate::LinkResult;

use super::lower::BlockLowerer;
use super::op::{select_load_op, select_store_op};
use super::pool::Pool;

impl<'a> BlockLowerer<'a> {
    /// Lower one local get.
    pub(super) fn lower_local_get(
        &self,
        destination: mir::Value,
        local: mir::LocalId,
    ) -> LinkResult<Instruction> {
        let destination_slot = self.frame_value_slot(destination)?;
        let local_slot = self.frame_local_slot(local)?;

        // move cell locals without runtime layout lookup
        if self.function.slot_is_cell(destination_slot) && self.function.slot_is_cell(local_slot) {
            return Ok(Instruction::new(
                Op::MoveCell,
                destination_slot.offset,
                local_slot.offset,
                0,
                0,
            ));
        }

        // move frame-backed locals through one fixed byte range
        if destination_slot.byte_len() != local_slot.byte_len() {
            return Err(self.invalid_instruction("local get byte length"));
        }
        let byte_len = destination_slot.byte_len();

        Ok(Instruction::new(
            Op::MoveAggregate,
            destination_slot.offset,
            byte_len,
            local_slot.offset,
            0,
        ))
    }

    /// Lower one local address.
    pub(super) fn lower_local_addr(
        &self,
        destination: mir::Value,
        local: mir::LocalId,
    ) -> LinkResult<Instruction> {
        let local = self.local_index(local)?;

        Ok(Instruction::new(
            Op::LocalAddress,
            self.cell_offset(destination)?,
            local,
            0,
            0,
        ))
    }

    /// Lower one local set.
    pub(super) fn lower_local_set(
        &self,
        local: mir::LocalId,
        value: mir::Value,
    ) -> LinkResult<Instruction> {
        let local_slot = self.frame_local_slot(local)?;
        let value_slot = self.frame_value_slot(value)?;

        // move cell locals without runtime layout lookup
        if self.function.slot_is_cell(local_slot) && self.function.slot_is_cell(value_slot) {
            return Ok(Instruction::new(
                Op::MoveCell,
                local_slot.offset,
                value_slot.offset,
                0,
                0,
            ));
        }

        // move frame-backed locals through one fixed byte range
        if local_slot.byte_len() != value_slot.byte_len() {
            return Err(self.invalid_instruction("local set byte length"));
        }
        let byte_len = local_slot.byte_len();

        Ok(Instruction::new(
            Op::MoveAggregate,
            local_slot.offset,
            byte_len,
            value_slot.offset,
            0,
        ))
    }

    /// Lower one global address.
    pub(super) fn lower_static_addr(
        &self,
        destination: mir::Value,
        global: mir::GlobalId,
    ) -> LinkResult<Instruction> {
        Ok(Instruction::new(
            Op::GlobalAddress,
            self.cell_offset(destination)?,
            self.function.program.global_id(global).0,
            0,
            0,
        ))
    }

    /// Lower one function address.
    pub(super) fn lower_function_addr(
        &self,
        destination: mir::Value,
        function: mir::FunctionId,
    ) -> LinkResult<Instruction> {
        Ok(Instruction::new(
            Op::FunctionAddress,
            self.cell_offset(destination)?,
            self.function.program_function(function).0,
            0,
            0,
        ))
    }

    /// Lower one load.
    pub(super) fn lower_load(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::Value,
        pointer: mir::Value,
    ) -> LinkResult<Instruction> {
        let access = self.pointee_projection_for_value(pointer)?;
        let pointer_layout = self
            .operand_map()
            .pointer_cell_layout(pointer)
            .ok_or_else(|| self.invalid_pointer_type(format!("{pointer:?}")))?;
        let op = select_load_op(pointer_layout, access)
            .ok_or_else(|| self.invalid_instruction("load operation"))?;
        if !access.is_cell() {
            let access = pool.projection(access);

            return Ok(Instruction::new(
                op,
                self.value_offset(destination)?,
                self.cell_offset(pointer)?,
                access.0,
                0,
            ));
        }

        Ok(Instruction::new(
            op,
            self.cell_offset(destination)?,
            self.cell_offset(pointer)?,
            self.instruction_byte_offset(access.byte_offset())?,
            0,
        ))
    }

    /// Lower one store.
    pub(super) fn lower_store(
        &self,
        pool: &mut Pool<'_, '_>,
        pointer: mir::Value,
        value: mir::Value,
    ) -> LinkResult<Instruction> {
        let access = self.pointee_projection_for_value(pointer)?;
        let pointer_layout = self
            .operand_map()
            .pointer_cell_layout(pointer)
            .ok_or_else(|| self.invalid_pointer_type(format!("{pointer:?}")))?;
        let op = select_store_op(pointer_layout, access)
            .ok_or_else(|| self.invalid_instruction("store operation"))?;

        if !access.is_cell() {
            let access = pool.projection(access);

            return Ok(Instruction::new(
                op,
                self.cell_offset(pointer)?,
                self.value_offset(value)?,
                access.0,
                0,
            ));
        }

        Ok(Instruction::new(
            op,
            self.cell_offset(pointer)?,
            self.cell_offset(value)?,
            self.instruction_byte_offset(access.byte_offset())?,
            0,
        ))
    }

    /// Return the lowered projection for a pointer value.
    fn pointee_projection_for_value(&self, pointer: mir::Value) -> LinkResult<Projection> {
        let pointee_type = self
            .operand_map()
            .heap_pointee_type(self.function.program, pointer)
            .or_else(|| {
                self.operand_map()
                    .raw_pointee_type(self.function.program, pointer)
            });
        pointee_type
            .and_then(|pointee_type| self.pointee_projection(pointee_type))
            .ok_or_else(|| self.invalid_instruction("pointee projection"))
    }
}
