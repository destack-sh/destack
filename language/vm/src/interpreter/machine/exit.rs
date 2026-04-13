use destack_engine as engine;
use destack_heap::Value;

use super::bind::{
    bind_transferred_value, capture_transferred_value, materialize_transferred_value_for_escape,
};
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::executable::Executable;
use crate::interpreter::{ExecutionOutcome, Interpreter};

impl Interpreter {
    /// Apply one return transfer.
    pub(crate) fn apply_return_transfer(
        &mut self,
        executable: &Executable,
        memory: &mut destack_heap::MemoryContext<'_>,
        value: Value,
    ) -> RuntimeResult<Option<ExecutionOutcome>> {
        // capture the returned value before the callee frame goes away
        let callee = self
            .call_stack
            .last()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let return_type = executable
            .tree
            .get(callee.function)
            .return_type
            .ty()
            .ok_or_else(|| Error::ConcreteMirRequired {
                context: "return transfer type".to_string(),
            })?;
        let returned = capture_transferred_value(
            executable,
            memory.heap_ref(),
            self.call_stack.as_slice(),
            return_type,
            value,
        )
        .map_err(RuntimeError::new)?;

        // pop the callee frame and release its live storage
        let frame = self
            .call_stack
            .pop()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        self.value_stack.truncate(frame.value_base);
        self.local_stack.truncate(frame.local_base);

        // complete top level execution when there is no caller
        if self.call_stack.is_empty() {
            let value =
                materialize_transferred_value_for_escape(executable, memory.heap(), returned)
                    .map_err(RuntimeError::new)?;
            return Ok(Some(self.complete_execution(memory.heap_ref(), value)));
        }

        // otherwise resume the caller through its pending transfer or return slot
        let caller_index = self.call_stack.len() - 1;
        let transfer = self
            .call_stack
            .get_mut(caller_index)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?
            .transfer
            .take();

        // branch-call callers resume through their semantic normal continuation
        if let Some(engine::FrameTransfer::Call(engine::CallTransfer::Branch {
            normal_resume_point,
            ..
        })) = transfer
        {
            self.apply_resume_point_transfer_typed(executable, normal_resume_point, returned)?;
            return Ok(None);
        }

        // plain callers resume through their return destination slot
        let caller = self
            .call_stack
            .last_mut()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        if let Some(destination) = executable.return_destination_for_position(
            caller.function,
            caller.current_block,
            caller.resume_pc as u32,
        )? {
            bind_transferred_value(
                executable,
                &mut self.value_stack,
                caller,
                caller_index,
                destination,
                returned,
            )
            .map_err(RuntimeError::new)?;
        }

        Ok(None)
    }

    /// Apply one thrown exception value through pending call continuations.
    pub(crate) fn apply_throw_transfer(
        &mut self,
        executable: &Executable,
        value: Value,
    ) -> RuntimeResult<Option<ExecutionOutcome>> {
        loop {
            // discard one frame of live state first
            let frame = self
                .call_stack
                .pop()
                .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
            self.value_stack.truncate(frame.value_base);
            self.local_stack.truncate(frame.local_base);

            // fail loudly once the exception escapes the whole stack
            if self.call_stack.is_empty() {
                return Err(self.make_error(
                    executable,
                    Error::Panic {
                        message: format!("uncaught exception: {value:?}"),
                    },
                ));
            }

            // load the caller transfer before deciding how to continue unwinding
            let caller = self
                .call_stack
                .last_mut()
                .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
            let transfer = caller.transfer.take();

            // resume the first caller that owns an unwind continuation
            match transfer {
                Some(engine::FrameTransfer::Call(engine::CallTransfer::Branch {
                    unwind_resume_point,
                    ..
                })) => {
                    self.apply_resume_point_transfer(executable, unwind_resume_point, value)?;
                    return Ok(None);
                }

                // otherwise keep unwinding outward
                None => continue,
            }
        }
    }
}
