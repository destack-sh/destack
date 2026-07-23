use destack_bytecode::CodeOffset;
use destack_program::{Continuation, Outcome, StopReason, Word};

use crate::diagnostic::{Error, Result};
use crate::machine::{Activation, Frame};

impl Activation<'_, '_> {
    /// Stop before one externally selected instruction when required.
    pub(crate) fn stop_before(
        &mut self,
        frame: Frame,
        instruction_offset: CodeOffset,
    ) -> Result<Option<Outcome<Continuation, Vec<Word>>>> {
        let Some(stop_points) = self.stop_points else {
            return Ok(None);
        };
        let Some(point) = self.point_at(frame, instruction_offset) else {
            return Ok(None);
        };

        // skip only the retained stop at its exact resume point
        let resume_skip = self.resume_skip.filter(|skip| skip.point == point);
        if resume_skip.is_some() {
            self.resume_skip = None;
        }
        let reason = stop_points.reason_at(point, resume_skip);
        let Some(reason) = reason else {
            return Ok(None);
        };

        // capture the canonical state before the selected instruction
        let frame_state = self
            .machine
            .program
            .frame_state_at(point)
            .ok_or_else(Error::invalid_continuation)?;
        let continuation = self.capture(frame_state)?;

        Ok(Some(Outcome::Stopped {
            continuation,
            reason,
        }))
    }

    /// Stop after one explicit breakpoint instruction.
    pub(crate) fn stop_after(
        &mut self,
        instruction_offset: CodeOffset,
    ) -> Result<Outcome<Continuation, Vec<Word>>> {
        let frame = self.frame();
        let point = self.point(frame, instruction_offset)?;
        let next_point = self.point(frame, frame.code_offset)?;
        let frame_state = self
            .machine
            .program
            .frame_state_at(next_point)
            .ok_or_else(Error::invalid_continuation)?;
        let continuation = self.capture(frame_state)?;

        Ok(Outcome::Stopped {
            continuation,
            reason: StopReason::Instruction { point },
        })
    }
}
