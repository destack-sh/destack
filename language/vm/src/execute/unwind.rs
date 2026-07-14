use destack_program::Program;

use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::machine::{Activation, Outcome};

impl Activation<'_> {
    /// Begin one language panic unwind.
    pub(crate) fn complete_panic(
        &mut self,
        program: &Program,
        error: Error,
    ) -> RuntimeResult<Option<Outcome>> {
        // abort when cleanup code starts another panic
        if self.machine.pending_unwind.is_some() {
            return Err(self.machine.runtime_error(Error::abort()));
        }

        self.machine.pending_unwind = Some(self.machine.runtime_error(error));

        self.complete_unwind(program)
    }

    /// Continue the active language panic unwind.
    pub(crate) fn complete_unwind(&mut self, program: &Program) -> RuntimeResult<Option<Outcome>> {
        if self.machine.pending_unwind.is_none() {
            return Err(RuntimeError::new(Error::invalid_instruction()));
        }

        loop {
            // release the frame that raised or resumed the panic
            let frame = self
                .machine
                .frames
                .pop()
                .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
            self.machine.truncate_stack(frame.stack_offset);

            // surface an unhandled panic after the outermost frame unwinds
            if self.machine.frames.is_empty() {
                let error = self
                    .machine
                    .pending_unwind
                    .take()
                    .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;

                return Err(error);
            }

            // enter the first caller with an explicit unwind continuation
            let caller_index = self.machine.frames.len() - 1;
            let invocation = self.machine.frames[caller_index].invocation.take();
            if let Some(invocation) = invocation {
                self.machine.enter_frame_state(
                    program,
                    caller_index,
                    invocation.unwind_state,
                    None,
                )?;

                return Ok(None);
            }
        }
    }
}
