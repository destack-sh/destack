use std::ptr::NonNull;

use smallvec::SmallVec;
use {destack_engine as engine, destack_mir as mir};

use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::executable::{
    ArgumentRange, ControlFlow, CopyPair, CopyRange, FunctionTarget, INVALID_FUNCTION_INDEX,
    INVALID_VALUE_ID, is_invalid_value,
};
use destack_heap::{Value, ValueTag};

use super::super::state::{ExecutionState, Frame, resize_and_clear_stack};
use super::dispatch_instruction;
use crate::executable::Executable;
use crate::execute::{Continuation, ExecutionOutcome, ExecutionOutput, ExecutionYield, YieldState};
use crate::interpreter::Interpreter;
use crate::isolate::{
    ExternalCallContext, ExternalFn, ExternalFnPtr, GlobalStorage, StringInterner,
};
use crate::options::IsolateOptions;

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
pub(crate) fn copy_values_with_plan(
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

/// Copy resume values within one frame using one semantic resume point.
fn copy_resume_values(
    values: &mut [Value],
    frame: &Frame,
    copies: &[engine::ResumeCopy],
) -> RuntimeResult<()> {
    let copied_values = copies
        .iter()
        .map(|copy| frame.get_value(values, copy.source))
        .collect::<RuntimeResult<Vec<_>>>()?;

    for (copy, value) in copies.iter().zip(copied_values) {
        frame.set_value(values, copy.destination, value);
    }

    Ok(())
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

impl Interpreter {
    /// Return whether one value still points into frame-local storage.
    fn is_frame_local_suspend_value(value: Value) -> bool {
        matches!(value.tag(), ValueTag::StackPointer | ValueTag::LocalPointer)
    }

    /// Return the active resume point for one frame during suspension.
    fn frame_suspend_resume_point(
        executable: &Executable,
        frame: &Frame,
        frame_index: usize,
        yield_state: &YieldState,
    ) -> engine::ResumePointId {
        if frame_index == yield_state.frame_index {
            return yield_state.resume_point;
        }

        executable
            .resume_point_for_position(frame.function, frame.current_block, frame.resume_pc as u32)
            .unwrap_or_else(|| {
                panic!(
                    "missing generic resume point for frame position: {:?} {:?} {}",
                    frame.function, frame.current_block, frame.resume_pc
                )
            })
    }

    /// Return whether one frame still exposes frame-local pointers through materialized slots.
    fn frame_has_live_suspend_pointers(
        executable: &Executable,
        frame: &Frame,
        value_stack: &[Value],
        local_stack: &[Value],
        resume_point: engine::ResumePointId,
    ) -> bool {
        let safepoint = executable
            .safepoint_for_resume_point(resume_point)
            .unwrap_or_else(|| panic!("missing safepoint for resume point: {:?}", resume_point));
        let safepoint = executable
            .safepoint(safepoint)
            .unwrap_or_else(|| panic!("missing safepoint entry for id: {:?}", safepoint));
        let materialization_map = safepoint.materialization_map.unwrap_or_else(|| {
            panic!(
                "missing materialization map for safepoint: {:?}",
                safepoint.id
            )
        });
        let materialization_map = executable
            .materialization_map(materialization_map)
            .unwrap_or_else(|| {
                panic!(
                    "missing materialization map entry for id: {:?}",
                    materialization_map
                )
            });
        let materialization_frame = &materialization_map.frames[0];

        let value_slice = &value_stack[frame.value_base..frame.value_base + frame.value_count];
        let local_slice = &local_stack[frame.local_base..frame.local_base + frame.local_count];

        for slot in &materialization_frame.slots {
            let engine::MaterializationValue::FrameSlot(slot_index) = slot.value else {
                continue;
            };

            let value = frame
                .slot_value(value_slice, local_slice, slot_index)
                .unwrap_or_else(|| {
                    panic!(
                        "missing frame slot during suspend validation: {:?} {slot_index}",
                        frame.function
                    )
                });

            if Self::is_frame_local_suspend_value(value) {
                return true;
            }
        }

        false
    }

    /// Return an error when one captured frame still owns frame-local suspend state.
    fn ensure_suspendable_state(
        &self,
        executable: &Executable,
        yield_state: &YieldState,
    ) -> Result<(), Error> {
        // reject live dynamic stack-local storage across suspension
        for (frame_index, frame) in self.call_stack.iter().enumerate() {
            if frame.has_live_stack_allocations() {
                return Err(Error::SuspendWithFrameLocalState);
            }

            let resume_point =
                Self::frame_suspend_resume_point(executable, frame, frame_index, yield_state);
            if Self::frame_has_live_suspend_pointers(
                executable,
                frame,
                &self.value_stack,
                &self.local_stack,
                resume_point,
            ) {
                return Err(Error::SuspendWithFrameLocalState);
            }
        }

        Ok(())
    }

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
        // execute with yield support
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

        // reject unexpected yields
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
        // resolve function id by name
        let func_id = Self::lookup_function_id(executable, name).ok_or_else(|| {
            self.make_error(
                executable,
                Error::ExternalFunctionNotFound {
                    name: name.to_string(),
                },
            )
        })?;

        // execute function
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
        // execute with yield support
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

        // reject unexpected yields
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

        // resolve the callable target
        match Self::functions(executable).resolve(func_id) {
            // execute imported callables through the external registry
            Some(FunctionTarget::Import) => {
                let handler =
                    self.external_for_id(executable, externals, externals_by_id, func_id)?;

                // execute external handler
                // safety: handler pointer is stable for interpreter lifetime
                let handler = unsafe { handler.as_ref() };
                let value = {
                    let memory = memory.reborrow();
                    let mut context = ExternalCallContext::new(string_interner, memory);
                    handler(&mut context, arguments)
                }
                .map_err(|error| self.make_error(executable, error))?;

                return Ok(self.finish_execution(memory.heap_ref(), value));
            }

            // execute lowered callables directly
            Some(FunctionTarget::Lowered(_)) => {}

            // reject missing callables loudly
            None => {
                return Err(
                    self.make_error(executable, Error::UndefinedFunction { function: func_id })
                );
            }
        }

        // execute using direct dispatch
        self.execute_function(
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
        // validate continuation ownership
        if continuation.isolate_id != isolate_id {
            return Err(self.make_error(executable, Error::InvalidContinuation));
        }

        // ensure the interpreter is idle
        if !self.call_stack.is_empty()
            || !self.value_stack.is_empty()
            || !self.local_stack.is_empty()
        {
            return Err(self.make_error(executable, Error::InvalidContinuation));
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
        self.resume_continuation(
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

    /// Resume execution using a captured yield state.
    fn resume_continuation(
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
        // load the frame to resume
        let frame = self
            .call_stack
            .get_mut(yield_state.frame_index)
            .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;

        let resume_point = executable
            .resume_point(yield_state.resume_point)
            .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;

        // resolve the current function and resume block
        let function = unsafe { frame.function_ptr.as_ref() };
        let resume_block_index = function
            .blocks
            .iter()
            .position(|block| block.mir_block == resume_point.block)
            .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;
        let resume_block = function
            .blocks
            .get(resume_block_index)
            .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;
        let resume_transfer = resume_point
            .transfer
            .and_then(|resume_transfer| executable.resume_transfer(resume_transfer));

        // bind resume arguments
        if let Some(resume_transfer) = resume_transfer {
            copy_resume_values(&mut self.value_stack, frame, &resume_transfer.copies)?;
        }

        // bind resumed value after explicit arguments
        if let Some(resume_value_slot) =
            resume_transfer.and_then(|resume_transfer| resume_transfer.resume_value)
        {
            frame.set_value(&mut self.value_stack, resume_value_slot, resume_value);
        }

        // update frame block metadata
        frame.block_index = resume_block_index;
        frame.block_ptr = NonNull::from(resume_block);
        frame.current_block = resume_block.mir_block;
        frame.resume_pc = resume_point.instruction_offset as usize;

        // continue execution
        self.execute_loop(
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

    /// Capture execution state into a continuation.
    fn suspend_continuation(&mut self, isolate_id: u64, yield_state: YieldState) -> Continuation {
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
            isolate_id,
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
    fn finish_execution(&mut self, heap: &destack_heap::Heap, value: Value) -> ExecutionOutcome {
        // assemble output
        let output = ExecutionOutput {
            value,
            stats: self.statistics.clone().into(),
            managed_allocation_count: heap.managed_allocation_count(),
            raw_allocation_count: heap.raw_allocation_count(),
        };

        // return completed outcome
        ExecutionOutcome::Completed { output }
    }

    /// Execute a lowered function using direct dispatch.
    ///
    /// This is the core execution loop. Each iteration executes one basic block,
    /// with instruction handlers chaining via tail calls within blocks.
    fn execute_function(
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
        // get lowered function
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

        // create initial frame
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

        // bind function parameters to SSA values
        let parameter_slice = unsafe {
            let function = function_ptr.as_ref();
            function.parameters.slice(function.argument_pool.as_slice())
        };
        for (i, param) in parameter_slice.iter().enumerate() {
            let value = arguments.get(i).copied().unwrap_or(Value::VOID);
            frame.set_value(&mut self.value_stack, *param, value);
        }

        // call stack for nested function calls
        self.call_stack.push(frame);
        if options.telemetry.collect_stats {
            self.statistics.max_stack_depth =
                self.statistics.max_stack_depth.max(self.call_stack.len());
        }

        // execute until completion or yield
        self.execute_loop(
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

    /// Continue execution from the current call stack.
    ///
    /// Returns when execution completes or yields.
    fn execute_loop(
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
        // cache stats settings
        let collect_stats = options.telemetry.collect_stats;
        let track_instructions = collect_stats || options.limits.max_instructions.is_some();

        // ensure there is an active frame
        if self.call_stack.is_empty() {
            return Err(self.make_error(executable, Error::InvalidInstruction));
        }

        // main execution loop (trampoline pattern)
        loop {
            // check step limit
            if let Some(max) = options.limits.max_instructions
                && self.statistics.lowered_instructions_executed >= max
            {
                return Err(self.make_error(executable, Error::StepLimitExceeded));
            }

            // get current frame info
            let (function_ptr, block_ptr, start_pc) = {
                let frame = self
                    .call_stack
                    .last_mut()
                    .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                let pc = frame.resume_pc;
                frame.resume_pc = 0;
                (frame.function_ptr, frame.block_ptr, pc)
            };

            // resolve the current function
            // #Safety: function pointer is valid for interpreter lifetime
            let current_func: &crate::executable::Function = unsafe { function_ptr.as_ref() };

            // get current block
            // #Safety: block pointer is valid for interpreter lifetime
            let block: &crate::executable::Block = unsafe { block_ptr.as_ref() };
            let block_len = block.instructions.len();

            // execute block starting from resume_pc
            let control = {
                let frame_index = self.call_stack.len() - 1;
                let mut state = ExecutionState::new(
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
                dispatch_instruction(&mut state, &block.instructions, start_pc)
            };

            // update statistics
            if track_instructions {
                self.statistics.lowered_instructions_executed += (block_len - start_pc) as u64;
            }
            if collect_stats && start_pc == 0 {
                // only count MIR instructions on first entry to block (start_pc == 0)
                // to avoid double-counting when resuming after calls
                self.statistics.mir_instructions_executed += block.mir_instruction_count as u64;
            }

            // refresh the current function after handler chain
            let current_func = {
                let frame = self
                    .call_stack
                    .last()
                    .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                unsafe { frame.function_ptr.as_ref() }
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
                    env,
                    copies,
                    resume_pc,
                } => {
                    // resolve target function id
                    let function_id = mir::LocalNodeId::<mir::Function>::new(function);

                    // resolve the callable target
                    let resolved_target = if callee_index == INVALID_FUNCTION_INDEX {
                        Self::functions(executable).resolve(function_id)
                    } else {
                        Some(FunctionTarget::Lowered(callee_index))
                    };

                    // execute imported callables through the external registry
                    if matches!(resolved_target, Some(FunctionTarget::Import)) {
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
                        let handler = self.external_for_id(
                            executable,
                            externals,
                            externals_by_id,
                            function_id,
                        )?;

                        // execute external handler
                        // safety: handler pointer is stable for interpreter lifetime
                        let handler = unsafe { handler.as_ref() };
                        let result = {
                            let memory = memory.reborrow();
                            let mut context = ExternalCallContext::new(string_interner, memory);
                            handler(&mut context, &args)
                        }
                        .map_err(|e| self.make_error(executable, e))?;

                        // store result and continue from resume_pc
                        let frame = self
                            .call_stack
                            .last_mut()
                            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                        let return_destination = executable
                            .return_destination_for_position(
                                frame.function,
                                frame.current_block,
                                resume_pc as u32,
                            )
                            .unwrap_or(destination);
                        if !is_invalid_value(return_destination) {
                            frame.set_value(&mut self.value_stack, return_destination, result);
                        }
                        frame.resume_pc = resume_pc;
                        continue;
                    }

                    let callee_index = match resolved_target {
                        Some(FunctionTarget::Lowered(index)) => index,
                        Some(FunctionTarget::Import) | None => {
                            return Err(self.make_error(
                                executable,
                                Error::UndefinedFunction {
                                    function: function_id,
                                },
                            ));
                        }
                    };
                    let callee_ptr = Self::functions(executable)
                        .get_ptr_by_index(callee_index)
                        .ok_or_else(|| {
                            RuntimeError::new(Error::UndefinedFunction {
                                function: function_id,
                            })
                        })?;
                    let (entry, entry_block_id, entry_block_ptr, value_count, local_count) = unsafe {
                        let callee = callee_ptr.as_ref();
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

                    // check stack overflow
                    if self.call_stack.len() >= options.limits.max_stack_depth {
                        return Err(self.make_error(executable, Error::StackOverflow));
                    }

                    // store resume position in the caller frame
                    let caller_frame = self
                        .call_stack
                        .last_mut()
                        .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                    let caller_info = (caller_frame.value_base, caller_frame.value_count);
                    caller_frame.resume_pc = resume_pc;

                    // create new frame for callee
                    let value_base = self.value_stack.len();
                    let local_base = self.local_stack.len();
                    self.value_stack
                        .resize(value_base + value_count, Value::VOID);
                    self.local_stack
                        .resize(local_base + local_count, Value::VOID);
                    let new_frame = Frame::new(
                        unsafe { callee_ptr.as_ref().frame_layout },
                        function_id,
                        callee_ptr,
                        entry_block_ptr,
                        entry_block_id,
                        entry as usize,
                        value_base,
                        value_count,
                        local_base,
                        local_count,
                        env.unwrap_or(Value::VOID),
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
                        let callee = unsafe { callee_ptr.as_ref() };
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
                    env,
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

                    // execute imported callables through the external registry
                    if matches!(
                        Self::functions(executable).resolve(function_id),
                        Some(FunctionTarget::Import)
                    ) {
                        // resolve external handler
                        let handler = self.external_for_id(
                            executable,
                            externals,
                            externals_by_id,
                            function_id,
                        )?;

                        // execute external handler
                        // safety: handler pointer is stable for interpreter lifetime
                        let handler = unsafe { handler.as_ref() };
                        let result = {
                            let memory = memory.reborrow();
                            let mut context = ExternalCallContext::new(string_interner, memory);
                            handler(&mut context, &argument_values)
                        }
                        .map_err(|e| self.make_error(executable, e))?;

                        // pop completed frame
                        let frame = self
                            .call_stack
                            .pop()
                            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                        self.value_stack.truncate(frame.value_base);
                        self.local_stack.truncate(frame.local_base);

                        // if stack is empty, execution is complete
                        if self.call_stack.is_empty() {
                            return Ok(self.finish_execution(memory.heap_ref(), result));
                        }

                        // store return value in the resumed caller slot
                        let caller = self
                            .call_stack
                            .last_mut()
                            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                        if let Some(destination) = executable.return_destination_for_position(
                            caller.function,
                            caller.current_block,
                            caller.resume_pc as u32,
                        ) {
                            caller.set_value(&mut self.value_stack, destination, result);
                        }
                        continue;
                    }

                    let callee_index = match if callee_index == INVALID_FUNCTION_INDEX {
                        Self::functions(executable).resolve(function_id)
                    } else {
                        Some(FunctionTarget::Lowered(callee_index))
                    } {
                        Some(FunctionTarget::Lowered(index)) => index,
                        Some(FunctionTarget::Import) | None => {
                            return Err(self.make_error(
                                executable,
                                Error::UndefinedFunction {
                                    function: function_id,
                                },
                            ));
                        }
                    };
                    let callee_ptr = Self::functions(executable)
                        .get_ptr_by_index(callee_index)
                        .ok_or_else(|| {
                            RuntimeError::new(Error::UndefinedFunction {
                                function: function_id,
                            })
                        })?;
                    let (entry, entry_block_id, entry_block_ptr, value_count, local_count) = unsafe {
                        let callee = callee_ptr.as_ref();
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

                    // reuse the current frame for the tail call
                    let frame = self
                        .call_stack
                        .last_mut()
                        .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                    let value_base = frame.value_base;
                    let local_base = frame.local_base;
                    let value_end = value_base + value_count;
                    let local_end = local_base + local_count;

                    // clear frame-local stack allocations
                    frame.stack_values.clear();

                    // resize stacks to callee requirements
                    resize_and_clear_stack(&mut self.value_stack, value_base, value_end);
                    resize_and_clear_stack(&mut self.local_stack, local_base, local_end);

                    // update frame metadata
                    frame.function = function_id;
                    frame.function_ptr = callee_ptr;
                    frame.block_ptr = entry_block_ptr;
                    frame.entry_block = entry_block_id;
                    frame.current_block = entry_block_id;
                    frame.block_index = entry as usize;
                    frame.resume_pc = 0;
                    frame.value_count = value_count;
                    frame.local_count = local_count;
                    frame.closure_env = env.unwrap_or(Value::VOID);

                    // bind callee parameters
                    let callee = unsafe { callee_ptr.as_ref() };
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
                    resume_point,
                } => {
                    // capture yield state
                    let frame_index = self.call_stack.len() - 1;
                    let yield_state = YieldState {
                        frame_index,
                        resume_point,
                    };

                    // reject stack-local state that cannot cross suspension
                    self.ensure_suspendable_state(executable, &yield_state)
                        .map_err(|error| self.make_error(executable, error))?;

                    // externalize continuation state
                    let continuation = self.suspend_continuation(isolate_id, yield_state);

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
                        return Ok(self.finish_execution(memory.heap_ref(), value));
                    }

                    // store return value in the resumed caller slot
                    let caller = self
                        .call_stack
                        .last_mut()
                        .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                    if let Some(destination) = executable.return_destination_for_position(
                        caller.function,
                        caller.current_block,
                        caller.resume_pc as u32,
                    ) {
                        caller.set_value(&mut self.value_stack, destination, value);
                    }
                }

                ControlFlow::Error(e) => {
                    // return runtime error
                    return Err(self.make_error(executable, e));
                }
            }
        }
    }
}
