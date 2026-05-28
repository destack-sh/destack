use crate::Word;
use destack_engine as engine;

use super::frame::{FrameValue, store_frame_value};
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::interpreter::{Frame, Interpreter};
use crate::program::{FrameBinding, Program};
use destack_mir as mir;

/// Saved frame value used while binding parameters.
enum SavedFrameValue {
    /// One scalar word.
    Word(Word),
    /// One frame byte range.
    Bytes(Vec<u8>),
}

/// Bind frame parameters within one frame.
fn bind_frame_parameters(
    layout: &engine::FrameLayout,
    frame: &mut Frame,
    bindings: &[FrameBinding],
) -> RuntimeResult<()> {
    // gather source values before rewriting destinations
    let saved_values = bindings
        .iter()
        .map(|binding| {
            let source_slot = layout
                .slot(binding.source)
                .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
            let destination_slot = layout
                .slot(binding.destination)
                .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;

            if source_slot.byte_len != destination_slot.byte_len
                || source_slot.is_word != destination_slot.is_word
            {
                return Err(RuntimeError::new(Error::invalid_instruction()));
            }

            if destination_slot.is_word {
                return Ok(SavedFrameValue::Word(frame.read_word(source_slot)));
            }

            Ok(SavedFrameValue::Bytes(
                frame.slot_bytes(source_slot).to_vec(),
            ))
        })
        .collect::<RuntimeResult<Vec<_>>>()?;

    // store the saved values into their destination slots
    for (binding, value) in bindings.iter().zip(saved_values) {
        let destination_slot = layout
            .slot(binding.destination)
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;

        match value {
            SavedFrameValue::Word(word) => frame.write_word(destination_slot, word),
            SavedFrameValue::Bytes(bytes) => {
                frame
                    .slot_bytes_mut(destination_slot)
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
        // resolve target position
        let point = program
            .point_for_frame_state(frame_state_id)
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        let frame_entry = program.frame_entry(frame_state_id).cloned();
        let target_block_id = point.block;
        let pc = point.pc as usize;
        let expected_function = point.function;

        // resolve the frame and lowered target block
        let frame = self
            .frames
            .get(frame_index)
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;

        // require the frame to match the target function
        if frame.function() != expected_function {
            return Err(RuntimeError::new(Error::invalid_instruction()));
        }

        let function = program
            .functions
            .function_by_id(frame.function())
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        let target_index = function
            .blocks
            .iter()
            .position(|candidate| candidate.mir_block == target_block_id)
            .ok_or_else(|| RuntimeError::new(Error::undefined_block(target_block_id)))?;
        // bind frame parameters and the optional received value
        let frame = self
            .frames
            .get_mut(frame_index)
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        let frame_layout = program
            .frame_layout_by_id(frame.frame_layout())
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        if let Some(frame_entry) = &frame_entry {
            bind_frame_parameters(frame_layout, frame, &frame_entry.bindings)?;
        }

        if let Some(received_value_slot) =
            frame_entry.and_then(|frame_entry| frame_entry.received_value)
        {
            let Some(frame_value) = received_value else {
                return Err(RuntimeError::new(Error::invalid_instruction()));
            };
            let received_value = frame_layout
                .value_for_slot(received_value_slot)
                .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
            store_frame_value(program, frame, mir::Value::new(received_value), frame_value)
                .map_err(RuntimeError::new)?;
        }

        // advance the frame to the resumed position
        frame.block = target_index as u32;
        frame.pc = pc;

        Ok(())
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
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;

        self.enter_frame_state(program, frame_index, frame_state_id, Some(value))
    }
}
