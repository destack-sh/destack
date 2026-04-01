use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::executable::{Executable, Transfer};
use crate::interpreter::{ExecutionOutcome, ExecutionYield, Interpreter, YieldState};
use crate::isolate::{ExternalFn, ExternalFnPtr, SchemaRegistry, StringInterner};
use crate::options::IsolateOptions;
use destack_engine as engine;
use destack_heap::Value;

use super::bind::copy_values_with_plan;

impl Interpreter {
    /// Apply one jump transfer within the current frame.
    fn apply_jump_transfer(
        &mut self,
        current_func: &crate::executable::Function,
        target: u32,
        copies: crate::executable::CopyRange,
    ) -> RuntimeResult<()> {
        // copy block parameters before updating the frame position
        let target_block = &current_func.blocks[target as usize];
        let frame = self
            .call_stack
            .last()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        copy_values_with_plan(
            &mut self.value_stack,
            frame,
            frame,
            copies,
            current_func.copy_pool.as_slice(),
        );

        // retarget the frame to the destination block
        let frame = self
            .call_stack
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
        executable: &Executable,
        isolate_id: u64,
        value: Value,
        resume_point: engine::ResumePointId,
    ) -> RuntimeResult<ExecutionOutcome> {
        // capture the logical yield position first
        let frame_index = self.call_stack.len() - 1;
        let yield_state = YieldState {
            frame_index,
            resume_point,
        };

        // reject unsuspendable state before capturing anything
        self.ensure_suspendable_state(executable, &yield_state)
            .map_err(|error| self.make_error(executable, error))?;

        // capture the continuation and package the yielded result
        let continuation = self.capture_continuation(isolate_id, yield_state);
        let yielded = ExecutionYield {
            value,
            continuation,
        };

        Ok(ExecutionOutcome::Yielded { yielded })
    }

    /// Apply one control transfer produced by instruction execution.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_transfer(
        &mut self,
        isolate_id: u64,
        executable: &Executable,
        options: &IsolateOptions,
        schema: &SchemaRegistry,
        string_interner: &mut StringInterner,
        externals: &std::collections::HashMap<String, ExternalFn>,
        externals_by_id: &mut Vec<Option<ExternalFnPtr>>,
        memory: &mut destack_heap::MemoryContext<'_>,
        current_func: &crate::executable::Function,
        transfer: Transfer,
        collect_stats: bool,
    ) -> RuntimeResult<Option<ExecutionOutcome>> {
        // dispatch the semantic transfer to the matching machine operation
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
                    executable,
                    options,
                    schema,
                    string_interner,
                    externals,
                    externals_by_id,
                    memory,
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
                    executable,
                    options,
                    schema,
                    string_interner,
                    externals,
                    externals_by_id,
                    memory,
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
                executable,
                schema,
                externals,
                externals_by_id,
                string_interner,
                memory,
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
                resume_point,
            } => self
                .apply_yield_transfer(executable, isolate_id, value, resume_point)
                .map(Some),
            Transfer::Throw(value) => self.apply_throw_transfer(executable, value),
            Transfer::Return(value) => self.apply_return_transfer(executable, memory, value),
            Transfer::Error(error) => Err(self.make_error(executable, error)),
        }
    }
}
