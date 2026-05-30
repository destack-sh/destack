use crate::Word;

use super::frame::{frame_value_from_word, materialize_value, store_frame_value};
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::machine::{Activation, Outcome};
use crate::program::Program;

impl Activation<'_> {
    /// Complete one return call.
    pub(crate) fn complete_return(
        &mut self,
        program: &Program,
        value: Word,
    ) -> RuntimeResult<Option<Outcome>> {
        // capture the returned value before the callee frame goes away
        let callee = self
            .machine
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        let return_type = program
            .tree
            .get(callee.function())
            .return_type
            .ty()
            .ok_or_else(|| Error::invalid_program("return call type"))?;
        let returned =
            frame_value_from_word(program, self.machine.frames.as_slice(), return_type, value)
                .map_err(RuntimeError::new)?;

        // pop the callee frame and release its live bytes
        let frame = self
            .machine
            .frames
            .pop()
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        self.machine.truncate_stack(frame.stack_offset);

        // complete top level execution when there is no caller
        if self.machine.frames.is_empty() {
            let value = materialize_value(
                program,
                self.heap,
                self.shared,
                self.shared_cache,
                self.shared_gc,
                returned,
            )
            .map_err(RuntimeError::new)?;
            return Ok(Some(self.machine.complete_execution(value)));
        }

        // otherwise take the call terminator edge before resuming the caller
        let caller_index = self.machine.frames.len() - 1;
        let return_state = self
            .machine
            .frames
            .get_mut(caller_index)
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?
            .return_state
            .take();

        // call terminators resume through their explicit edge
        if let Some(return_state) = return_state {
            self.machine
                .enter_caller_state(program, return_state, returned)?;
            return Ok(None);
        }

        // plain callers resume through their return destination
        let caller = self
            .machine
            .frames
            .last_mut()
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        let block = caller.block_id(program).map_err(RuntimeError::new)?;
        let point = program.point(caller.function(), block, caller.pc as u32);
        if let Some(destination) = program.return_destination_at(point)? {
            store_frame_value(program, caller, destination, returned).map_err(RuntimeError::new)?;
        }

        Ok(None)
    }
}
