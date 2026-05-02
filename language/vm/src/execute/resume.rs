use std::ptr::NonNull;

use crate::Word;
use destack_engine as engine;

use super::frame::{FrameValue, frame_value_type, store_frame_value};
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::interpreter::{Frame, Interpreter};
use crate::program::Program;
use destack_mir as mir;

/// Saved frame value used while applying moves.
enum SavedFrameValue {
    /// One scalar word.
    Word(Word),
    /// One frame byte range.
    Bytes(Vec<u8>),
}

/// Complete frame moves within one frame.
fn complete_frame_moves(
    layout: &engine::FrameLayout,
    frame: &mut Frame,
    moves: &[engine::FrameMove],
) -> RuntimeResult<()> {
    // gather source values before rewriting destinations
    let saved_values = moves
        .iter()
        .map(|frame_move| {
            let source_region = layout
                .region(frame_move.source)
                .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
            let destination_region = layout
                .region(frame_move.destination)
                .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

            if source_region.byte_len != destination_region.byte_len
                || source_region.is_word != destination_region.is_word
            {
                return Err(RuntimeError::new(Error::InvalidInstruction));
            }

            if destination_region.is_word {
                return Ok(SavedFrameValue::Word(frame.read_word(source_region)));
            }

            Ok(SavedFrameValue::Bytes(
                frame.region_bytes(source_region).to_vec(),
            ))
        })
        .collect::<RuntimeResult<Vec<_>>>()?;

    // store the saved values back into their destination regions
    for (frame_move, value) in moves.iter().zip(saved_values) {
        let destination_region = layout
            .region(frame_move.destination)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

        match value {
            SavedFrameValue::Word(word) => frame.write_word(destination_region, word),
            SavedFrameValue::Bytes(bytes) => {
                frame
                    .region_bytes_mut(destination_region)
                    .copy_from_slice(&bytes);
            }
        }
    }

    Ok(())
}

impl Interpreter {
    /// Enter one frame state in an existing frame.
    pub(crate) fn enter_frame_state(
        &mut self,
        program: &Program,
        frame_index: usize,
        frame_state_id: engine::FrameStateId,
        received_value: Option<FrameValue>,
    ) -> RuntimeResult<()> {
        // resolve frame-state metadata first
        let frame_state = program
            .frame_state(frame_state_id)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let point = program
            .point_for_frame_state(frame_state_id)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let frame_entry = frame_state
            .entry
            .and_then(|frame_entry| program.frame_entry(frame_entry))
            .cloned();
        let target_block_id = point.block;
        let instruction_index = point.instruction_index as usize;
        let expected_function = point.function;

        // resolve the frame and lowered target block
        let frame = self
            .frames
            .get(frame_index)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

        // require the frame to match the target state
        if frame.function() != expected_function {
            return Err(RuntimeError::new(Error::InvalidInstruction));
        }

        let function = unsafe { frame.function_ptr.as_ref() };
        let target_index = function
            .blocks
            .iter()
            .position(|candidate| candidate.mir_block == target_block_id)
            .ok_or_else(|| {
                RuntimeError::new(Error::UndefinedBlock {
                    block: target_block_id,
                })
            })?;
        let target_block = &function.blocks[target_index];

        // bind frame moves and the optional received value
        let frame = self
            .frames
            .get_mut(frame_index)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let frame_layout = program
            .frame_layout_by_id(frame.frame_layout)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        if let Some(frame_entry) = &frame_entry {
            complete_frame_moves(frame_layout, frame, &frame_entry.moves)?;
        }

        if let Some(received_value_region) =
            frame_entry.and_then(|frame_entry| frame_entry.received_value)
        {
            let Some(frame_value) = received_value else {
                return Err(RuntimeError::new(Error::InvalidInstruction));
            };
            let received_value = frame_layout
                .value_for_region(received_value_region)
                .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
            store_frame_value(program, frame, mir::Value::new(received_value), frame_value)
                .map_err(RuntimeError::new)?;
        }

        // advance the frame to the resumed position
        frame.block_ptr = NonNull::from(target_block);
        frame.resume_pc = instruction_index;

        Ok(())
    }

    /// Enter one frame state in the current caller frame with one word value.
    pub(crate) fn enter_caller_state_word(
        &mut self,
        program: &Program,
        frame_state_id: engine::FrameStateId,
        value: Word,
    ) -> RuntimeResult<()> {
        // target the current caller frame
        let frame_index = self
            .frames
            .len()
            .checked_sub(1)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

        let frame_state = program
            .frame_state(frame_state_id)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let frame_entry = frame_state
            .entry
            .and_then(|frame_entry| program.frame_entry(frame_entry));
        let received_value_region = frame_entry
            .and_then(|frame_entry| frame_entry.received_value)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let frame = self
            .frames
            .get(frame_index)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let layout = program
            .frame_layout_by_id(frame.frame_layout)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let received_value = layout
            .value_for_region(received_value_region)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let ty = frame_value_type(program, frame, mir::Value::new(received_value))
            .map_err(RuntimeError::new)?;
        let value = FrameValue::word(ty, value);

        self.enter_frame_state(program, frame_index, frame_state_id, Some(value))
    }

    /// Enter one frame state in the current caller frame from one frame value.
    pub(crate) fn enter_caller_state(
        &mut self,
        program: &Program,
        frame_state_id: engine::FrameStateId,
        value: FrameValue,
    ) -> RuntimeResult<()> {
        let frame_index = self
            .frames
            .len()
            .checked_sub(1)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

        self.enter_frame_state(program, frame_index, frame_state_id, Some(value))
    }
}
