use destack_engine as engine;
use destack_mir as mir;

use crate::program::{Instruction, Op, Projection};
use crate::{Error, Result};

use super::frame::{value_offset, word_offset};
use super::lower::BlockLowerer;
use super::op::{select_load_op, select_store_op};
use super::pool::Pool;
use super::projection::pointee_projection;
use super::value::{
    heap_pointee_type_for_value, heap_pointee_type_for_value_layout, pointer_class_for_value,
    raw_pointee_type_for_value, raw_pointee_type_for_value_layout,
};

/// Encode one fixed byte offset into an instruction operand.
fn instruction_byte_offset(byte_offset: usize) -> Result<u32> {
    u32::try_from(byte_offset).map_err(|_| Error::invalid_instruction())
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
            .ok_or_else(|| Error::invalid_program("local get destination"))?;
        let local = local
            .local()
            .ok_or_else(|| Error::invalid_program("local get source"))?;
        let destination_slot = frame_value_slot(self, destination)?;
        let local_slot = frame_local_slot(self, local)?;

        // move word locals without runtime layout lookup
        if destination_slot.is_word && local_slot.is_word {
            return Ok(Instruction::new(
                Op::MoveWord,
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
            Op::MoveFrame,
            destination_slot.offset,
            byte_len,
            local_slot.offset,
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
            .ok_or_else(|| Error::invalid_program("local address destination"))?;
        let local = local
            .local()
            .ok_or_else(|| Error::invalid_program("local address local"))?;
        let local = self.local_index(local)?;

        Ok(Instruction::new(
            Op::AddressLocal,
            word_offset(self, destination)?,
            local,
            0,
            0,
        ))
    }

    /// Lower one local set.
    pub(super) fn lower_local_set(
        &self,
        local: mir::LocalReference,
        value: mir::ValueReference,
    ) -> Result<Instruction> {
        let local = local
            .local()
            .ok_or_else(|| Error::invalid_program("local set destination"))?;
        let value = value
            .value()
            .ok_or_else(|| Error::invalid_program("local set value"))?;
        let local_slot = frame_local_slot(self, local)?;
        let value_slot = frame_value_slot(self, value)?;

        // move word locals without runtime layout lookup
        if local_slot.is_word && value_slot.is_word {
            return Ok(Instruction::new(
                Op::MoveWord,
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
            Op::MoveFrame,
            local_slot.offset,
            byte_len,
            value_slot.offset,
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
            .ok_or_else(|| Error::invalid_program("static address destination"))?;
        let global = global
            .global()
            .ok_or_else(|| Error::invalid_program("static address global"))?;

        Ok(Instruction::new(
            Op::AddressStatic,
            word_offset(self, destination)?,
            global.id,
            0,
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
            .ok_or_else(|| Error::invalid_program("function address destination"))?;
        let function = function
            .function()
            .ok_or_else(|| Error::invalid_program("function address callee"))?;

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
        pool: &mut Pool<'_, '_>,
        destination: mir::ValueReference,
        pointer: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::invalid_program("load destination"))?;
        let pointer = pointer
            .value()
            .ok_or_else(|| Error::invalid_program("load pointer"))?;
        let access = self.pointee_projection_for_value(pointer)?;
        let pointer_class = pointer_class_for_value(self.value_layout_map(), pointer);
        let op = select_load_op(pointer_class, access)?;
        if !access.is_word() {
            let access = pool.projection(access);

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
        pool: &mut Pool<'_, '_>,
        pointer: mir::ValueReference,
        value: mir::ValueReference,
    ) -> Result<Instruction> {
        let pointer = pointer
            .value()
            .ok_or_else(|| Error::invalid_program("store pointer"))?;
        let value = value
            .value()
            .ok_or_else(|| Error::invalid_program("store value"))?;
        let access = self.pointee_projection_for_value(pointer)?;
        let pointer_class = pointer_class_for_value(self.value_layout_map(), pointer);
        let op = select_store_op(pointer_class, access)?;

        if !access.is_word() {
            let access = pool.projection(access);

            return Ok(Instruction::new(
                op,
                word_offset(self, pointer)?,
                value_offset(self, value)?,
                access.0,
                0,
            ));
        }

        Ok(Instruction::new(
            op,
            word_offset(self, pointer)?,
            word_offset(self, value)?,
            instruction_byte_offset(access.byte_offset)?,
            0,
        ))
    }

    /// Return the lowered projection for a pointer value.
    fn pointee_projection_for_value(&self, pointer: mir::Value) -> Result<Projection> {
        let pointee_type = heap_pointee_type_for_value_layout(self.value_layout_map(), pointer)
            .or_else(|| raw_pointee_type_for_value_layout(self.value_layout_map(), pointer))
            .or_else(|| heap_pointee_type_for_value(self.tree, self.value_type(), pointer))
            .or_else(|| raw_pointee_type_for_value(self.tree, self.value_type(), pointer));
        pointee_type
            .and_then(|pointee_type| pointee_projection(self.tree, self.layouts(), pointee_type))
            .ok_or(Error::invalid_instruction())
    }
}

/// Return one value's frame slot.
fn frame_value_slot<'a>(
    lowerer: &'a BlockLowerer<'_>,
    value: mir::Value,
) -> Result<&'a engine::FrameSlot> {
    lowerer
        .frame_layout
        .value(value.0)
        .ok_or(Error::invalid_instruction())
}

/// Return one local's frame slot.
fn frame_local_slot<'a>(
    lowerer: &'a BlockLowerer<'_>,
    local: mir::LocalNodeId<mir::Local>,
) -> Result<&'a engine::FrameSlot> {
    let local = lowerer.local_index(local)?;

    lowerer
        .frame_layout
        .local(local)
        .ok_or(Error::invalid_instruction())
}
