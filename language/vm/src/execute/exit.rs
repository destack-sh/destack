use crate::Word;
use destack_heap::{Heap, SharedAllocator, SharedGcWorker};

use super::frame::{frame_value_from_word, materialize_value, store_frame_value};
use crate::SharedHeap;
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::interpreter::{Interpreter, Outcome, PendingCall};
use crate::program::Program;

impl Interpreter {
    /// Complete one return call.
    pub(crate) fn complete_return(
        &mut self,
        program: &Program,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_allocator: &mut SharedAllocator,
        shared_gc: &SharedGcWorker,
        value: Word,
    ) -> RuntimeResult<Option<Outcome>> {
        // capture the returned value before the callee frame goes away
        let callee = self
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let return_type = program
            .tree
            .get(callee.function())
            .return_type
            .ty()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "return call type".to_string(),
            })?;
        let returned = frame_value_from_word(program, self.frames.as_slice(), return_type, value)
            .map_err(RuntimeError::new)?;

        // pop the callee frame and release its live bytes
        let frame = self
            .frames
            .pop()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        self.truncate_stack(frame.stack_offset);

        // complete top level execution when there is no caller
        if self.frames.is_empty() {
            let value =
                materialize_value(program, heap, shared, shared_allocator, shared_gc, returned)
                    .map_err(RuntimeError::new)?;
            return Ok(Some(self.complete_execution(value)));
        }

        // otherwise take the call terminator edge before resuming the caller
        let caller_index = self.frames.len() - 1;
        let pending_call = self
            .frames
            .get_mut(caller_index)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?
            .pending_call
            .take();

        // call terminators resume through their explicit edge
        if let Some(PendingCall { target_state }) = pending_call {
            self.enter_caller_state(program, target_state, returned)?;
            return Ok(None);
        }

        // plain callers resume through their return destination
        let caller = self
            .frames
            .last_mut()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let point = program.point(caller.function(), caller.block_id(), caller.pc as u32);
        if let Some(destination) = program.return_destination_at(point)? {
            store_frame_value(program, caller, destination, returned).map_err(RuntimeError::new)?;
        }

        Ok(None)
    }
}
