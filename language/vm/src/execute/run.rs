use std::ptr::NonNull;

use {destack_engine as engine, destack_mir as mir};

use super::bind::{materialize_plain_value, transferred_value_from_materialized};
use super::dispatch_instruction;
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::interpreter::{Continuation, ExecutionState, Frame, Interpreter, RunOutcome, RunOutput};
use crate::isolate::{ExternalCallContext, ExternalFn, GlobalStorage};
use crate::module::{Block, CallTarget, Function, Module};
use crate::options::IsolateOptions;
use crate::{SharedHeap, Value};
use destack_heap::{Heap, SharedRawLimits};

impl Interpreter {
    /// Execute a function by name.
    ///
    /// Looks up a function in the MIR tree by name and executes it.
    pub(crate) fn run_function_by_name(
        &mut self,
        isolate_id: engine::IsolateId,
        module: &Module,
        options: &IsolateOptions,
        globals: &mut GlobalStorage,
        externals: &std::collections::HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        name: &str,
        arguments: &[Value],
    ) -> RuntimeResult<RunOutput> {
        let outcome = self.run_function_by_name_yielding(
            isolate_id, module, options, globals, externals, heap, shared, name, arguments,
        )?;

        match outcome {
            RunOutcome::Completed { output } => Ok(output),
            RunOutcome::Yielded { .. } => Err(self.make_error(module, Error::UnexpectedYield)),
        }
    }

    /// Execute a function by name with yield support.
    ///
    /// Returns a yielded value when the coroutine suspends.
    pub(crate) fn run_function_by_name_yielding(
        &mut self,
        isolate_id: engine::IsolateId,
        module: &Module,
        options: &IsolateOptions,
        globals: &mut GlobalStorage,
        externals: &std::collections::HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        name: &str,
        arguments: &[Value],
    ) -> RuntimeResult<RunOutcome> {
        let function_id = module
            .function_id_by_name
            .get(name)
            .copied()
            .ok_or_else(|| {
                self.make_error(
                    module,
                    Error::ExternalFunctionNotFound {
                        name: name.to_string(),
                    },
                )
            })?;

        self.run_function_yielding(
            isolate_id,
            module,
            options,
            globals,
            externals,
            heap,
            shared,
            function_id,
            arguments,
        )
    }

    /// Execute a function by id.
    ///
    /// Uses direct dispatch for maximum performance.
    /// Functions are lowered when the interpreter is created.
    pub(crate) fn run_function(
        &mut self,
        isolate_id: engine::IsolateId,
        module: &Module,
        options: &IsolateOptions,
        globals: &mut GlobalStorage,
        externals: &std::collections::HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        function_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Value],
    ) -> RuntimeResult<RunOutput> {
        let outcome = self.run_function_yielding(
            isolate_id,
            module,
            options,
            globals,
            externals,
            heap,
            shared,
            function_id,
            arguments,
        )?;

        match outcome {
            RunOutcome::Completed { output } => Ok(output),
            RunOutcome::Yielded { .. } => Err(self.make_error(module, Error::UnexpectedYield)),
        }
    }

    /// Execute a function by id with yield support.
    ///
    /// Returns a yielded value when the coroutine suspends.
    pub(crate) fn run_function_yielding(
        &mut self,
        isolate_id: engine::IsolateId,
        module: &Module,
        options: &IsolateOptions,
        globals: &mut GlobalStorage,
        externals: &std::collections::HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        function_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Value],
    ) -> RuntimeResult<RunOutcome> {
        // reset interpreter state for this call
        self.statistics.reset();
        self.stack.clear();

        // resolve the function target before entering the main loop
        match module.functions.resolve(function_id) {
            Some(CallTarget::Import) => {
                // call imports directly without entering the lowered machine
                let function = module.tree.get(function_id);
                let name = module.strings.get(function.name).to_string();
                let handler = externals.get(&name).cloned().ok_or_else(|| {
                    self.make_error(module, Error::ExternalFunctionNotFound { name })
                })?;
                let value = {
                    let mut context =
                        ExternalCallContext::new(module, heap, shared, SharedRawLimits::default());
                    handler(&mut context, arguments)
                }
                .map_err(|error| self.make_error(module, error))?;

                let value = materialize_plain_value(heap, value).map_err(RuntimeError::new)?;

                return Ok(self.complete_execution(heap, value));
            }
            Some(CallTarget::Local(_)) => {}

            // reject missing functions loudly
            None => {
                return Err(self.make_error(
                    module,
                    Error::UndefinedFunction {
                        function: function_id,
                    },
                ));
            }
        }

        self.run_function_body(
            isolate_id,
            module,
            options,
            globals,
            externals,
            heap,
            shared,
            function_id,
            arguments,
        )
    }

    /// Resume a previously yielded coroutine.
    ///
    /// The resume value is appended after explicit resume arguments.
    pub(crate) fn resume(
        &mut self,
        isolate_id: engine::IsolateId,
        module: &Module,
        options: &IsolateOptions,
        globals: &mut GlobalStorage,
        externals: &std::collections::HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        continuation: Continuation,
        resume_value: engine::MaterializedValue,
    ) -> RuntimeResult<RunOutcome> {
        if continuation.isolate_id != isolate_id {
            return Err(self.make_error(module, Error::InvalidContinuation));
        }

        if !self.stack.is_empty() {
            return Err(self.make_error(module, Error::InvalidContinuation));
        }

        self.stack = continuation.stack;
        self.statistics = continuation.statistics;
        #[cfg(feature = "stats")]
        {
            self.instruction_profile = continuation.instruction_profile;
        }

        self.resume_continuation(
            isolate_id,
            module,
            options,
            globals,
            externals,
            heap,
            shared,
            continuation.resume_frame_index,
            continuation.resume_point,
            resume_value,
        )
    }

    /// Resume execution from one captured yield point.
    fn resume_continuation(
        &mut self,
        isolate_id: engine::IsolateId,
        module: &Module,
        options: &IsolateOptions,
        globals: &mut GlobalStorage,
        externals: &std::collections::HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        resume_frame_index: usize,
        resume_point: engine::ResumePointId,
        resume_value: engine::MaterializedValue,
    ) -> RuntimeResult<RunOutcome> {
        let resume_value = transferred_value_from_materialized(&resume_value)
            .map_err(|_| self.make_error(module, Error::InvalidContinuation))?;

        self.apply_resume_point_to_frame(
            module,
            resume_frame_index,
            resume_point,
            Some(resume_value),
        )
        .map_err(|error| RuntimeError {
            error: Error::InvalidContinuation,
            stack: error.stack,
            anchor: error.anchor,
        })?;

        self.run_loop(
            isolate_id, module, options, globals, externals, heap, shared,
        )
    }

    /// Assemble a completed execution outcome.
    pub(crate) fn complete_execution(
        &mut self,
        heap: &Heap,
        value: engine::MaterializedValue,
    ) -> RunOutcome {
        // package the final runtime output
        let output = RunOutput {
            value,
            stats: self.statistics.clone().into(),
            heap_allocation_count: heap.heap_allocation_count(),
            raw_allocation_count: heap.raw_allocation_count(),
        };

        RunOutcome::Completed { output }
    }

    /// Run one lowered function from its entry block.
    fn run_function_body(
        &mut self,
        isolate_id: engine::IsolateId,
        module: &Module,
        options: &IsolateOptions,
        globals: &mut GlobalStorage,
        externals: &std::collections::HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        function_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Value],
    ) -> RuntimeResult<RunOutcome> {
        // resolve the lowered entry metadata
        let function_ptr = module.functions.get_ptr_for(function_id).ok_or_else(|| {
            RuntimeError::new(Error::UndefinedFunction {
                function: function_id,
            })
        })?;
        let (entry, entry_block_id, entry_block_ptr, value_count, local_count) = unsafe {
            let function = function_ptr.as_ref();
            let entry = function.entry;
            let entry_block = &function.blocks[entry as usize];

            (
                entry,
                entry_block.mir_block,
                NonNull::from(entry_block),
                function.value_count,
                function.local_count,
            )
        };
        let frame_layout = unsafe { function_ptr.as_ref().frame_layout };

        // allocate the entry frame
        let mut frame = Frame::new(
            frame_layout,
            function_id,
            function_ptr,
            entry_block_ptr,
            entry_block_id,
            entry as usize,
            value_count,
            local_count,
            Value::VOID,
        );

        // bind explicit entry arguments into the new frame
        let parameter_slice = unsafe {
            let function = function_ptr.as_ref();
            function.parameters.slice(function.argument_pool.as_slice())
        };
        for (index, param) in parameter_slice.iter().enumerate() {
            let value = arguments.get(index).copied().unwrap_or(Value::VOID);
            frame.set_value(*param, value);
        }

        // push the entry frame and update stack telemetry
        self.stack.push(frame);
        if options.telemetry.collect_stats {
            self.statistics.max_stack_depth = self.statistics.max_stack_depth.max(self.stack.len());
        }

        self.run_loop(
            isolate_id, module, options, globals, externals, heap, shared,
        )
    }

    /// Run the interpreter loop from the current stack.
    fn run_loop(
        &mut self,
        isolate_id: engine::IsolateId,
        module: &Module,
        options: &IsolateOptions,
        globals: &mut GlobalStorage,
        externals: &std::collections::HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
    ) -> RuntimeResult<RunOutcome> {
        let collect_stats = options.telemetry.collect_stats;
        let track_instructions = collect_stats || options.limits.max_instructions.is_some();

        // require at least one live frame before stepping
        if self.stack.is_empty() {
            return Err(self.make_error(module, Error::InvalidInstruction));
        }

        loop {
            // enforce the instruction limit before stepping again
            if let Some(max) = options.limits.max_instructions
                && self.statistics.lowered_instructions_executed >= max
            {
                return Err(self.make_error(module, Error::StepLimitExceeded));
            }

            // load the current frame position and clear any pending resume pc
            let (function_ptr, block_ptr, start_pc) = {
                let frame = self
                    .stack
                    .last_mut()
                    .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                let pc = frame.resume_pc;
                frame.resume_pc = 0;
                (frame.function_ptr, frame.block_ptr, pc)
            };

            let current_func: &Function = unsafe { function_ptr.as_ref() };
            let block: &Block = unsafe { block_ptr.as_ref() };
            let block_len = block.instructions.len();

            // dispatch the current lowered block from the chosen instruction offset
            let transfer = {
                let frame_index = self.stack.len() - 1;
                let mut state = ExecutionState::new(
                    module,
                    options,
                    globals,
                    heap,
                    shared,
                    self,
                    frame_index,
                    current_func.argument_pool.as_slice(),
                );
                dispatch_instruction(&mut state, &block.instructions, start_pc)
            };

            // record the instructions covered by this dispatch
            if track_instructions {
                self.statistics.lowered_instructions_executed += (block_len - start_pc) as u64;
            }
            if collect_stats && start_pc == 0 {
                self.statistics.mir_instructions_executed += block.mir_instruction_count as u64;
            }

            // reload the current function after any direct call fast path rewrites
            let current_func = {
                let frame = self
                    .stack
                    .last()
                    .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                unsafe { frame.function_ptr.as_ref() }
            };

            // apply the transfer and stop once it produces an outcome
            if let Some(outcome) = self.apply_transfer(
                isolate_id,
                module,
                options,
                externals,
                heap,
                shared,
                current_func,
                transfer,
                collect_stats,
            )? {
                return Ok(outcome);
            }
        }
    }
}
