use std::ptr::NonNull;

use crate::Word;
use destack_engine as engine;

use super::frame::{FrameValue, frame_value_type, write_frame_value};
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::interpreter::{Frame, Interpreter};
use crate::program::Program;
use destack_mir as mir;

/// Value copied while a continuation resumes into a target block.
enum ResumeCopyValue {
    /// One scalar word.
    Word(Word),
    /// One frame byte range.
    Bytes(Vec<u8>),
}

/// Apply resume moves within one frame.
fn apply_resume_moves(
    layout: &engine::FrameLayout,
    frame: &mut Frame,
    copies: &[engine::ResumeCopy],
) -> RuntimeResult<()> {
    // gather source values before rewriting destinations
    let saved_values = copies
        .iter()
        .map(|copy| {
            let source_region = layout
                .region(copy.source)
                .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
            let destination_region = layout
                .region(copy.destination)
                .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

            if source_region.byte_len != destination_region.byte_len
                || source_region.is_word != destination_region.is_word
            {
                return Err(RuntimeError::new(Error::InvalidInstruction));
            }

            if destination_region.is_word {
                return Ok(ResumeCopyValue::Word(frame.read_word(source_region)));
            }

            Ok(ResumeCopyValue::Bytes(
                frame.region_bytes(source_region).to_vec(),
            ))
        })
        .collect::<RuntimeResult<Vec<_>>>()?;

    // write the saved values back into their destination regions
    for (copy, value) in copies.iter().zip(saved_values) {
        let destination_region = layout
            .region(copy.destination)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

        match value {
            ResumeCopyValue::Word(word) => frame.write_word(destination_region, word),
            ResumeCopyValue::Bytes(bytes) => {
                frame
                    .region_bytes_mut(destination_region)
                    .copy_from_slice(&bytes);
            }
        }
    }

    Ok(())
}

impl Interpreter {
    /// Apply one semantic resume point to one existing frame.
    pub(crate) fn apply_resume_point_to_frame(
        &mut self,
        program: &Program,
        frame_index: usize,
        resume_point_id: engine::ResumePointId,
        resume_value: Option<FrameValue>,
    ) -> RuntimeResult<()> {
        // resolve the semantic resume metadata first
        let resume_point = program
            .resume_point(resume_point_id)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let resume_transfer = resume_point
            .transfer
            .and_then(|resume_transfer| program.resume_transfer(resume_transfer))
            .cloned();
        let resume_block = program.block_for_id(resume_point.block);
        let instruction_offset = resume_point.instruction_offset as usize;
        let expected_function = program.function_for_id(resume_point.function);

        // resolve the frame and lowered target block
        let frame = self
            .frames
            .get(frame_index)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

        // require the resumed frame to match the resume metadata
        if frame.function != expected_function {
            return Err(RuntimeError::new(Error::InvalidInstruction));
        }

        let function = unsafe { frame.function_ptr.as_ref() };
        let target_index = function
            .blocks
            .iter()
            .position(|candidate| candidate.mir_block == resume_block)
            .ok_or_else(|| {
                RuntimeError::new(Error::UndefinedBlock {
                    block: resume_block,
                })
            })?;
        let target_block = &function.blocks[target_index];

        // bind resume moves and the optional resumed value
        let frame = self
            .frames
            .get_mut(frame_index)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let frame_layout = program
            .frame_layout_by_id(frame.frame_layout)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        if let Some(resume_transfer) = &resume_transfer {
            apply_resume_moves(frame_layout, frame, &resume_transfer.copies)?;
        }

        if let Some(resume_value_region) =
            resume_transfer.and_then(|resume_transfer| resume_transfer.resume_value)
        {
            let Some(frame_value) = resume_value else {
                return Err(RuntimeError::new(Error::InvalidInstruction));
            };
            let resume_value = frame_layout
                .value_for_region(resume_value_region)
                .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
            write_frame_value(program, frame, mir::Value::new(resume_value.0), frame_value)
                .map_err(RuntimeError::new)?;
        }

        // advance the frame to the resumed position
        frame.block_ptr = NonNull::from(target_block);
        frame.current_block = resume_block;
        frame.resume_pc = instruction_offset;

        Ok(())
    }

    /// Apply one semantic resume point on the resumed caller frame.
    pub(crate) fn apply_resume_point_transfer(
        &mut self,
        program: &Program,
        resume_point_id: engine::ResumePointId,
        value: Word,
    ) -> RuntimeResult<()> {
        // target the current caller frame
        let frame_index = self
            .frames
            .len()
            .checked_sub(1)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

        let resume_point = program
            .resume_point(resume_point_id)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let resume_transfer = resume_point
            .transfer
            .and_then(|resume_transfer| program.resume_transfer(resume_transfer));
        let resume_value_region = resume_transfer
            .and_then(|resume_transfer| resume_transfer.resume_value)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let frame = self
            .frames
            .get(frame_index)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let layout = program
            .frame_layout_by_id(frame.frame_layout)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let resume_value = layout
            .value_for_region(resume_value_region)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let ty = frame_value_type(program, frame, mir::Value::new(resume_value.0))
            .map_err(RuntimeError::new)?;
        let value = FrameValue::word(ty, value);

        self.apply_resume_point_to_frame(program, frame_index, resume_point_id, Some(value))
    }

    /// Apply one resume point on the resumed caller frame from one frame value.
    pub(crate) fn apply_frame_resume_point_transfer(
        &mut self,
        program: &Program,
        resume_point_id: engine::ResumePointId,
        value: FrameValue,
    ) -> RuntimeResult<()> {
        let frame_index = self
            .frames
            .len()
            .checked_sub(1)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

        self.apply_resume_point_to_frame(program, frame_index, resume_point_id, Some(value))
    }
}
