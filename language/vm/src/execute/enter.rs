use std::ptr::NonNull;

use crate::Value;
use {destack_engine as engine, destack_mir as mir};

use super::bind::{
    TransferredValue, bind_parameters_from_transferred_values,
    collect_transferred_values_from_copies, collect_transferred_values_range,
    copy_values_between_frames_typed, copy_values_with_plan_typed, materialize_plain_value,
    materialize_transferred_value_for_external_call,
};
use crate::SharedHeap;
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::interpreter::{Frame, Interpreter, RunOutcome};
use crate::isolate::{ExternalCallContext, ExternalFn};
use crate::module::{ArgumentRange, CallTarget, CopyRange, Function, Module, is_invalid_value};
use crate::options::IsolateOptions;
use destack_heap::{Heap, SharedRawLimits};

/// The resolved lowered callee entry for one call.
struct LocalCallee {
    /// The MIR function id for the callee.
    function_id: mir::LocalNodeId<mir::Function>,
    /// The lowered module function pointer.
    function_ptr: NonNull<Function>,
}

impl Interpreter {
    /// Require one lowered callee from one call target.
    fn resolve_local_callee(
        module: &Module,
        function_id: mir::LocalNodeId<mir::Function>,
        target: CallTarget,
    ) -> RuntimeResult<LocalCallee> {
        // require a lowered target kind first
        let local_index = match target {
            CallTarget::Local(index) => index,
            CallTarget::Import => {
                return Err(RuntimeError::new(Error::UndefinedFunction {
                    function: function_id,
                }));
            }
        };

        // resolve the lowered function pointer
        let function_ptr = module
            .functions
            .get_ptr_by_index(local_index)
            .ok_or_else(|| {
                RuntimeError::new(Error::UndefinedFunction {
                    function: function_id,
                })
            })?;

        Ok(LocalCallee {
            function_id,
            function_ptr,
        })
    }

    /// Invoke one imported function with pre-collected argument values.
    #[allow(clippy::too_many_arguments)]
    fn call_imported_function(
        &mut self,
        module: &Module,
        function_id: mir::LocalNodeId<mir::Function>,
        externals: &std::collections::HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        arguments: &[TransferredValue],
    ) -> RuntimeResult<Value> {
        // resolve the external handler first
        let function = module.tree.get(function_id);
        let name = module.strings.get(function.name).to_string();
        let handler = externals
            .get(&name)
            .cloned()
            .ok_or_else(|| self.make_error(module, Error::ExternalFunctionNotFound { name }))?;

        // externalize argument values before crossing the runtime boundary
        let arguments = arguments
            .iter()
            .cloned()
            .map(|argument| {
                materialize_transferred_value_for_external_call(module, heap, argument)
                    .map_err(RuntimeError::new)
            })
            .collect::<RuntimeResult<Vec<_>>>()?;

        // call through the external context
        let result = {
            let mut context =
                ExternalCallContext::new(module, heap, shared, SharedRawLimits::default());
            handler(&mut context, &arguments)
        }
        .map_err(|error| self.make_error(module, error))?;

        Ok(result)
    }

    /// Push one lowered callee frame on the stack.
    #[allow(clippy::too_many_arguments)]
    fn push_local_call_frame(
        &mut self,
        module: &Module,
        heap: &destack_heap::Heap,
        options: &IsolateOptions,
        current_func: &Function,
        callee: LocalCallee,
        arguments: ArgumentRange,
        env: Option<Value>,
        copies: Option<CopyRange>,
        resume_pc: usize,
        transfer: Option<engine::ControlTransfer>,
        collect_stats: bool,
    ) -> RuntimeResult<()> {
        // reject stack overflow before allocating anything
        if self.stack.len() >= options.limits.max_stack_depth {
            return Err(self.make_error(module, Error::StackOverflow));
        }

        // resolve the lowered callee entry metadata
        let (entry, entry_block_id, entry_block_ptr, value_count, local_count) = unsafe {
            let callee = callee.function_ptr.as_ref();
            let entry = callee.entry;
            let entry_block = &callee.blocks[entry as usize];

            (
                entry,
                entry_block.mir_block,
                NonNull::from(entry_block),
                callee.value_count,
                callee.local_count,
            )
        };

        // record the caller continuation before mutating the stacks
        let caller_frame = self
            .stack
            .last_mut()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let caller_value_count = caller_frame.value_count;
        caller_frame.resume_pc = resume_pc;
        caller_frame.transfer = transfer;

        let mut new_frame = Frame::new(
            unsafe { callee.function_ptr.as_ref().frame_layout },
            callee.function_id,
            callee.function_ptr,
            entry_block_ptr,
            entry_block_id,
            entry as usize,
            value_count,
            local_count,
            env.unwrap_or(Value::VOID),
        );

        // bind arguments from the caller into the new frame
        let caller = self
            .stack
            .last()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        debug_assert!(
            caller.value_count == caller_value_count,
            "caller frame moved while binding arguments"
        );

        let new_frame_index = self.stack.len();
        let frames_ptr = self.stack.as_ptr();
        let frames_len = self.stack.len();
        if let Some(copies) = copies {
            copy_values_with_plan_typed(
                module,
                heap,
                unsafe { std::slice::from_raw_parts(frames_ptr, frames_len) },
                caller,
                &mut new_frame,
                new_frame_index,
                copies,
                current_func.copy_pool.as_slice(),
            )
            .map_err(RuntimeError::new)?;
        } else {
            let callee_function = unsafe { callee.function_ptr.as_ref() };
            copy_values_between_frames_typed(
                module,
                heap,
                unsafe { std::slice::from_raw_parts(frames_ptr, frames_len) },
                caller,
                &mut new_frame,
                new_frame_index,
                callee_function.argument_pool.as_slice(),
                callee_function.parameters,
                current_func.argument_pool.as_slice(),
                arguments,
            )
            .map_err(RuntimeError::new)?;
        }

        // push the new frame and update call stats
        self.stack.push(new_frame);
        if collect_stats {
            self.statistics.calls_made += 1;
            self.statistics.max_stack_depth = self.statistics.max_stack_depth.max(self.stack.len());
        }

        Ok(())
    }

    /// Reuse the current frame for one lowered tail call.
    fn reuse_tail_call_frame(
        &mut self,
        module: &Module,
        callee: LocalCallee,
        arguments: &[super::bind::TransferredValue],
        env: Option<Value>,
        collect_stats: bool,
    ) -> RuntimeResult<()> {
        // resolve the callee entry metadata first
        let (entry, entry_block_id, entry_block_ptr, value_count, local_count) = unsafe {
            let callee_function = callee.function_ptr.as_ref();
            let entry = callee_function.entry;
            let entry_block = &callee_function.blocks[entry as usize];

            (
                entry,
                entry_block.mir_block,
                NonNull::from(entry_block),
                callee_function.value_count,
                callee_function.local_count,
            )
        };

        // clear the current frame storage and retarget it to the callee
        let frame_index = self.stack.len() - 1;
        let frame = self
            .stack
            .last_mut()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        frame.stack_allocations.clear();
        frame.reset_slots(value_count, local_count);

        frame.frame_layout = unsafe { callee.function_ptr.as_ref().frame_layout };
        frame.function = callee.function_id;
        frame.function_ptr = callee.function_ptr;
        frame.block_ptr = entry_block_ptr;
        frame.entry_block = entry_block_id;
        frame.current_block = entry_block_id;
        frame.block_index = entry as usize;
        frame.resume_pc = 0;
        frame.transfer = None;
        frame.value_count = value_count;
        frame.local_count = local_count;
        frame.environment = env.unwrap_or(Value::VOID);

        // bind the new arguments into the reused frame
        let callee_function = unsafe { callee.function_ptr.as_ref() };
        bind_parameters_from_transferred_values(
            module,
            frame,
            frame_index,
            callee_function.argument_pool.as_slice(),
            callee_function.parameters,
            arguments,
        )
        .map_err(RuntimeError::new)?;

        // update call stats for the tail call entry
        if collect_stats {
            self.statistics.calls_made += 1;
        }

        Ok(())
    }

    /// Apply one call transfer from the current frame.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_call_transfer(
        &mut self,
        module: &Module,
        options: &IsolateOptions,
        externals: &std::collections::HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        current_func: &Function,
        function: u32,
        target: CallTarget,
        destination: mir::Value,
        arguments: ArgumentRange,
        env: Option<Value>,
        copies: Option<CopyRange>,
        resume_pc: usize,
        collect_stats: bool,
    ) -> RuntimeResult<()> {
        // resolve the target kind first
        let function_id = mir::LocalNodeId::<mir::Function>::new(function);

        // complete imported calls immediately in the caller frame
        if matches!(target, CallTarget::Import) {
            let caller = self
                .stack
                .last()
                .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

            // materialize the explicit call arguments in caller order
            let arguments = if let Some(copies) = copies {
                collect_transferred_values_from_copies(
                    module,
                    heap,
                    self.stack.as_slice(),
                    caller,
                    current_func.copy_pool.as_slice(),
                    copies,
                )?
            } else {
                collect_transferred_values_range(
                    module,
                    heap,
                    self.stack.as_slice(),
                    caller,
                    current_func.argument_pool.as_slice(),
                    arguments,
                )?
            };

            // invoke the imported callee outside the lowered machine
            let result = self.call_imported_function(
                module,
                function_id,
                externals,
                heap,
                shared,
                &arguments,
            )?;

            // write the return value into the caller result slot
            let frame = self
                .stack
                .last_mut()
                .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
            let return_destination = module
                .return_destination_for_position(
                    frame.function,
                    frame.current_block,
                    resume_pc as u32,
                )?
                .unwrap_or(destination);

            if !is_invalid_value(return_destination) {
                frame.set_value(return_destination, result);
            }

            // leave the caller positioned after the imported call
            frame.resume_pc = resume_pc;
            return Ok(());
        }

        // otherwise enter the lowered callee on a new frame
        let callee = Self::resolve_local_callee(module, function_id, target)?;
        self.push_local_call_frame(
            module,
            heap,
            options,
            current_func,
            callee,
            arguments,
            env,
            copies,
            resume_pc,
            None,
            collect_stats,
        )
    }

    /// Apply one exceptional call transfer from the current frame.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_call_branch_transfer(
        &mut self,
        module: &Module,
        options: &IsolateOptions,
        externals: &std::collections::HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        current_func: &Function,
        function: u32,
        target: CallTarget,
        arguments: ArgumentRange,
        env: Option<Value>,
        normal_resume_point: engine::ResumePointId,
        unwind_resume_point: engine::ResumePointId,
        collect_stats: bool,
    ) -> RuntimeResult<()> {
        // resolve the target kind first
        let function_id = mir::LocalNodeId::<mir::Function>::new(function);

        // imported exceptional calls resume the normal branch immediately
        if matches!(target, CallTarget::Import) {
            let caller = self
                .stack
                .last()
                .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

            // materialize the explicit branch-call arguments first
            let arguments = collect_transferred_values_range(
                module,
                heap,
                self.stack.as_slice(),
                caller,
                current_func.argument_pool.as_slice(),
                arguments,
            )?;

            // invoke the imported callee and continue through the normal branch
            let result = self.call_imported_function(
                module,
                function_id,
                externals,
                heap,
                shared,
                &arguments,
            )?;

            self.apply_resume_point_transfer(module, normal_resume_point, result)?;
            return Ok(());
        }

        // otherwise push the lowered callee and record both continuations
        let callee = Self::resolve_local_callee(module, function_id, target)?;
        let caller = self
            .stack
            .last()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

        // resume after the terminator once the branch call completes
        let resume_pc = current_func.blocks[caller.block_index].instructions.len();
        let transfer = engine::ControlTransfer::Call(engine::CallTransfer::Branch {
            normal_resume_point,
            unwind_resume_point,
        });

        self.push_local_call_frame(
            module,
            heap,
            options,
            current_func,
            callee,
            arguments,
            env,
            None,
            resume_pc,
            Some(transfer),
            collect_stats,
        )
    }

    /// Apply one tail call transfer on the current frame.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_tail_call_transfer(
        &mut self,
        module: &Module,
        externals: &std::collections::HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        current_func: &Function,
        function: u32,
        target: CallTarget,
        arguments: ArgumentRange,
        env: Option<Value>,
        copies: Option<CopyRange>,
        collect_stats: bool,
    ) -> RuntimeResult<Option<RunOutcome>> {
        // collect tail call arguments before reusing or popping the frame
        let caller = self
            .stack
            .last()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let argument_values = if let Some(copies) = copies {
            collect_transferred_values_from_copies(
                module,
                heap,
                self.stack.as_slice(),
                caller,
                current_func.copy_pool.as_slice(),
                copies,
            )?
        } else {
            collect_transferred_values_range(
                module,
                heap,
                self.stack.as_slice(),
                caller,
                current_func.argument_pool.as_slice(),
                arguments,
            )?
        };

        // resolve the callee target after the arguments are materialized
        let function_id = mir::LocalNodeId::<mir::Function>::new(function);

        // complete imported tail calls before returning to the caller
        if matches!(target, CallTarget::Import) {
            let result = self.call_imported_function(
                module,
                function_id,
                externals,
                heap,
                shared,
                &argument_values,
            )?;

            // discard the current frame before delivering the tail-call result
            let _frame = self
                .stack
                .pop()
                .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

            // complete execution immediately when there is no caller left
            if self.stack.is_empty() {
                let result = materialize_plain_value(heap, result).map_err(RuntimeError::new)?;

                return Ok(Some(self.complete_execution(heap, result)));
            }

            // otherwise write the result into the caller return destination
            let caller = self
                .stack
                .last_mut()
                .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
            if let Some(destination) = module.return_destination_for_position(
                caller.function,
                caller.current_block,
                caller.resume_pc as u32,
            )? {
                caller.set_value(destination, result);
            }

            return Ok(None);
        }

        // otherwise reuse the current frame for the lowered callee
        let transferred_arguments = if let Some(copies) = copies {
            collect_transferred_values_from_copies(
                module,
                heap,
                self.stack.as_slice(),
                caller,
                current_func.copy_pool.as_slice(),
                copies,
            )
            .map_err(RuntimeError::new)?
        } else {
            collect_transferred_values_range(
                module,
                heap,
                self.stack.as_slice(),
                caller,
                current_func.argument_pool.as_slice(),
                arguments,
            )
            .map_err(RuntimeError::new)?
        };
        let callee = Self::resolve_local_callee(module, function_id, target)?;
        self.reuse_tail_call_frame(module, callee, &transferred_arguments, env, collect_stats)?;

        Ok(None)
    }
}
