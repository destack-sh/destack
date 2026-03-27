use std::ptr::NonNull;

use destack_mir as mir;

use super::super::state::{Frame, StepState};
use super::step_instruction;
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::executable::{Executable, FunctionTarget};
use crate::execute::{Continuation, ExecutionOutcome, ExecutionOutput, YieldState};
use crate::interpreter::Interpreter;
use crate::isolate::{
    ExternalCallContext, ExternalFn, ExternalFnPtr, GlobalStorage, StringInterner,
};
use crate::options::IsolateOptions;
use destack_heap::Value;

impl Interpreter {
    /// Execute a function by name.
    ///
    /// Looks up a function in the MIR tree by name and executes it.
    pub(crate) fn run_function_by_name(
        &mut self,
        isolate_id: u64,
        executable: &Executable,
        options: &IsolateOptions,
        string_interner: &mut StringInterner,
        globals: &mut GlobalStorage,
        externals: &std::collections::HashMap<String, ExternalFn>,
        externals_by_id: &mut Vec<Option<ExternalFnPtr>>,
        memory: &mut destack_heap::MemoryContext<'_>,
        name: &str,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutput> {
        let outcome = self.run_function_by_name_yielding(
            isolate_id,
            executable,
            options,
            string_interner,
            globals,
            externals,
            externals_by_id,
            memory,
            name,
            arguments,
        )?;

        match outcome {
            ExecutionOutcome::Completed { output } => Ok(output),
            ExecutionOutcome::Yielded { .. } => {
                Err(self.make_error(executable, Error::UnexpectedYield))
            }
        }
    }

    /// Execute a function by name with yield support.
    ///
    /// Returns a yielded value when the coroutine suspends.
    pub(crate) fn run_function_by_name_yielding(
        &mut self,
        isolate_id: u64,
        executable: &Executable,
        options: &IsolateOptions,
        string_interner: &mut StringInterner,
        globals: &mut GlobalStorage,
        externals: &std::collections::HashMap<String, ExternalFn>,
        externals_by_id: &mut Vec<Option<ExternalFnPtr>>,
        memory: &mut destack_heap::MemoryContext<'_>,
        name: &str,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutcome> {
        let func_id = Self::lookup_function_id(executable, name).ok_or_else(|| {
            self.make_error(
                executable,
                Error::ExternalFunctionNotFound {
                    name: name.to_string(),
                },
            )
        })?;

        self.run_function_yielding(
            isolate_id,
            executable,
            options,
            string_interner,
            globals,
            externals,
            externals_by_id,
            memory,
            func_id,
            arguments,
        )
    }

    /// Execute a function by id.
    ///
    /// Uses direct dispatch for maximum performance.
    /// Functions are lowered when the interpreter is created.
    pub(crate) fn run_function(
        &mut self,
        isolate_id: u64,
        executable: &Executable,
        options: &IsolateOptions,
        string_interner: &mut StringInterner,
        globals: &mut GlobalStorage,
        externals: &std::collections::HashMap<String, ExternalFn>,
        externals_by_id: &mut Vec<Option<ExternalFnPtr>>,
        memory: &mut destack_heap::MemoryContext<'_>,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutput> {
        let outcome = self.run_function_yielding(
            isolate_id,
            executable,
            options,
            string_interner,
            globals,
            externals,
            externals_by_id,
            memory,
            func_id,
            arguments,
        )?;

        match outcome {
            ExecutionOutcome::Completed { output } => Ok(output),
            ExecutionOutcome::Yielded { .. } => {
                Err(self.make_error(executable, Error::UnexpectedYield))
            }
        }
    }

    /// Execute a function by id with yield support.
    ///
    /// Returns a yielded value when the coroutine suspends.
    pub(crate) fn run_function_yielding(
        &mut self,
        isolate_id: u64,
        executable: &Executable,
        options: &IsolateOptions,
        string_interner: &mut StringInterner,
        globals: &mut GlobalStorage,
        externals: &std::collections::HashMap<String, ExternalFn>,
        externals_by_id: &mut Vec<Option<ExternalFnPtr>>,
        memory: &mut destack_heap::MemoryContext<'_>,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutcome> {
        // reset interpreter state for this call
        self.statistics.reset();
        self.call_stack.clear();
        self.value_stack.clear();
        self.local_stack.clear();

        // resolve the function target before entering the main loop
        match Self::functions(executable).resolve(func_id) {
            Some(FunctionTarget::Import) => {
                // call imports directly without entering the lowered machine
                let handler =
                    self.external_for_id(executable, externals, externals_by_id, func_id)?;

                let handler = unsafe { handler.as_ref() };
                let value = {
                    let memory = memory.reborrow();
                    let mut context = ExternalCallContext::new(string_interner, memory);
                    handler(&mut context, arguments)
                }
                .map_err(|error| self.make_error(executable, error))?;

                return Ok(self.complete_execution(memory.heap_ref(), value));
            }
            Some(FunctionTarget::Lowered(_)) => {}

            // reject missing functions loudly
            None => {
                return Err(
                    self.make_error(executable, Error::UndefinedFunction { function: func_id })
                );
            }
        }

        self.run_lowered_function(
            isolate_id,
            executable,
            options,
            string_interner,
            globals,
            externals,
            externals_by_id,
            memory,
            func_id,
            arguments,
        )
    }

    /// Resume a previously yielded coroutine.
    ///
    /// The resume value is appended after explicit resume arguments.
    pub(crate) fn resume(
        &mut self,
        isolate_id: u64,
        executable: &Executable,
        options: &IsolateOptions,
        string_interner: &mut StringInterner,
        globals: &mut GlobalStorage,
        externals: &std::collections::HashMap<String, ExternalFn>,
        externals_by_id: &mut Vec<Option<ExternalFnPtr>>,
        memory: &mut destack_heap::MemoryContext<'_>,
        continuation: Continuation,
        resume_value: Value,
    ) -> RuntimeResult<ExecutionOutcome> {
        if continuation.isolate_id != isolate_id {
            return Err(self.make_error(executable, Error::InvalidContinuation));
        }

        if !self.call_stack.is_empty()
            || !self.value_stack.is_empty()
            || !self.local_stack.is_empty()
        {
            return Err(self.make_error(executable, Error::InvalidContinuation));
        }

        self.call_stack = continuation.call_stack;
        self.value_stack = continuation.value_stack;
        self.local_stack = continuation.local_stack;
        self.statistics = continuation.statistics;
        #[cfg(feature = "stats")]
        {
            self.instruction_profile = continuation.instruction_profile;
        }

        self.resume_yield_state(
            isolate_id,
            executable,
            options,
            string_interner,
            globals,
            externals,
            externals_by_id,
            memory,
            continuation.yield_state,
            resume_value,
        )
    }

    /// Resume execution from one captured yield point.
    fn resume_yield_state(
        &mut self,
        isolate_id: u64,
        executable: &Executable,
        options: &IsolateOptions,
        string_interner: &mut StringInterner,
        globals: &mut GlobalStorage,
        externals: &std::collections::HashMap<String, ExternalFn>,
        externals_by_id: &mut Vec<Option<ExternalFnPtr>>,
        memory: &mut destack_heap::MemoryContext<'_>,
        yield_state: YieldState,
        resume_value: Value,
    ) -> RuntimeResult<ExecutionOutcome> {
        self.apply_resume_point_to_frame(
            executable,
            yield_state.frame_index,
            yield_state.resume_point,
            Some(resume_value),
        )
        .map_err(|error| RuntimeError {
            error: Error::InvalidContinuation,
            call_stack: error.call_stack,
            anchor: error.anchor,
        })?;

        self.run_loop(
            isolate_id,
            executable,
            options,
            string_interner,
            globals,
            externals,
            externals_by_id,
            memory,
        )
    }

    /// Assemble a completed execution outcome.
    pub(crate) fn complete_execution(
        &mut self,
        heap: &destack_heap::Heap,
        value: Value,
    ) -> ExecutionOutcome {
        // package the final runtime output
        let output = ExecutionOutput {
            value,
            stats: self.statistics.clone().into(),
            managed_allocation_count: heap.managed_allocation_count(),
            raw_allocation_count: heap.raw_allocation_count(),
        };

        ExecutionOutcome::Completed { output }
    }

    /// Run one lowered function from its entry block.
    fn run_lowered_function(
        &mut self,
        isolate_id: u64,
        executable: &Executable,
        options: &IsolateOptions,
        string_interner: &mut StringInterner,
        globals: &mut GlobalStorage,
        externals: &std::collections::HashMap<String, ExternalFn>,
        externals_by_id: &mut Vec<Option<ExternalFnPtr>>,
        memory: &mut destack_heap::MemoryContext<'_>,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutcome> {
        // resolve the lowered entry metadata
        let function_ptr = Self::functions(executable)
            .get_ptr_for(func_id)
            .ok_or_else(|| RuntimeError::new(Error::UndefinedFunction { function: func_id }))?;
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

        // allocate live value and local storage
        let value_base = self.value_stack.len();
        let local_base = self.local_stack.len();
        self.value_stack
            .resize(value_base + value_count, Value::VOID);
        self.local_stack
            .resize(local_base + local_count, Value::VOID);
        let frame = Frame::new(
            frame_layout,
            func_id,
            function_ptr,
            entry_block_ptr,
            entry_block_id,
            entry as usize,
            value_base,
            value_count,
            local_base,
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
            frame.set_value(&mut self.value_stack, *param, value);
        }

        // push the entry frame and update stack telemetry
        self.call_stack.push(frame);
        if options.telemetry.collect_stats {
            self.statistics.max_stack_depth =
                self.statistics.max_stack_depth.max(self.call_stack.len());
        }

        self.run_loop(
            isolate_id,
            executable,
            options,
            string_interner,
            globals,
            externals,
            externals_by_id,
            memory,
        )
    }

    /// Run the interpreter loop from the current call stack.
    fn run_loop(
        &mut self,
        isolate_id: u64,
        executable: &Executable,
        options: &IsolateOptions,
        string_interner: &mut StringInterner,
        globals: &mut GlobalStorage,
        externals: &std::collections::HashMap<String, ExternalFn>,
        externals_by_id: &mut Vec<Option<ExternalFnPtr>>,
        memory: &mut destack_heap::MemoryContext<'_>,
    ) -> RuntimeResult<ExecutionOutcome> {
        let collect_stats = options.telemetry.collect_stats;
        let track_instructions = collect_stats || options.limits.max_instructions.is_some();

        // require at least one live frame before stepping
        if self.call_stack.is_empty() {
            return Err(self.make_error(executable, Error::InvalidInstruction));
        }

        loop {
            // enforce the instruction limit before stepping again
            if let Some(max) = options.limits.max_instructions
                && self.statistics.lowered_instructions_executed >= max
            {
                return Err(self.make_error(executable, Error::StepLimitExceeded));
            }

            // load the current frame position and clear any pending resume pc
            let (function_ptr, block_ptr, start_pc) = {
                let frame = self
                    .call_stack
                    .last_mut()
                    .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                let pc = frame.resume_pc;
                frame.resume_pc = 0;
                (frame.function_ptr, frame.block_ptr, pc)
            };

            let current_func: &crate::executable::Function = unsafe { function_ptr.as_ref() };
            let block: &crate::executable::Block = unsafe { block_ptr.as_ref() };
            let block_len = block.instructions.len();

            // step the current lowered block from the chosen instruction offset
            let transfer = {
                let frame_index = self.call_stack.len() - 1;
                let mut state = StepState::new(
                    executable,
                    options,
                    string_interner,
                    globals,
                    memory.reborrow(),
                    self,
                    frame_index,
                    current_func.argument_pool.as_slice(),
                    current_func.switch_case_pool.as_slice(),
                );
                step_instruction(&mut state, &block.instructions, start_pc)
            };

            // record the instructions covered by this step
            if track_instructions {
                self.statistics.lowered_instructions_executed += (block_len - start_pc) as u64;
            }
            if collect_stats && start_pc == 0 {
                self.statistics.mir_instructions_executed += block.mir_instruction_count as u64;
            }

            // reload the current function after any direct call fast path rewrites
            let current_func = {
                let frame = self
                    .call_stack
                    .last()
                    .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                unsafe { frame.function_ptr.as_ref() }
            };

            // apply the semantic transfer and stop once it produces an outcome
            if let Some(outcome) = self.apply_transfer(
                isolate_id,
                executable,
                options,
                string_interner,
                externals,
                externals_by_id,
                memory,
                current_func,
                transfer,
                collect_stats,
            )? {
                return Ok(outcome);
            }
        }
    }
}
