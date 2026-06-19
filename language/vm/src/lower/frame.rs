use destack_mir as mir;
use destack_program as program;

use crate::{Error, Result};
use destack_program::vm::{Instruction, Op, Projection, cell_layout_from_type};

use super::lower::BlockLowerer;
use super::op::{select_frame_value_load_op, select_frame_value_store_op};
use super::projection::field_projection;
impl<'a> BlockLowerer<'a> {
    /// Lower one MIR value constructor into frame stores.
    pub(super) fn lower_frame_constructor(
        &self,
        destination: mir::Value,
        values: mir::ValueSlice,
    ) -> Result<Vec<Instruction>> {
        // resolve constructor values
        let values = self.tree.get_values(values);

        self.lower_frame_init(destination, values)
    }

    /// Lower one MIR value constructor into frame stores.
    pub(super) fn lower_frame_init(
        &self,
        destination: mir::Value,
        values: &[mir::Value],
    ) -> Result<Vec<Instruction>> {
        // resolve physical frame ranges
        let destination_type = self.value_type_for_value(destination)?;
        let ranges = self.frame_ranges(destination_type)?;

        // require one source per range
        if values.len() != ranges.len() {
            return Err(Error::invalid_field_access(
                values.len() as u32,
                ranges.len(),
            ));
        }

        // emit one physical store per source
        let mut instructions = Vec::with_capacity(ranges.len());
        for (index, value) in values.iter().copied().enumerate() {
            let range = ranges[index];
            instructions.push(self.store_frame_range(destination, value, range)?);
        }

        Ok(instructions)
    }

    /// Lower one frame field read into a cell load or frame move.
    pub(super) fn lower_field_read(&self, inst: &mir::Instruction) -> Result<Vec<Instruction>> {
        let mir::Instruction::FieldGet {
            destination,
            aggregate: base,
            index,
        } = inst
        else {
            return Err(Error::invalid_instruction());
        };

        // resolve values and layout
        let destination = *destination;
        let base = *base;
        let destination_type = self.value_type_for_value(destination)?;
        let base_type = self.value_type_for_value(base)?;
        let layout = self.layout_for_type(base_type)?;
        let field_count = layout
            .field_count()
            .or_else(|| layout.element_count())
            .ok_or(Error::invalid_instruction())?;
        let field = field_projection(self.tree, self.layouts(), base_type, *index)
            .ok_or(Error::invalid_field_access(*index, field_count))?;
        let field_layout = self.layout_for_type(field.value_type)?;

        // read cell fields directly
        if field_layout.is_cell() {
            let access = field;
            let op = select_frame_value_load_op(access)?;

            return Ok(vec![Instruction::new(
                op,
                cell_offset(self, destination)?,
                value_offset(self, base)?,
                instruction_byte_offset(access.byte_offset)?,
                0,
            )]);
        }

        // move non-cell fields as frame bytes
        let destination_access = FrameRange {
            value_type: destination_type,
            byte_offset: 0,
            byte_len: field.byte_len,
        };
        let source_access = FrameRange {
            value_type: field.value_type,
            byte_offset: field.byte_offset,
            byte_len: field.byte_len,
        };

        Ok(vec![move_frame_instruction(
            self,
            destination,
            destination_access.into(),
            base,
            source_access.into(),
        )?])
    }

    /// Lower one functional field update into frame stores.
    pub(super) fn lower_field_update(
        &self,
        destination: mir::Value,
        base: mir::Value,
        index: u32,
        value: mir::Value,
    ) -> Result<Vec<Instruction>> {
        // resolve original frame value and replacement field
        let destination_type = self.value_type_for_value(destination)?;
        let layout = self.layout_for_type(destination_type)?;
        let field_count = layout
            .field_count()
            .or_else(|| layout.element_count())
            .ok_or(Error::invalid_instruction())?;
        let field = field_projection(self.tree, self.layouts(), destination_type, index)
            .ok_or(Error::invalid_field_access(index, field_count))?;
        let whole = FrameRange {
            value_type: destination_type,
            byte_offset: 0,
            byte_len: layout.byte_len,
        };
        let field = FrameRange {
            value_type: field.value_type,
            byte_offset: field.byte_offset,
            byte_len: field.byte_len,
        };

        // move the original frame value and overwrite one field
        Ok(vec![
            self.store_frame_range(destination, base, whole)?,
            self.store_frame_range(destination, value, field)?,
        ])
    }

    /// Return direct byte ranges for one frame-backed value type.
    fn frame_ranges(&self, value_type: mir::LocalNodeId<mir::Type>) -> Result<Vec<FrameRange>> {
        // prefer named fields for records
        let layout = self.layout_for_type(value_type)?;
        if let Some(field_count) = layout.field_count() {
            let mut ranges = Vec::with_capacity(field_count);
            for index in 0..field_count {
                let index = index as u32;
                let field = layout
                    .field(index)
                    .ok_or(Error::invalid_field_access(index, field_count))?;
                ranges.push(FrameRange {
                    value_type: field.ty,
                    byte_offset: field.offset,
                    byte_len: field.byte_len,
                });
            }

            return Ok(ranges);
        }

        // fall back to fixed indexed elements
        let element = layout.element().ok_or_else(|| {
            Error::type_mismatch(
                "frame-backed layout",
                format!("{value_type:?} for {:?}", self.tree.get(value_type)),
            )
        })?;
        let element_count = layout.element_count().ok_or(Error::invalid_instruction())?;
        let mut ranges = Vec::with_capacity(element_count);
        for index in 0..element_count {
            let byte_offset = element.stride * index;
            ranges.push(FrameRange {
                value_type: element.ty,
                byte_offset,
                byte_len: element.byte_len,
            });
        }

        Ok(ranges)
    }

    /// Lower one value write into destination frame bytes.
    fn store_frame_range(
        &self,
        destination: mir::Value,
        value: mir::Value,
        range: FrameRange,
    ) -> Result<Instruction> {
        // move non-cell values as frame bytes
        if cell_layout_from_type(self.tree, range.value_type).is_none() {
            let destination_access = range.into();
            let source_access = FrameRange {
                byte_offset: 0,
                ..range
            }
            .into();

            return move_frame_instruction(
                self,
                destination,
                destination_access,
                value,
                source_access,
            );
        }

        // store cell values through the normal frame store path
        let access = Projection::fixed(
            range.value_type,
            range.byte_offset,
            range.byte_len,
            cell_layout_from_type(self.tree, range.value_type),
        );
        let op = select_frame_value_store_op(access)?;

        Ok(Instruction::new(
            op,
            value_offset(self, destination)?,
            cell_offset(self, value)?,
            instruction_byte_offset(access.byte_offset)?,
            0,
        ))
    }
}

/// Encode one fixed byte offset into an instruction operand.
fn instruction_byte_offset(byte_offset: usize) -> Result<u32> {
    u32::try_from(byte_offset).map_err(|_| Error::invalid_instruction())
}

/// One direct byte range inside a frame value.
#[derive(Clone, Copy)]
struct FrameRange {
    /// The MIR type written into this byte range.
    value_type: mir::LocalNodeId<mir::Type>,
    /// The byte offset from the frame value base.
    byte_offset: usize,
    /// The byte width of this byte range.
    byte_len: usize,
}

impl From<FrameRange> for Projection {
    fn from(range: FrameRange) -> Self {
        Self {
            value_type: range.value_type,
            byte_offset: range.byte_offset,
            byte_stride: 0,
            length: 0,
            byte_len: range.byte_len,
            cell_layout: None,
        }
    }
}

/// Return one cell value's frame byte offset.
pub(super) fn cell_offset(lowerer: &BlockLowerer<'_>, value: mir::Value) -> Result<u32> {
    let region = frame_slot(lowerer, value)?;

    // cell instructions require single-cell frame slots
    if !region.is_cell {
        return Err(Error::type_mismatch(
            "cell value",
            format!("frame-backed value: {value:?}"),
        ));
    }

    Ok(region.offset)
}

/// Return one value's frame byte offset.
pub(super) fn value_offset(lowerer: &BlockLowerer<'_>, value: mir::Value) -> Result<u32> {
    Ok(frame_slot(lowerer, value)?.offset)
}

/// Return one lowered frame slot.
fn frame_slot<'a>(
    lowerer: &'a BlockLowerer<'_>,
    value: mir::Value,
) -> Result<&'a program::FrameSlot> {
    lowerer
        .frame_layout
        .value(value.0)
        .ok_or(Error::invalid_instruction())
}

/// Return one frame byte move instruction.
fn move_frame_instruction(
    lowerer: &BlockLowerer<'_>,
    destination: mir::Value,
    destination_access: Projection,
    source: mir::Value,
    source_access: Projection,
) -> Result<Instruction> {
    if destination_access.byte_len != source_access.byte_len {
        return Err(Error::invalid_instruction());
    }

    let destination_offset =
        value_offset(lowerer, destination)? + destination_access.byte_offset as u32;
    let source_offset = value_offset(lowerer, source)? + source_access.byte_offset as u32;

    Ok(Instruction::new(
        Op::MoveFrame,
        destination_offset,
        destination_access.byte_len as u32,
        source_offset,
        0,
    ))
}
