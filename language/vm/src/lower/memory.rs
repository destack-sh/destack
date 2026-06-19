use destack_mir as mir;
use destack_program as program;

use crate::{Error, Result};
use destack_program::vm::{Instruction, Op, Projection};

use super::frame::{cell_offset, value_offset};
use super::lower::BlockLowerer;
use super::op::{select_load_op, select_store_op};
use super::pool::Pool;
use super::projection::pointee_projection;
use super::value::{
    address_space_for_value, heap_pointee_type_for_storage_id, heap_pointee_type_for_value,
    raw_pointee_type_for_storage_id, raw_pointee_type_for_value,
};

/// Encode one fixed byte offset into an instruction operand.
fn instruction_byte_offset(byte_offset: usize) -> Result<u32> {
    u32::try_from(byte_offset).map_err(|_| Error::invalid_instruction())
}

impl<'a> BlockLowerer<'a> {
    /// Lower one local get.
    pub(super) fn lower_local_get(
        &self,
        destination: mir::Value,
        local: mir::LocalId,
    ) -> Result<Instruction> {
        let destination_slot = frame_value_slot(self, destination)?;
        let local_slot = frame_local_slot(self, local)?;

        // move cell locals without runtime layout lookup
        if destination_slot.is_cell && local_slot.is_cell {
            return Ok(Instruction::new(
                Op::MoveCell,
                destination_slot.offset,
                local_slot.offset,
                0,
                0,
            ));
        }

        // move frame-backed locals through one fixed byte range
        if destination_slot.byte_len != local_slot.byte_len {
            return Err(Error::invalid_instruction());
        }
        let byte_len = destination_slot.byte_len;

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
    ) -> Result<Instruction> {
        let local = self.local_index(local)?;

        Ok(Instruction::new(
            Op::LocalAddress,
            cell_offset(self, destination)?,
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
    ) -> Result<Instruction> {
        let local_slot = frame_local_slot(self, local)?;
        let value_slot = frame_value_slot(self, value)?;

        // move cell locals without runtime layout lookup
        if local_slot.is_cell && value_slot.is_cell {
            return Ok(Instruction::new(
                Op::MoveCell,
                local_slot.offset,
                value_slot.offset,
                0,
                0,
            ));
        }

        // move frame-backed locals through one fixed byte range
        if local_slot.byte_len != value_slot.byte_len {
            return Err(Error::invalid_instruction());
        }
        let byte_len = local_slot.byte_len;

        Ok(Instruction::new(
            Op::MoveAggregate,
            local_slot.offset,
            byte_len,
            value_slot.offset,
            0,
        ))
    }

    /// Lower one static address.
    pub(super) fn lower_static_addr(
        &self,
        destination: mir::Value,
        global: mir::GlobalId,
    ) -> Result<Instruction> {
        Ok(Instruction::new(
            Op::StaticAddress,
            cell_offset(self, destination)?,
            global.id,
            0,
            0,
        ))
    }

    /// Lower one function address.
    pub(super) fn lower_function_addr(
        &self,
        destination: mir::Value,
        function: mir::FunctionId,
    ) -> Result<Instruction> {
        Ok(Instruction::new(
            Op::FunctionAddress,
            cell_offset(self, destination)?,
            function.id,
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
    ) -> Result<Instruction> {
        let access = self.pointee_projection_for_value(pointer)?;
        let address_space = address_space_for_value(self.value_shape_map(), pointer)?;
        let op = select_load_op(address_space, access)?;
        if !access.is_cell() {
            let access = pool.projection(access);

            return Ok(Instruction::new(
                op,
                value_offset(self, destination)?,
                cell_offset(self, pointer)?,
                access.0,
                0,
            ));
        }

        Ok(Instruction::new(
            op,
            cell_offset(self, destination)?,
            cell_offset(self, pointer)?,
            instruction_byte_offset(access.byte_offset)?,
            0,
        ))
    }

    /// Lower one store.
    pub(super) fn lower_store(
        &self,
        pool: &mut Pool<'_, '_>,
        pointer: mir::Value,
        value: mir::Value,
    ) -> Result<Instruction> {
        let access = self.pointee_projection_for_value(pointer)?;
        let address_space = address_space_for_value(self.value_shape_map(), pointer)?;
        let op = select_store_op(address_space, access)?;

        if !access.is_cell() {
            let access = pool.projection(access);

            return Ok(Instruction::new(
                op,
                cell_offset(self, pointer)?,
                value_offset(self, value)?,
                access.0,
                0,
            ));
        }

        Ok(Instruction::new(
            op,
            cell_offset(self, pointer)?,
            cell_offset(self, value)?,
            instruction_byte_offset(access.byte_offset)?,
            0,
        ))
    }

    /// Return the lowered projection for a pointer value.
    fn pointee_projection_for_value(&self, pointer: mir::Value) -> Result<Projection> {
        let pointee_type = heap_pointee_type_for_storage_id(self.value_shape_map(), pointer)
            .or_else(|| raw_pointee_type_for_storage_id(self.value_shape_map(), pointer))
            .or_else(|| heap_pointee_type_for_value(self.tree, self.value_type(), pointer))
            .or_else(|| raw_pointee_type_for_value(self.tree, self.value_type(), pointer));
        pointee_type
            .and_then(|pointee_type| pointee_projection(self.tree, self.layouts(), pointee_type))
            .ok_or(Error::invalid_instruction())
    }
}

/// Return one value's frame slot.
pub(super) fn frame_value_slot<'a>(
    lowerer: &'a BlockLowerer<'_>,
    value: mir::Value,
) -> Result<&'a program::FrameSlot> {
    lowerer
        .frame_layout
        .value(value.0)
        .ok_or(Error::invalid_instruction())
}

/// Return one local's frame slot.
fn frame_local_slot<'a>(
    lowerer: &'a BlockLowerer<'_>,
    local: mir::LocalNodeId<mir::Local>,
) -> Result<&'a program::FrameSlot> {
    let local = lowerer.local_index(local)?;

    lowerer
        .frame_layout
        .local(local)
        .ok_or(Error::invalid_instruction())
}
