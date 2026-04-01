use std::ptr::NonNull;

use destack_engine as engine;
use destack_heap::Value;

use super::super::state::Frame;
use super::bind::{TransferredValue, bind_transferred_value};
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::executable::Executable;
use crate::interpreter::Interpreter;
use destack_mir as mir;

/// Copy resume values within one frame using one semantic resume point.
fn copy_resume_values(
    values: &mut [Value],
    frame: &Frame,
    copies: &[engine::ResumeCopy],
) -> RuntimeResult<()> {
    // gather source values before rewriting any destination slots
    let copied_values = copies
        .iter()
        .map(|copy| frame.get_value(values, mir::Value::new(copy.source)))
        .collect::<RuntimeResult<Vec<_>>>()?;

    // write the copied values back into their destination slots
    for (copy, value) in copies.iter().zip(copied_values) {
        frame.set_value(values, mir::Value::new(copy.destination), value);
    }

    Ok(())
}

impl Interpreter {
    /// Apply one semantic resume point to one existing frame.
    pub(crate) fn apply_resume_point_to_frame(
        &mut self,
        executable: &Executable,
        frame_index: usize,
        resume_point_id: engine::ResumePointId,
        resume_value: Option<TransferredValue>,
    ) -> RuntimeResult<()> {
        // resolve the semantic resume metadata first
        let resume_point = executable
            .resume_point(resume_point_id)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let resume_transfer = resume_point
            .transfer
            .and_then(|resume_transfer| executable.resume_transfer(resume_transfer))
            .cloned();
        let resume_block = resume_point.block;
        let instruction_offset = resume_point.instruction_offset as usize;
        let expected_function = resume_point.function;

        // resolve the frame and lowered target block
        let frame = self
            .call_stack
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

        // bind resume copies and the optional resumed value
        let frame = self
            .call_stack
            .get_mut(frame_index)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        if let Some(resume_transfer) = &resume_transfer {
            copy_resume_values(&mut self.value_stack, frame, &resume_transfer.copies)?;
        }

        if let Some(resume_value_slot) =
            resume_transfer.and_then(|resume_transfer| resume_transfer.resume_value)
        {
            let Some(resume_value) = resume_value else {
                return Err(RuntimeError::new(Error::InvalidInstruction));
            };
            bind_transferred_value(
                executable,
                &mut self.value_stack,
                frame,
                frame_index,
                mir::Value::new(resume_value_slot),
                resume_value,
            )
            .map_err(RuntimeError::new)?;
        }

        // advance the frame to the resumed position
        frame.block_index = target_index;
        frame.block_ptr = NonNull::from(target_block);
        frame.current_block = resume_block;
        frame.resume_pc = instruction_offset;

        Ok(())
    }

    /// Apply one semantic resume point on the resumed caller frame.
    pub(crate) fn apply_resume_point_transfer(
        &mut self,
        executable: &Executable,
        resume_point_id: engine::ResumePointId,
        value: Value,
    ) -> RuntimeResult<()> {
        // target the current caller frame
        let frame_index = self
            .call_stack
            .len()
            .checked_sub(1)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

        // bind the resumed value through the semantic resume point
        self.apply_resume_point_to_frame(
            executable,
            frame_index,
            resume_point_id,
            Some(TransferredValue::Plain(value)),
        )
    }

    /// Apply one semantic resume point on the resumed caller frame from one owned payload.
    pub(crate) fn apply_resume_point_transfer_typed(
        &mut self,
        executable: &Executable,
        resume_point_id: engine::ResumePointId,
        value: TransferredValue,
    ) -> RuntimeResult<()> {
        let frame_index = self
            .call_stack
            .len()
            .checked_sub(1)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

        self.apply_resume_point_to_frame(executable, frame_index, resume_point_id, Some(value))
    }
}
