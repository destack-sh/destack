use crate::Value;
use destack_engine as engine;
use destack_heap::Heap;

use super::bind::{
    bind_transferred_value, capture_transferred_value, materialize_transferred_value,
};
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::interpreter::{Interpreter, RunOutcome};
use crate::module::Module;

impl Interpreter {
    /// Apply one return transfer.
    pub(crate) fn apply_return_transfer(
        &mut self,
        module: &Module,
        heap: &mut Heap,
        value: Value,
    ) -> RuntimeResult<Option<RunOutcome>> {
        // capture the returned value before the callee frame goes away
        let callee = self
            .stack
            .last()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let return_type = module
            .tree
            .get(callee.function)
            .return_type
            .ty()
            .ok_or_else(|| Error::ConcreteMirRequired {
                context: "return transfer type".to_string(),
            })?;
        let returned =
            capture_transferred_value(module, heap, self.stack.as_slice(), return_type, value)
                .map_err(RuntimeError::new)?;

        // pop the callee frame and release its live storage
        let _frame = self
            .stack
            .pop()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

        // complete top level execution when there is no caller
        if self.stack.is_empty() {
            let value =
                materialize_transferred_value(module, heap, returned).map_err(RuntimeError::new)?;
            return Ok(Some(self.complete_execution(heap, value)));
        }

        // otherwise resume the caller through its pending transfer or return slot
        let caller_index = self.stack.len() - 1;
        let transfer = self
            .stack
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
            self.apply_resume_point_transfer_typed(module, normal_resume_point, returned)?;
            return Ok(None);
        }

        // plain callers resume through their return destination slot
        let caller = self
            .stack
            .last_mut()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        if let Some(destination) = module.return_destination_for_position(
            caller.function,
            caller.current_block,
            caller.resume_pc as u32,
        )? {
            bind_transferred_value(module, caller, caller_index, destination, returned)
                .map_err(RuntimeError::new)?;
        }

        Ok(None)
    }

    /// Apply one thrown exception value through pending call continuations.
    pub(crate) fn apply_throw_transfer(
        &mut self,
        module: &Module,
        value: Value,
    ) -> RuntimeResult<Option<RunOutcome>> {
        loop {
            // discard one frame of live state first
            let _frame = self
                .stack
                .pop()
                .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

            // fail loudly once the exception escapes the whole stack
            if self.stack.is_empty() {
                return Err(self.make_error(
                    module,
                    Error::Panic {
                        message: format!("uncaught exception: {value:?}"),
                    },
                ));
            }

            // load the caller transfer before deciding how to continue unwinding
            let caller = self
                .stack
                .last_mut()
                .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
            let transfer = caller.transfer.take();

            // resume the first caller that owns an unwind continuation
            match transfer {
                Some(engine::ControlTransfer::Call(engine::CallTransfer::Branch {
                    unwind_resume_point,
                    ..
                })) => {
                    self.apply_resume_point_transfer(module, unwind_resume_point, value)?;
                    return Ok(None);
                }

                // otherwise keep unwinding outward
                None => continue,
            }
        }
    }
}
