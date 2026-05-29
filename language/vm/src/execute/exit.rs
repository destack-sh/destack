use crate::Word;
use destack_heap::{AllocationCache, GcWorker, Heap};

use super::frame::{frame_value_from_word, materialize_value, store_frame_value};
use crate::SharedHeap;
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::interpreter::{Interpreter, Outcome};
use crate::program::Program;

impl Interpreter {
    /// Complete one return call.
    pub(crate) fn complete_return(
        &mut self,
        program: &Program,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut AllocationCache,
        shared_gc: &GcWorker,
        value: Word,
    ) -> RuntimeResult<Option<Outcome>> {
        // capture the returned value before the callee frame goes away
        let callee = self
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        let return_type = program
            .tree
            .get(callee.function())
            .return_type
            .ty()
            .ok_or_else(|| Error::invalid_program("return call type"))?;
        let returned = frame_value_from_word(program, self.frames.as_slice(), return_type, value)
            .map_err(RuntimeError::new)?;

        // pop the callee frame and release its live bytes
        let frame = self
            .frames
            .pop()
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        self.truncate_stack(frame.stack_offset);

        // complete top level execution when there is no caller
        if self.frames.is_empty() {
            let value = materialize_value(program, heap, shared, shared_cache, shared_gc, returned)
                .map_err(RuntimeError::new)?;
            return Ok(Some(self.complete_execution(value)));
        }

        // otherwise take the call terminator edge before resuming the caller
        let caller_index = self.frames.len() - 1;
        let return_state = self
            .frames
            .get_mut(caller_index)
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?
            .return_state
            .take();

        // call terminators resume through their explicit edge
        if let Some(return_state) = return_state {
            self.enter_caller_state(program, return_state, returned)?;
            return Ok(None);
        }

        // plain callers resume through their return destination
        let caller = self
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
