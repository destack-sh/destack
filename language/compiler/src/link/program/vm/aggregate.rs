use destack_mir as mir;

use destack_program::vm::{Instruction, Op, Projection};

use crate::LinkResult;

use super::lower::BlockLowerer;
use super::op::{select_frame_value_load_op, select_frame_value_store_op};

/// One direct byte range inside a frame value.
#[derive(Clone, Copy)]
struct FrameRange {
    /// The program type written into this byte range.
    value_type: destack_program::TypeId,
    /// The byte offset from the frame value base.
    byte_offset: usize,
    /// The byte width of this byte range.
    byte_len: usize,
    /// The cell representation for this range.
    cell_layout: Option<destack_program::CellLayout>,
}

impl FrameRange {
    /// Return this frame range as a projection.
    fn to_projection(self) -> Projection {
        Projection {
            value_type: self.value_type,
            byte_offset: self.byte_offset,
            byte_stride: 0,
            length: 0,
            byte_len: self.byte_len,
            cell_layout: self.cell_layout,
        }
    }
}

impl<'a> BlockLowerer<'a> {
    /// Lower one MIR value constructor into frame stores.
    pub(super) fn lower_frame_constructor(
        &self,
        destination: mir::Value,
        values: mir::ValueSlice,
    ) -> LinkResult<Vec<Instruction>> {
        // resolve constructor values
        let values = self.function.tree.get_values(values);

        self.lower_frame_init(destination, values)
    }

    /// Lower one MIR value constructor into frame stores.
    pub(super) fn lower_frame_init(
        &self,
        destination: mir::Value,
        values: &[mir::Value],
    ) -> LinkResult<Vec<Instruction>> {
        // resolve physical frame ranges
        let destination_type = self.value_type_for_value(destination)?;
        let ranges = self.frame_ranges(destination_type)?;

        // require one source per range
        if values.len() != ranges.len() {
            return Err(self.invalid_field_access(values.len() as u32, ranges.len()));
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
    pub(super) fn lower_field_read(&self, inst: &mir::Instruction) -> LinkResult<Vec<Instruction>> {
        let mir::Instruction::FieldGet {
            destination,
            aggregate: base,
            index,
        } = inst
        else {
            return Err(self.invalid_instruction("field read"));
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
            .ok_or_else(|| self.invalid_instruction("field read count"))?;
        let field = self
            .field_projection(base_type, *index)
            .ok_or_else(|| self.invalid_field_access(*index, field_count))?;

        // read cell fields directly
        if field.cell_layout.is_some() {
            let access = field;
            let op = select_frame_value_load_op(access)
                .ok_or_else(|| self.invalid_instruction("frame value load operation"))?;

            return Ok(vec![Instruction::new(
                op,
                self.cell_offset(destination)?,
                self.value_offset(base)?,
                self.instruction_byte_offset(access.byte_offset)?,
                0,
            )]);
        }

        // move non-cell fields as aggregate ranges
        let destination_access = FrameRange {
            value_type: self.function.program.type_id(destination_type),
            byte_offset: 0,
            byte_len: field.byte_len,
            cell_layout: None,
        };
        let source_access = FrameRange {
            value_type: field.value_type,
            byte_offset: field.byte_offset,
            byte_len: field.byte_len,
            cell_layout: field.cell_layout,
        };

        Ok(vec![self.move_frame_instruction(
            destination,
            destination_access.to_projection(),
            base,
            source_access.to_projection(),
        )?])
    }

    /// Lower one functional field update into frame stores.
    pub(super) fn lower_field_update(
        &self,
        destination: mir::Value,
        base: mir::Value,
        index: u32,
        value: mir::Value,
    ) -> LinkResult<Vec<Instruction>> {
        // resolve original frame value and replacement field
        let destination_type = self.value_type_for_value(destination)?;
        let layout = self.layout_for_type(destination_type)?;
        let field_count = layout
            .field_count()
            .or_else(|| layout.element_count())
            .ok_or_else(|| self.invalid_instruction("field update count"))?;
        let field = self
            .field_projection(destination_type, index)
            .ok_or_else(|| self.invalid_field_access(index, field_count))?;
        let whole = FrameRange {
            value_type: self.function.program.type_id(destination_type),
            byte_offset: 0,
            byte_len: layout.byte_len,
            cell_layout: None,
        };
        let field = FrameRange {
            value_type: field.value_type,
            byte_offset: field.byte_offset,
            byte_len: field.byte_len,
            cell_layout: field.cell_layout,
        };

        // move the original frame value and overwrite one field
        Ok(vec![
            self.store_frame_range(destination, base, whole)?,
            self.store_frame_range(destination, value, field)?,
        ])
    }

    /// Return direct byte ranges for one frame-backed value type.
    fn frame_ranges(&self, value_type: mir::LocalNodeId<mir::Type>) -> LinkResult<Vec<FrameRange>> {
        // prefer named fields for records
        let layout = self.layout_for_type(value_type)?;
        if let Some(field_count) = layout.field_count() {
            let mut ranges = Vec::with_capacity(field_count);
            for index in 0..field_count {
                let index = index as u32;
                let field = layout
                    .field(index)
                    .ok_or_else(|| self.invalid_field_access(index, field_count))?;
                ranges.push(FrameRange {
                    value_type: self.function.program.type_id(field.ty),
                    byte_offset: field.offset,
                    byte_len: field.byte_len,
                    cell_layout: self.function.cell_layout_for_type(field.ty),
                });
            }

            return Ok(ranges);
        }

        // fall back to fixed indexed elements
        let element = layout.element().ok_or_else(|| {
            self.type_mismatch(
                "frame-backed layout",
                format!(
                    "{value_type:?} for {:?}",
                    self.function.tree.get(value_type)
                ),
            )
        })?;
        let element_count = layout
            .element_count()
            .ok_or_else(|| self.invalid_instruction("frame element count"))?;
        let mut ranges = Vec::with_capacity(element_count);
        for index in 0..element_count {
            let byte_offset = element.stride * index;
            ranges.push(FrameRange {
                value_type: self.function.program.type_id(element.ty),
                byte_offset,
                byte_len: element.byte_len,
                cell_layout: self.function.cell_layout_for_type(element.ty),
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
    ) -> LinkResult<Instruction> {
        // move non-cell values as aggregate ranges
        if range.cell_layout.is_none() {
            let destination_access = range.to_projection();
            let source_access = FrameRange {
                byte_offset: 0,
                ..range
            }
            .to_projection();

            return self.move_frame_instruction(
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
            range.cell_layout,
        );
        let op = select_frame_value_store_op(access)
            .ok_or_else(|| self.invalid_instruction("frame value store operation"))?;

        Ok(Instruction::new(
            op,
            self.value_offset(destination)?,
            self.cell_offset(value)?,
            self.instruction_byte_offset(access.byte_offset)?,
            0,
        ))
    }

    /// Return one frame byte move instruction.
    fn move_frame_instruction(
        &self,
        destination: mir::Value,
        destination_access: Projection,
        source: mir::Value,
        source_access: Projection,
    ) -> LinkResult<Instruction> {
        if destination_access.byte_len != source_access.byte_len {
            return Err(self.invalid_instruction("frame move byte length"));
        }

        let destination_offset =
            self.value_offset(destination)? + destination_access.byte_offset as u32;
        let source_offset = self.value_offset(source)? + source_access.byte_offset as u32;

        Ok(Instruction::new(
            Op::MoveAggregate,
            destination_offset,
            destination_access.byte_len as u32,
            source_offset,
            0,
        ))
    }
}
