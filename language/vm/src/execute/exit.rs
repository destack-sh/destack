use crate::Word;
use destack_engine as engine;
use destack_heap::Heap;

use super::frame::{frame_value_from_word, materialize_frame_value, write_frame_value};
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::interpreter::{Interpreter, Outcome};
use crate::program::Program;

impl Interpreter {
    /// Apply one return transfer.
    pub(crate) fn apply_return_transfer(
        &mut self,
        program: &Program,
        heap: &mut Heap,
        value: Word,
    ) -> RuntimeResult<Option<Outcome>> {
        // capture the returned value before the callee frame goes away
        let callee = self
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let return_type = program
            .tree
            .get(callee.function)
            .return_type
            .ty()
            .ok_or_else(|| Error::ConcreteMirRequired {
                context: "return transfer type".to_string(),
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
                materialize_frame_value(program, heap, returned).map_err(RuntimeError::new)?;
            return Ok(Some(self.complete_execution(heap, value)));
        }

        // otherwise resume the caller through its pending transfer or return slot
        let caller_index = self.frames.len() - 1;
        let transfer = self
            .frames
            .get_mut(caller_index)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?
            .transfer
            .take();

        // branch-call callers resume through their normal continuation
        if let Some(engine::ControlTransfer::Call(engine::CallTransfer::Branch {
            normal_resume_point,
            ..
        })) = transfer
        {
            self.apply_frame_resume_point_transfer(program, normal_resume_point, returned)?;
            return Ok(None);
        }

        // plain callers resume through their return destination
        let caller = self
            .frames
            .last_mut()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        if let Some(destination) = program.return_destination_for_position(
            caller.function,
            caller.current_block,
            caller.resume_pc as u32,
        )? {
            write_frame_value(program, caller, destination, returned).map_err(RuntimeError::new)?;
        }

        Ok(None)
    }

    /// Apply one thrown exception value through pending call continuations.
    pub(crate) fn apply_throw_transfer(
        &mut self,
        program: &Program,
        value: Word,
    ) -> RuntimeResult<Option<Outcome>> {
        loop {
            // discard one frame of live state first
            let frame = self
                .frames
                .pop()
                .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
            self.truncate_stack(frame.stack_offset);

            // fail loudly once the exception escapes the whole stack
            if self.frames.is_empty() {
                return Err(self.make_error(
                    program,
                    Error::Panic {
                        message: format!("uncaught exception: {value:?}"),
                    },
                ));
            }

            // load the caller transfer before deciding how to continue unwinding
            let caller = self
                .frames
                .last_mut()
                .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
            let transfer = caller.transfer.take();

            // resume the first caller that owns an unwind continuation
            match transfer {
                Some(engine::ControlTransfer::Call(engine::CallTransfer::Branch {
                    unwind_resume_point,
                    ..
                })) => {
                    self.apply_resume_point_transfer(program, unwind_resume_point, value)?;
                    return Ok(None);
                }

                // otherwise keep unwinding outward
                None => continue,
            }
        }
    }
}
