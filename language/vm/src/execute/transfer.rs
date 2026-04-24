use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::interpreter::{Interpreter, RunOutcome};
use crate::isolate::ExternalFn;
use crate::module::{CopyRange, Function, Module, Transfer};
use crate::options::IsolateOptions;
use crate::{SharedHeap, Value};
use destack_heap::Heap;
use {destack_engine as engine, destack_mir as mir};

use super::bind::copy_values_with_plan;

impl Interpreter {
    /// Apply one jump transfer within the current frame.
    fn apply_jump_transfer(
        &mut self,
        current_func: &Function,
        target: u32,
        copies: CopyRange,
    ) -> RuntimeResult<()> {
        // copy block parameters before updating the frame position
        let target_block = &current_func.blocks[target as usize];
        let frame = self
            .stack
            .last()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let copied = frame.clone_for_fork();
        let frame = self
            .stack
            .last_mut()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        copy_values_with_plan(&copied, frame, copies, current_func.copy_pool.as_slice())
            .map_err(RuntimeError::new)?;

        // retarget the frame to the destination block
        let frame = self
            .stack
            .last_mut()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        frame.block_index = target as usize;
        frame.block_ptr = std::ptr::NonNull::from(target_block);
        frame.current_block = target_block.mir_block;

        Ok(())
    }

    /// Apply one yield transfer and return the yielded outcome.
    fn apply_yield_transfer(
        &mut self,
        module: &Module,
        heap: &mut Heap,
        isolate_id: engine::IsolateId,
        value: Value,
        source: mir::Value,
        resume_point: engine::ResumePointId,
    ) -> RuntimeResult<RunOutcome> {
        // capture the logical yield position first
        let resume_frame_index = self.stack.len() - 1;

        // reject unsuspendable state before capturing anything
        self.ensure_suspendable_state(module, resume_frame_index, resume_point)
            .map_err(|error| self.make_error(module, error))?;

        // capture the yielded result before moving the stack into the continuation
        let frame = self
            .stack
            .last()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let yield_type =
            super::bind::frame_value_type(module, frame, source).map_err(RuntimeError::new)?;
        let value = super::bind::capture_transferred_value(
            module,
            heap,
            self.stack.as_slice(),
            yield_type,
            value,
        )
        .and_then(|value| super::bind::materialize_transferred_value(module, heap, value))
        .map_err(RuntimeError::new)?;

        // capture the continuation after packaging the yielded result
        let mut continuation =
            self.capture_continuation(isolate_id, resume_frame_index, resume_point);
        continuation.stabilize(module, heap)?;

        Ok(RunOutcome::Yielded {
            continuation,
            value,
        })
    }

    /// Apply one control transfer produced by instruction execution.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_transfer(
        &mut self,
        isolate_id: engine::IsolateId,
        module: &Module,
        options: &IsolateOptions,
        externals: &std::collections::HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        current_func: &Function,
        transfer: Transfer,
        collect_stats: bool,
    ) -> RuntimeResult<Option<RunOutcome>> {
        // dispatch the transfer to the matching machine opcode
        match transfer {
            Transfer::Jump { block, copies } => {
                self.apply_jump_transfer(current_func, block, copies)?;
                Ok(None)
            }
            Transfer::Call {
                function,
                target,
                destination,
                arguments,
                env,
                copies,
                resume_pc,
            } => {
                self.apply_call_transfer(
                    module,
                    options,
                    externals,
                    heap,
                    shared,
                    current_func,
                    function,
                    target,
                    destination,
                    arguments,
                    env,
                    copies,
                    resume_pc,
                    collect_stats,
                )?;
                Ok(None)
            }
            Transfer::CallBranch {
                function,
                target,
                arguments,
                env,
                normal_resume_point,
                unwind_resume_point,
            } => {
                self.apply_call_branch_transfer(
                    module,
                    options,
                    externals,
                    heap,
                    shared,
                    current_func,
                    function,
                    target,
                    arguments,
                    env,
                    normal_resume_point,
                    unwind_resume_point,
                    collect_stats,
                )?;
                Ok(None)
            }
            Transfer::TailCall {
                function,
                target,
                arguments,
                env,
                copies,
            } => self.apply_tail_call_transfer(
                module,
                externals,
                heap,
                shared,
                current_func,
                function,
                target,
                arguments,
                env,
                copies,
                collect_stats,
            ),
            Transfer::Yield {
                value,
                source,
                resume_point,
            } => self
                .apply_yield_transfer(module, heap, isolate_id, value, source, resume_point)
                .map(Some),
            Transfer::Throw(value) => self.apply_throw_transfer(module, value),
            Transfer::Return(value) => self.apply_return_transfer(module, heap, value),
            Transfer::Error(error) => Err(self.make_error(module, error)),
        }
    }
}
