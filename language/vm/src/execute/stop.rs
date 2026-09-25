use tspp_bytecode::CodeOffset;
use tspp_program::{Outcome, Runtime, StopReason, Word};

use crate::diagnostic::Result;
use crate::machine::{Activation, Frame};

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Stop before one externally selected instruction when required.
    pub(crate) fn stop_before(
        &mut self,
        frame: Frame,
        pc: CodeOffset,
    ) -> Result<Option<Outcome<Vec<Word>>>> {
        let Some(stop_points) = self.stop_points else {
            return Ok(None);
        };
        let Some(point) = self.point_at(frame, pc) else {
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

        // retain the stopped frames in place for later continuation
        self.retain_stop(pc);

        Ok(Some(Outcome::Stopped { reason }))
    }

    /// Stop after one explicit breakpoint instruction.
    pub(crate) fn stop_after(
        &mut self,
        point_pc: CodeOffset,
        resume_pc: CodeOffset,
    ) -> Result<Outcome<Vec<Word>>> {
        let frame = self.frame();
        let point = self.point(frame, point_pc)?;
        self.retain_stop(resume_pc);

        Ok(Outcome::Stopped {
            reason: StopReason::Instruction { point },
        })
    }
}
