use std::ptr::NonNull;

use destack_mir as mir;
use smallvec::SmallVec;

use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::memory::Value;

use super::interpreter::{Continuation, YieldState};
use super::threaded::{
    ArgumentRange, ControlFlow, CopyPair, CopyRange, INVALID_FUNCTION_INDEX, INVALID_VALUE_ID,
    ThreadedState, is_invalid_value,
};
use super::{
    ExecutionOutcome, ExecutionOutput, ExecutionYield, Frame, Interpreter, resize_and_clear_stack,
};

// tuning: small contiguous ranges copy faster with a loop
const CONTIGUOUS_COPY_THRESHOLD: usize = 8;

/// Copy argument values from one frame into another.
fn copy_values_between_frames(
    values: &mut [Value],
    source_frame: &Frame,
    dest_frame: &Frame,
    param_pool: &[mir::Value],
    params: ArgumentRange,
    argument_pool: &[mir::Value],
    arguments: ArgumentRange,
) {
    // resolve argument and parameter slices
    let argument_slice = arguments.slice(argument_pool);
    let param_slice = params.slice(param_pool);

    // fast path: contiguous params and arguments
    if let (Some((src_start, src_len)), Some((dest_start, dest_len))) =
        (arguments.contiguous_range(), params.contiguous_range())
        && src_len == dest_len
    {
        let src_index = source_frame.value_base + src_start as usize;
        let dest_index = dest_frame.value_base + dest_start as usize;

        // validate bounds in debug builds
        debug_assert!(
            (src_start as usize) + src_len <= source_frame.value_count,
            "ssa value out of bounds: {src_start}"
        );
        debug_assert!(
            (dest_start as usize) + src_len <= dest_frame.value_count,
            "ssa value out of bounds: {dest_start}"
        );

        // copy argument range
        let values_ptr = values.as_mut_ptr();
        unsafe {
            if std::ptr::eq(source_frame, dest_frame) {
                std::ptr::copy(
                    values_ptr.add(src_index),
                    values_ptr.add(dest_index),
                    src_len,
                );
            } else {
                std::ptr::copy_nonoverlapping(
                    values_ptr.add(src_index),
                    values_ptr.add(dest_index),
                    src_len,
                );
            }
        }
        return;
    }

    // use raw pointer to avoid repeated bounds checks
    let values_ptr = values.as_mut_ptr();

    // fast path: arguments cover all parameters
    if argument_slice.len() >= param_slice.len() {
        // move arguments into destination parameters
        for (index, param) in param_slice.iter().enumerate() {
            // load argument value
            let argument = argument_slice[index];
            let arg_index = source_frame.value_base + argument.0 as usize;
            debug_assert!(
                (argument.0 as usize) < source_frame.value_count,
                "ssa value out of bounds: {argument:?}"
            );
            let value = unsafe { *values_ptr.add(arg_index) };

            // write parameter value
            let dest_index = dest_frame.value_base + param.0 as usize;
            debug_assert!(
                (param.0 as usize) < dest_frame.value_count,
                "ssa value out of bounds: {param:?}"
            );
            unsafe {
                *values_ptr.add(dest_index) = value;
            }
        }
        return;
    }

    // move arguments into destination parameters
    for (index, param) in param_slice.iter().enumerate() {
        // load argument value
        let value = if let Some(argument) = argument_slice.get(index) {
            let arg_index = source_frame.value_base + argument.0 as usize;
            debug_assert!(
                (argument.0 as usize) < source_frame.value_count,
                "ssa value out of bounds: {argument:?}"
            );
            unsafe { *values_ptr.add(arg_index) }
        } else {
            Value::VOID
        };

        // write parameter value
        let dest_index = dest_frame.value_base + param.0 as usize;
        debug_assert!(
            (param.0 as usize) < dest_frame.value_count,
            "ssa value out of bounds: {param:?}"
        );
        unsafe {
            *values_ptr.add(dest_index) = value;
        }
    }
}

/// Copy values between frames using a precomputed plan.
pub(super) fn copy_values_with_plan(
    values: &mut [Value],
    source_frame: &Frame,
    dest_frame: &Frame,
    copies: CopyRange,
    copy_pool: &[CopyPair],
) {
    // fast path: contiguous copy pairs
    if let Some((src_start, dest_start, len)) = copies.contiguous_plan() {
        let src_index = source_frame.value_base + src_start as usize;
        let dest_index = dest_frame.value_base + dest_start as usize;

        // validate bounds in debug builds
        debug_assert!(
            (src_start as usize) + len <= source_frame.value_count,
            "ssa value out of bounds: {src_start}"
        );
        debug_assert!(
            (dest_start as usize) + len <= dest_frame.value_count,
            "ssa value out of bounds: {dest_start}"
        );

        // copy contiguous range
        let values_ptr = values.as_mut_ptr();
        unsafe {
            if std::ptr::eq(source_frame, dest_frame) {
                std::ptr::copy(values_ptr.add(src_index), values_ptr.add(dest_index), len);
            } else {
                std::ptr::copy_nonoverlapping(
                    values_ptr.add(src_index),
                    values_ptr.add(dest_index),
                    len,
                );
            }
        }
        return;
    }

    // use raw pointer to avoid repeated bounds checks
    let values_ptr = values.as_mut_ptr();

    // resolve copy pairs
    let pairs = copies.slice(copy_pool);

    // move values into destination parameters
    for pair in pairs {
        // compute destination index
        let dest_index = dest_frame.value_base + pair.dest as usize;

        // validate bounds in debug builds
        debug_assert!(
            (pair.dest as usize) < dest_frame.value_count,
            "ssa value out of bounds: {}",
            pair.dest
        );

        // load source value
        let value = if pair.src == INVALID_VALUE_ID {
            Value::VOID
        } else {
            let src_index = source_frame.value_base + pair.src as usize;
            debug_assert!(
                (pair.src as usize) < source_frame.value_count,
                "ssa value out of bounds: {}",
                pair.src
            );
            unsafe { *values_ptr.add(src_index) }
        };

        // write parameter value
        unsafe {
            *values_ptr.add(dest_index) = value;
        }
    }
}

/// Collect argument values from a frame into a smallvec.
fn collect_argument_values_range(
    values: &[Value],
    frame: &Frame,
    argument_pool: &[mir::Value],
    arguments: ArgumentRange,
) -> SmallVec<[Value; 16]> {
    // fast path: contiguous argument ids
    if let Some((start, len)) = arguments.contiguous_range() {
        debug_assert!(
            (start as usize) + len <= frame.value_count,
            "ssa value out of bounds for contiguous args"
        );
        let mut args = SmallVec::with_capacity(len);
        if len <= CONTIGUOUS_COPY_THRESHOLD {
            let start_index = frame.value_base + start as usize;
            let values_ptr = values.as_ptr();
            for offset in 0..len {
                unsafe {
                    args.push(*values_ptr.add(start_index + offset));
                }
            }
            return args;
        }
        unsafe {
            args.set_len(len);
            let src_index = frame.value_base + start as usize;
            std::ptr::copy_nonoverlapping(values.as_ptr().add(src_index), args.as_mut_ptr(), len);
        }
        return args;
    }

    // resolve argument slice
    let argument_slice = arguments.slice(argument_pool);

    // allocate argument buffer
    let mut args = SmallVec::with_capacity(argument_slice.len());

    // resolve argument values
    for argument in argument_slice {
        let index = frame.value_base + argument.0 as usize;
        debug_assert!(
            (argument.0 as usize) < frame.value_count,
            "ssa value out of bounds: {argument:?}"
        );
        let value = unsafe { *values.get_unchecked(index) };
        args.push(value);
    }

    // return arguments
    args
}

/// Collect argument values from a copy plan into a smallvec.
fn collect_argument_values_from_copies(
    values: &[Value],
    frame: &Frame,
    copy_pool: &[CopyPair],
    copies: CopyRange,
) -> SmallVec<[Value; 16]> {
    // fast path: contiguous copy pairs
    if let Some((src_start, _dest_start, len)) = copies.contiguous_plan() {
        debug_assert!(
            (src_start as usize) + len <= frame.value_count,
            "ssa value out of bounds for contiguous args"
        );
        let mut args = SmallVec::with_capacity(len);
        if len <= CONTIGUOUS_COPY_THRESHOLD {
            let start_index = frame.value_base + src_start as usize;
            let values_ptr = values.as_ptr();
            for offset in 0..len {
                unsafe {
                    args.push(*values_ptr.add(start_index + offset));
                }
            }
            return args;
        }
        unsafe {
            args.set_len(len);
            let src_index = frame.value_base + src_start as usize;
            std::ptr::copy_nonoverlapping(values.as_ptr().add(src_index), args.as_mut_ptr(), len);
        }
        return args;
    }

    // resolve copy pairs
    let pairs = copies.slice(copy_pool);

    // allocate argument buffer
    let mut args = SmallVec::with_capacity(pairs.len());

    // use raw pointer to avoid repeated bounds checks
    let values_ptr = values.as_ptr();

    // resolve argument values
    for pair in pairs {
        // load argument value
        let value = if pair.src == INVALID_VALUE_ID {
            Value::VOID
        } else {
            let src_index = frame.value_base + pair.src as usize;
            debug_assert!(
                (pair.src as usize) < frame.value_count,
                "ssa value out of bounds: {}",
                pair.src
            );
            unsafe { *values_ptr.add(src_index) }
        };
        args.push(value);
    }

    // return arguments
    args
}

/// Bind argument values to parameter slots in a frame.
fn bind_parameters_from_values(
    values: &mut [Value],
    frame: &Frame,
    param_pool: &[mir::Value],
    params: ArgumentRange,
    arguments: &[Value],
) {
    // resolve parameter slice
    let param_slice = params.slice(param_pool);

    // fast path: contiguous params with full argument list
    if let Some((dest_start, len)) = params.contiguous_range()
        && len == arguments.len()
    {
        let dest_index = frame.value_base + dest_start as usize;

        // validate bounds in debug builds
        debug_assert!(
            (dest_start as usize) + len <= frame.value_count,
            "ssa value out of bounds: {dest_start}"
        );

        // copy argument values
        let values_ptr = values.as_mut_ptr();
        unsafe {
            std::ptr::copy_nonoverlapping(arguments.as_ptr(), values_ptr.add(dest_index), len);
        }
        return;
    }

    // use raw pointer to avoid repeated bounds checks
    let values_ptr = values.as_mut_ptr();

    // fast path: arguments cover all parameters
    if arguments.len() >= param_slice.len() {
        // write parameter values
        for (index, param) in param_slice.iter().enumerate() {
            // load argument value
            let value = arguments[index];

            // write parameter value
            let dest_index = frame.value_base + param.0 as usize;
            debug_assert!(
                (param.0 as usize) < frame.value_count,
                "ssa value out of bounds: {param:?}"
            );
            unsafe {
                *values_ptr.add(dest_index) = value;
            }
        }
        return;
    }

    // write parameter values
    for (index, param) in param_slice.iter().enumerate() {
        // load argument value
        let value = arguments.get(index).copied().unwrap_or(Value::VOID);

        // write parameter value
        let dest_index = frame.value_base + param.0 as usize;
        debug_assert!(
            (param.0 as usize) < frame.value_count,
            "ssa value out of bounds: {param:?}"
        );
        unsafe {
            *values_ptr.add(dest_index) = value;
        }
    }
}

/// Resolve an argument range into a slice.
#[inline(always)]
fn argument_slice(argument_pool: &[mir::Value], arguments: ArgumentRange) -> &[mir::Value] {
    arguments.slice(argument_pool)
}

impl Interpreter {
    /// Execute a function by name.
    ///
    /// Looks up a function in the MIR tree by name and executes it.
    pub fn run_function_by_name(
        &mut self,
        name: &str,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutput> {
        // execute with yield support
        let outcome = self.run_function_by_name_yielding(name, arguments)?;

        // reject unexpected yields
        match outcome {
            ExecutionOutcome::Completed { output } => Ok(output),
            ExecutionOutcome::Yielded { .. } => Err(self.make_error(Error::UnexpectedYield)),
        }
    }

    /// Execute a function by name with yield support.
    ///
    /// Returns a yielded value when the coroutine suspends.
    pub fn run_function_by_name_yielding(
        &mut self,
        name: &str,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutcome> {
        // resolve function id by name
        let func_id = self.function_name_map.get(name).copied().ok_or_else(|| {
            self.make_error(Error::ExternalFunctionNotFound {
                name: name.to_string(),
            })
        })?;

        // execute function
        self.run_function_yielding(func_id, arguments)
    }

    /// Execute a function by id.
    ///
    /// Uses direct-threaded dispatch for maximum performance. Functions are
    /// pre-compiled to threaded form when the interpreter is created.
    pub fn run_function(
        &mut self,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutput> {
        // execute with yield support
        let outcome = self.run_function_yielding(func_id, arguments)?;

        // reject unexpected yields
        match outcome {
            ExecutionOutcome::Completed { output } => Ok(output),
            ExecutionOutcome::Yielded { .. } => Err(self.make_error(Error::UnexpectedYield)),
        }
    }

    /// Execute a function by id with yield support.
    ///
    /// Returns a yielded value when the coroutine suspends.
    pub fn run_function_yielding(
        &mut self,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutcome> {
        // reset interpreter state for this call
        self.statistics.reset();
        self.call_stack.clear();
        self.value_stack.clear();
        self.local_stack.clear();

        // load function metadata
        let function = self.tree.get(func_id);

        // handle imported or external functions
        if function.is_import() {
            // resolve external handler
            let handler = self.external_for_id(func_id)?;

            // execute external handler
            // safety: handler pointer is stable for interpreter lifetime
            let handler = unsafe { handler.as_ref() };
            let value = handler(arguments).map_err(|e| self.make_error(e))?;

            // return external result
            let outcome = self.finish_execution(value);
            return Ok(outcome);
        }

        // execute using threaded dispatch
        self.execute_threaded(func_id, arguments)
    }

    /// Resume a previously yielded coroutine.
    ///
    /// The resume value is appended after explicit resume arguments.
    pub fn resume(
        &mut self,
        continuation: Continuation,
        resume_value: Value,
    ) -> RuntimeResult<ExecutionOutcome> {
        // validate continuation ownership
        if continuation.interpreter_id != self.id {
            return Err(self.make_error(Error::InvalidContinuation));
        }

        // ensure the interpreter is idle
        if !self.call_stack.is_empty()
            || !self.value_stack.is_empty()
            || !self.local_stack.is_empty()
        {
            return Err(self.make_error(Error::InvalidContinuation));
        }

        // restore execution state
        self.call_stack = continuation.call_stack;
        self.value_stack = continuation.value_stack;
        self.local_stack = continuation.local_stack;
        self.statistics = continuation.statistics;
        #[cfg(feature = "stats")]
        {
            self.instruction_profile = continuation.instruction_profile;
        }

        // resume from the captured state
        self.resume_continuation(continuation.yield_state, resume_value)
    }

    /// Resume execution using a captured yield state.
    fn resume_continuation(
        &mut self,
        yield_state: YieldState,
        resume_value: Value,
    ) -> RuntimeResult<ExecutionOutcome> {
        // load the frame to resume
        let frame = self
            .call_stack
            .get_mut(yield_state.frame_index)
            .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;

        // resolve threaded function and resume block
        let threaded = unsafe { frame.threaded.as_ref() };
        let resume_block = threaded
            .blocks
            .get(yield_state.resume_block as usize)
            .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;

        // bind resume arguments
        copy_values_with_plan(
            &mut self.value_stack,
            frame,
            frame,
            yield_state.resume_copies,
            threaded.copy_pool.as_slice(),
        );

        // bind resumed value after explicit arguments
        if !is_invalid_value(yield_state.resume_value) {
            frame.set_value(
                &mut self.value_stack,
                yield_state.resume_value,
                resume_value,
            );
        }

        // update frame block metadata
        frame.block_index = yield_state.resume_block as usize;
        frame.block_ptr = NonNull::from(resume_block);
        frame.current_block = resume_block.mir_block;
        frame.resume_pc = 0;

        // continue execution
        self.execute_threaded_loop()
    }

    /// Capture execution state into a continuation.
    fn suspend_continuation(&mut self, yield_state: YieldState) -> Continuation {
        // move execution stacks into the continuation
        let call_stack = std::mem::take(&mut self.call_stack);
        let value_stack = std::mem::take(&mut self.value_stack);
        let local_stack = std::mem::take(&mut self.local_stack);

        // move execution statistics into the continuation
        let statistics = std::mem::take(&mut self.statistics);

        // move instruction profile state into the continuation
        #[cfg(feature = "stats")]
        let instruction_profile = self.instruction_profile.take();

        Continuation {
            interpreter_id: self.id,
            call_stack,
            value_stack,
            local_stack,
            yield_state,
            statistics,
            #[cfg(feature = "stats")]
            instruction_profile,
        }
    }

    /// Assemble a completed execution outcome.
    fn finish_execution(&mut self, value: Value) -> ExecutionOutcome {
        // assemble output
        let output = ExecutionOutput {
            value,
            statistics: self.statistics.clone(),
            heap_cells: self.managed_heap.cell_count(),
            raw_heap_cells: self.raw_heap.cell_count(),
        };

        // return completed outcome
        ExecutionOutcome::Completed { output }
    }

    /// Execute a function using direct-threaded dispatch.
    ///
    /// This is the core execution loop. Each iteration executes one basic block,
    /// with instruction handlers chaining via tail calls within blocks.
    fn execute_threaded(
        &mut self,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutcome> {
        // get pre threaded function
        let threaded_index = self
            .threaded_functions
            .index_for(func_id)
            .ok_or_else(|| RuntimeError::new(Error::UndefinedFunction { function: func_id }))?;
        let threaded = self
            .threaded_functions
            .get_by_index(threaded_index)
            .ok_or_else(|| RuntimeError::new(Error::UndefinedFunction { function: func_id }))?;

        // create initial frame
        let entry_block = &threaded.blocks[threaded.entry as usize];
        let entry_block_id = entry_block.mir_block;
        let entry_block_ptr = NonNull::from(entry_block);
        let value_base = self.value_stack.len();
        let local_base = self.local_stack.len();
        self.value_stack
            .resize(value_base + threaded.value_count, Value::VOID);
        self.local_stack
            .resize(local_base + threaded.local_count, Value::VOID);
        let threaded_ptr = NonNull::from(threaded);
        let frame = Frame::new(
            func_id,
            threaded_ptr,
            entry_block_ptr,
            entry_block_id,
            threaded.entry as usize,
            value_base,
            threaded.value_count,
            local_base,
            threaded.local_count,
        );

        // bind function parameters to SSA values
        let parameter_slice =
            argument_slice(threaded.argument_pool.as_slice(), threaded.parameters);
        for (i, param) in parameter_slice.iter().enumerate() {
            let value = arguments.get(i).copied().unwrap_or(Value::VOID);
            frame.set_value(&mut self.value_stack, *param, value);
        }

        // call stack for nested function calls
        self.call_stack.push(frame);
        if self.options.collect_stats {
            self.statistics.max_stack_depth =
                self.statistics.max_stack_depth.max(self.call_stack.len());
        }

        // execute until completion or yield
        self.execute_threaded_loop()
    }

    /// Continue execution from the current call stack.
    ///
    /// Returns when execution completes or yields.
    fn execute_threaded_loop(&mut self) -> RuntimeResult<ExecutionOutcome> {
        // cache stats settings
        let collect_stats = self.options.collect_stats;
        let track_instructions = collect_stats || self.options.max_instructions.is_some();

        // ensure there is an active frame
        if self.call_stack.is_empty() {
            return Err(self.make_error(Error::InvalidInstruction));
        }

        // main execution loop (trampoline pattern)
        loop {
            // check step limit
            if let Some(max) = self.options.max_instructions
                && self.statistics.threaded_instructions_executed >= max
            {
                return Err(self.make_error(Error::StepLimitExceeded));
            }

            // get current frame info
            let (threaded_ptr, block_ptr, start_pc) = {
                let frame = self
                    .call_stack
                    .last_mut()
                    .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                let pc = frame.resume_pc;
                frame.resume_pc = 0;
                (frame.threaded, frame.block_ptr, pc)
            };

            // resolve threaded function
            // #Safety: threaded pointer is valid for interpreter lifetime
            let current_func = unsafe { threaded_ptr.as_ref() };

            // get current block
            // #Safety: block pointer is valid for interpreter lifetime
            let block = unsafe { block_ptr.as_ref() };
            let block_len = block.instructions.len();

            // execute block starting from resume_pc
            let control = {
                let frame_index = self.call_stack.len() - 1;
                let mut state = ThreadedState::new(
                    self,
                    frame_index,
                    current_func.argument_pool.as_slice(),
                    current_func.switch_case_pool.as_slice(),
                );
                (block.instructions[start_pc].handler)(&mut state, &block.instructions, start_pc)
            };

            // update statistics
            if track_instructions {
                self.statistics.threaded_instructions_executed += (block_len - start_pc) as u64;
            }
            if collect_stats && start_pc == 0 {
                // only count MIR instructions on first entry to block (start_pc == 0)
                // to avoid double-counting when resuming after calls
                self.statistics.mir_instructions_executed += block.mir_instruction_count as u64;
            }

            // refresh threaded function after handler chain (tail calls can swap frames)
            let current_func = {
                let frame = self
                    .call_stack
                    .last()
                    .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                unsafe { frame.threaded.as_ref() }
            };

            // handle control flow
            match control {
                ControlFlow::Jump {
                    block: target,
                    copies,
                } => {
                    // bind block parameters for target block
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

                    // update current block
                    let frame = self
                        .call_stack
                        .last_mut()
                        .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                    frame.block_index = target as usize;
                    frame.block_ptr = NonNull::from(target_block);
                    frame.current_block = target_block.mir_block;
                }

                ControlFlow::Call {
                    function,
                    callee_index,
                    destination,
                    arguments,
                    copies,
                    resume_pc,
                } => {
                    // resolve target function id
                    let function_id = mir::LocalNodeId::<mir::Function>::new(function);

                    // check for external or imported function
                    if callee_index == INVALID_FUNCTION_INDEX
                        && self.threaded_functions.is_import(function_id)
                    {
                        let caller = self
                            .call_stack
                            .last()
                            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

                        // resolve arguments from caller
                        let args = if let Some(copies) = copies {
                            collect_argument_values_from_copies(
                                &self.value_stack,
                                caller,
                                current_func.copy_pool.as_slice(),
                                copies,
                            )
                        } else {
                            collect_argument_values_range(
                                &self.value_stack,
                                caller,
                                current_func.argument_pool.as_slice(),
                                arguments,
                            )
                        };

                        // resolve external handler
                        let handler = self.external_for_id(function_id)?;

                        // execute external handler
                        // safety: handler pointer is stable for interpreter lifetime
                        let handler = unsafe { handler.as_ref() };
                        let result = handler(&args).map_err(|e| self.make_error(e))?;

                        // store result and continue from resume_pc
                        let frame = self
                            .call_stack
                            .last_mut()
                            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                        if !is_invalid_value(destination) {
                            frame.set_value(&mut self.value_stack, destination, result);
                        }
                        frame.resume_pc = resume_pc;
                        continue;
                    }

                    // resolve callee index
                    let callee_index = if callee_index == INVALID_FUNCTION_INDEX {
                        self.threaded_functions.index_for(function_id)
                    } else {
                        Some(callee_index)
                    };

                    // get callee's threaded function
                    let callee_index = callee_index.ok_or_else(|| {
                        RuntimeError::new(Error::UndefinedFunction {
                            function: function_id,
                        })
                    })?;
                    let callee = self
                        .threaded_functions
                        .get_by_index(callee_index)
                        .ok_or_else(|| {
                            RuntimeError::new(Error::UndefinedFunction {
                                function: function_id,
                            })
                        })?;

                    // check stack overflow
                    if self.call_stack.len() >= self.options.max_stack_depth {
                        return Err(self.make_error(Error::StackOverflow));
                    }

                    // store return destination and resume_pc in caller frame
                    let caller_frame = self
                        .call_stack
                        .last_mut()
                        .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                    let caller_info = (caller_frame.value_base, caller_frame.value_count);
                    caller_frame.return_destination = destination;
                    caller_frame.resume_pc = resume_pc;

                    // create new frame for callee
                    let value_base = self.value_stack.len();
                    let local_base = self.local_stack.len();
                    self.value_stack
                        .resize(value_base + callee.value_count, Value::VOID);
                    self.local_stack
                        .resize(local_base + callee.local_count, Value::VOID);
                    let entry_block = &callee.blocks[callee.entry as usize];
                    let entry_block_id = entry_block.mir_block;
                    let entry_block_ptr = NonNull::from(entry_block);
                    let callee_ptr = NonNull::from(callee);
                    let new_frame = Frame::new(
                        function_id,
                        callee_ptr,
                        entry_block_ptr,
                        entry_block_id,
                        callee.entry as usize,
                        value_base,
                        callee.value_count,
                        local_base,
                        callee.local_count,
                    );

                    // bind callee's parameters
                    let caller = self
                        .call_stack
                        .last()
                        .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                    debug_assert!(
                        caller.value_base == caller_info.0 && caller.value_count == caller_info.1,
                        "caller frame moved while binding arguments"
                    );
                    if let Some(copies) = copies {
                        // copy with precomputed plan
                        copy_values_with_plan(
                            &mut self.value_stack,
                            caller,
                            &new_frame,
                            copies,
                            current_func.copy_pool.as_slice(),
                        );
                    } else {
                        // copy with parameter slices
                        copy_values_between_frames(
                            &mut self.value_stack,
                            caller,
                            &new_frame,
                            callee.argument_pool.as_slice(),
                            callee.parameters,
                            current_func.argument_pool.as_slice(),
                            arguments,
                        );
                    }

                    // push callee frame
                    self.call_stack.push(new_frame);
                    if collect_stats {
                        self.statistics.calls_made += 1;
                        self.statistics.max_stack_depth =
                            self.statistics.max_stack_depth.max(self.call_stack.len());
                    }
                }

                ControlFlow::TailCall {
                    function,
                    callee_index,
                    arguments,
                    copies,
                } => {
                    // resolve target function id
                    let function_id = mir::LocalNodeId::<mir::Function>::new(function);

                    // collect argument values from the current frame
                    let caller = self
                        .call_stack
                        .last()
                        .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                    let argument_values = if let Some(copies) = copies {
                        collect_argument_values_from_copies(
                            &self.value_stack,
                            caller,
                            current_func.copy_pool.as_slice(),
                            copies,
                        )
                    } else {
                        collect_argument_values_range(
                            &self.value_stack,
                            caller,
                            current_func.argument_pool.as_slice(),
                            arguments,
                        )
                    };

                    // check for external or imported function
                    if self.threaded_functions.is_import(function_id) {
                        // resolve external handler
                        let handler = self.external_for_id(function_id)?;

                        // execute external handler
                        // safety: handler pointer is stable for interpreter lifetime
                        let handler = unsafe { handler.as_ref() };
                        let result = handler(&argument_values).map_err(|e| self.make_error(e))?;

                        // pop completed frame
                        let frame = self
                            .call_stack
                            .pop()
                            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                        self.value_stack.truncate(frame.value_base);
                        self.local_stack.truncate(frame.local_base);

                        // if stack is empty, execution is complete
                        if self.call_stack.is_empty() {
                            return Ok(self.finish_execution(result));
                        }

                        // store return value in caller's frame
                        let caller = self
                            .call_stack
                            .last_mut()
                            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                        let dest = caller.return_destination;
                        if !is_invalid_value(dest) {
                            caller.return_destination = mir::Value(INVALID_VALUE_ID);
                            caller.set_value(&mut self.value_stack, dest, result);
                        }
                        continue;
                    }

                    // resolve callee index
                    let callee_index = if callee_index == INVALID_FUNCTION_INDEX {
                        self.threaded_functions.index_for(function_id)
                    } else {
                        Some(callee_index)
                    };

                    // get callee's threaded function
                    let callee_index = callee_index.ok_or_else(|| {
                        RuntimeError::new(Error::UndefinedFunction {
                            function: function_id,
                        })
                    })?;
                    let callee = self
                        .threaded_functions
                        .get_by_index(callee_index)
                        .ok_or_else(|| {
                            RuntimeError::new(Error::UndefinedFunction {
                                function: function_id,
                            })
                        })?;

                    // reuse the current frame for the tail call
                    let frame = self
                        .call_stack
                        .last_mut()
                        .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                    let value_base = frame.value_base;
                    let local_base = frame.local_base;
                    let value_end = value_base + callee.value_count;
                    let local_end = local_base + callee.local_count;

                    // clear frame-local stack allocations
                    frame.stack_cells.clear();

                    // resize stacks to callee requirements
                    resize_and_clear_stack(&mut self.value_stack, value_base, value_end);
                    resize_and_clear_stack(&mut self.local_stack, local_base, local_end);

                    // update frame metadata
                    let entry_block = &callee.blocks[callee.entry as usize];
                    frame.function = function_id;
                    frame.threaded = NonNull::from(callee);
                    frame.block_ptr = NonNull::from(entry_block);
                    frame.entry_block = entry_block.mir_block;
                    frame.current_block = entry_block.mir_block;
                    frame.block_index = callee.entry as usize;
                    frame.resume_pc = 0;
                    frame.value_count = callee.value_count;
                    frame.local_count = callee.local_count;

                    // bind callee parameters
                    bind_parameters_from_values(
                        &mut self.value_stack,
                        frame,
                        callee.argument_pool.as_slice(),
                        callee.parameters,
                        &argument_values,
                    );

                    // update statistics
                    if collect_stats {
                        self.statistics.calls_made += 1;
                    }
                }

                ControlFlow::Yield {
                    value,
                    resume_block,
                    resume_copies,
                    resume_value,
                } => {
                    // capture yield state
                    let frame_index = self.call_stack.len() - 1;
                    let yield_state = YieldState {
                        frame_index,
                        resume_block,
                        resume_copies,
                        resume_value,
                    };

                    // externalize continuation state
                    let continuation = self.suspend_continuation(yield_state);

                    // return yielded value
                    let yielded = ExecutionYield {
                        value,
                        continuation,
                    };
                    return Ok(ExecutionOutcome::Yielded { yielded });
                }

                ControlFlow::Return(value) => {
                    // pop completed frame
                    let frame = self
                        .call_stack
                        .pop()
                        .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                    self.value_stack.truncate(frame.value_base);
                    self.local_stack.truncate(frame.local_base);

                    // if stack is empty, execution is complete
                    if self.call_stack.is_empty() {
                        return Ok(self.finish_execution(value));
                    }

                    // store return value in caller's frame
                    let caller = self
                        .call_stack
                        .last_mut()
                        .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                    let dest = caller.return_destination;
                    if !is_invalid_value(dest) {
                        caller.return_destination = mir::Value(INVALID_VALUE_ID);
                        caller.set_value(&mut self.value_stack, dest, value);
                    }
                }

                ControlFlow::Error(e) => {
                    // return runtime error
                    return Err(self.make_error(e));
                }
            }
        }
    }
}
