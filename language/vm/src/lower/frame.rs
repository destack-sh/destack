use {destack_engine as engine, destack_mir as mir};

use crate::program::{Instruction, Op, Projection, word_layout_from_type};
use crate::{Error, Result};

use super::lower::BlockLowerer;
use super::op::{select_frame_value_load_op, select_frame_value_store_op};
use super::projection::{element_projection, field_projection};
impl<'a> BlockLowerer<'a> {
    /// Lower one MIR value constructor into frame stores.
    pub(super) fn lower_frame_constructor(
        &self,
        destination: mir::ValueReference,
        values: mir::ArgumentSlice,
    ) -> Result<Vec<Instruction>> {
        // resolve constructor values
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "frame constructor destination".to_string(),
            })?;
        let values = self.tree.get_arguments(values);

        self.lower_frame_init(destination, values)
    }

    /// Lower one MIR value constructor into frame stores.
    pub(super) fn lower_frame_init(
        &self,
        destination: mir::Value,
        values: &[mir::ValueReference],
    ) -> Result<Vec<Instruction>> {
        // resolve physical frame ranges
        let destination_type = self.value_type_for_value(destination)?;
        let ranges = self.frame_ranges(destination_type)?;

        // require one source per range
        if values.len() != ranges.len() {
            return Err(Error::InvalidFieldAccess {
                index: values.len() as u32,
                field_count: ranges.len(),
            });
        }

        // emit one physical store per source
        let mut instructions = Vec::with_capacity(ranges.len());
        for (index, value) in values.iter().enumerate() {
            let value = value.value().ok_or_else(|| Error::MissingRepresentation {
                context: "frame constructor value".to_string(),
            })?;
            let range = ranges[index];
            instructions.push(self.store_frame_range(destination, value, range)?);
        }

        Ok(instructions)
    }

    /// Lower one frame field read into a word load or frame move.
    pub(super) fn lower_field_read(&self, inst: &mir::Instruction) -> Result<Vec<Instruction>> {
        let mir::Instruction::FieldGet {
            destination,
            aggregate: base,
            index,
        } = inst
        else {
            return Err(Error::InvalidInstruction);
        };

        // resolve values and layout
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "field get destination".to_string(),
            })?;
        let base = base.value().ok_or_else(|| Error::MissingRepresentation {
            context: "field get base".to_string(),
        })?;
        let destination_type = self.value_type_for_value(destination)?;
        let base_type = self.value_type_for_value(base)?;
        let layout = self.layout_for_type(base_type)?;
        let field_count = layout.field_count().ok_or(Error::InvalidInstruction)?;
        let field = field_projection(self.tree, self.layouts(), base_type, *index).ok_or(
            Error::InvalidFieldAccess {
                index: *index,
                field_count,
            },
        )?;
        let field_layout = self.layout_for_type(field.value_type)?;

        // read word fields directly
        if field_layout.is_word() {
            let access = field;
            let op = select_frame_value_load_op(access)?;

            return Ok(vec![Instruction::new(
                op,
                word_offset(self, destination)?,
                value_offset(self, base)?,
                instruction_byte_offset(access.byte_offset)?,
                0,
            )]);
        }

        // move non-word fields as frame bytes
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

    /// Lower one frame element read into a word load or frame move.
    pub(super) fn lower_element_read(&self, inst: &mir::Instruction) -> Result<Vec<Instruction>> {
        let mir::Instruction::ElementGet {
            destination,
            array,
            index,
        } = inst
        else {
            return Err(Error::InvalidInstruction);
        };

        // resolve values and reject dynamic slices
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "element get destination".to_string(),
            })?;
        let array = array.value().ok_or_else(|| Error::MissingRepresentation {
            context: "element get array".to_string(),
        })?;
        let destination_type = self.value_type_for_value(destination)?;
        let array_type = self.value_type_for_value(array)?;
        if matches!(self.tree.get(array_type), mir::Type::Slice { .. }) {
            return Err(Error::TypeMismatch {
                expected: "fixed frame element get".to_string(),
                actual: format!("{array_type:?}"),
            });
        }

        // resolve element layout
        let layout = self.layout_for_type(array_type)?;
        let element_count = layout.element_count().ok_or(Error::InvalidInstruction)?;
        if *index as usize >= element_count {
            return Err(Error::InvalidArrayAccess {
                index: u64::from(*index),
                length: element_count as u64,
            });
        }

        let element = element_projection(self.tree, self.layouts(), array_type)
            .ok_or(Error::InvalidInstruction)?;
        let element_layout = self.layout_for_type(element.value_type)?;
        let element_offset = element.byte_stride * *index as usize;

        // read word elements directly
        if element_layout.is_word() {
            let access = element.at_offset(element_offset).with_length(0);
            let op = select_frame_value_load_op(access)?;

            return Ok(vec![Instruction::new(
                op,
                word_offset(self, destination)?,
                value_offset(self, array)?,
                instruction_byte_offset(access.byte_offset)?,
                0,
            )]);
        }

        // move non-word elements as frame bytes
        let destination_access = FrameRange {
            value_type: destination_type,
            byte_offset: 0,
            byte_len: element.byte_len,
        };
        let source_access = element.at_offset(element_offset).with_length(0);

        Ok(vec![move_frame_instruction(
            self,
            destination,
            destination_access.into(),
            array,
            source_access,
        )?])
    }

    /// Lower one functional field update into frame stores.
    pub(super) fn lower_field_update(
        &self,
        destination: mir::ValueReference,
        base: mir::ValueReference,
        index: u32,
        value: mir::ValueReference,
    ) -> Result<Vec<Instruction>> {
        // require SSA values
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "field set destination".to_string(),
            })?;
        let base = base.value().ok_or_else(|| Error::MissingRepresentation {
            context: "field set base".to_string(),
        })?;
        let value = value.value().ok_or_else(|| Error::MissingRepresentation {
            context: "field set value".to_string(),
        })?;

        // resolve original frame value and replacement field
        let destination_type = self.value_type_for_value(destination)?;
        let layout = self.layout_for_type(destination_type)?;
        let field_count = layout.field_count().ok_or(Error::InvalidInstruction)?;
        let field = layout
            .field(index)
            .ok_or(Error::InvalidFieldAccess { index, field_count })?;
        let whole = FrameRange {
            value_type: destination_type,
            byte_offset: 0,
            byte_len: layout.byte_len,
        };
        let field = FrameRange {
            value_type: field.ty,
            byte_offset: field.offset,
            byte_len: field.byte_len,
        };

        // move the original frame value and overwrite one field
        Ok(vec![
            self.store_frame_range(destination, base, whole)?,
            self.store_frame_range(destination, value, field)?,
        ])
    }

    /// Lower one functional element update into frame stores.
    pub(super) fn lower_element_update(
        &self,
        destination: mir::ValueReference,
        array: mir::ValueReference,
        index: u32,
        value: mir::ValueReference,
    ) -> Result<Vec<Instruction>> {
        // require SSA values
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "element set destination".to_string(),
            })?;
        let array = array.value().ok_or_else(|| Error::MissingRepresentation {
            context: "element set array".to_string(),
        })?;
        let value = value.value().ok_or_else(|| Error::MissingRepresentation {
            context: "element set value".to_string(),
        })?;

        // reject dynamic slices
        let destination_type = self.value_type_for_value(destination)?;
        if matches!(self.tree.get(destination_type), mir::Type::Slice { .. }) {
            return Err(Error::TypeMismatch {
                expected: "fixed frame element set".to_string(),
                actual: format!("{destination_type:?}"),
            });
        }

        // resolve original frame value and replacement element
        let layout = self.layout_for_type(destination_type)?;
        let element_count = layout.element_count().ok_or(Error::InvalidInstruction)?;
        if index as usize >= element_count {
            return Err(Error::InvalidArrayAccess {
                index: u64::from(index),
                length: element_count as u64,
            });
        }

        let whole = FrameRange {
            value_type: destination_type,
            byte_offset: 0,
            byte_len: layout.byte_len,
        };
        let element = element_projection(self.tree, self.layouts(), destination_type)
            .ok_or(Error::InvalidInstruction)?;
        let element_offset = element.byte_stride * index as usize;

        // store word elements directly
        let instruction = if element.is_word() {
            let access = element.at_offset(element_offset).with_length(0);
            let op = select_frame_value_store_op(access)?;

            Instruction::new(
                op,
                value_offset(self, destination)?,
                word_offset(self, value)?,
                instruction_byte_offset(access.byte_offset)?,
                0,
            )
        } else {
            let destination_access = element.at_offset(element_offset).with_length(0);
            let source_access = FrameRange {
                value_type: element.value_type,
                byte_offset: 0,
                byte_len: element.byte_len,
            };

            move_frame_instruction(
                self,
                destination,
                destination_access,
                value,
                source_access.into(),
            )?
        };

        // move the original frame value and overwrite one element
        Ok(vec![
            self.store_frame_range(destination, array, whole)?,
            instruction,
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
                    .ok_or(Error::InvalidFieldAccess { index, field_count })?;
                ranges.push(FrameRange {
                    value_type: field.ty,
                    byte_offset: field.offset,
                    byte_len: field.byte_len,
                });
            }

            return Ok(ranges);
        }

        // fall back to fixed indexed elements
        let element = layout.element().ok_or_else(|| Error::TypeMismatch {
            expected: "frame-backed layout".to_string(),
            actual: format!("{value_type:?} for {:?}", self.tree.get(value_type)),
        })?;
        let element_count = layout.element_count().ok_or(Error::InvalidInstruction)?;
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
        // move non-word values as frame bytes
        if word_layout_from_type(self.tree, range.value_type).is_none() {
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

        // store word values through the normal frame store path
        let access = Projection::fixed(
            range.value_type,
            range.byte_offset,
            range.byte_len,
            word_layout_from_type(self.tree, range.value_type),
        );
        let op = select_frame_value_store_op(access)?;

        Ok(Instruction::new(
            op,
            value_offset(self, destination)?,
            word_offset(self, value)?,
            instruction_byte_offset(access.byte_offset)?,
            0,
        ))
    }
}

/// Encode one fixed byte offset into an instruction operand.
fn instruction_byte_offset(byte_offset: usize) -> Result<u32> {
    u32::try_from(byte_offset).map_err(|_| Error::InvalidInstruction)
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
            word_layout: None,
        }
    }
}

/// Return one word value's frame byte offset.
pub(super) fn word_offset(lowerer: &BlockLowerer<'_>, value: mir::Value) -> Result<u32> {
    let region = frame_slot(lowerer, value)?;

    // word instructions require single-word frame slots
    if !region.is_word {
        return Err(Error::TypeMismatch {
            expected: "word value".to_string(),
            actual: format!("frame-backed value: {value:?}"),
        });
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
) -> Result<&'a engine::FrameSlot> {
    lowerer
        .frame_layout
        .value(value.0)
        .ok_or(Error::InvalidInstruction)
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
        return Err(Error::InvalidInstruction);
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
