use std::ptr::NonNull;

use destack_mir as mir;
use smallvec::SmallVec;

use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::memory::Value;

use super::threaded::{
    ArgumentRange, ControlFlow, CopyPair, CopyRange, INVALID_FUNCTION_INDEX, INVALID_VALUE_ID,
    ThreadedState, is_invalid_value,
};
use super::{ExecutionOutput, Frame, Interpreter};

/// Copy argument values from one frame into another.
fn copy_values_between_frames(
    values: &mut [Value],
    source_frame: &Frame,
    dest_frame: &Frame,
    params: &[mir::Value],
    arguments: &[mir::Value],
) {
    // use raw pointer to avoid repeated bounds checks
    let values_ptr = values.as_mut_ptr();

    // move arguments into destination parameters
    for (index, param) in params.iter().enumerate() {
        // load argument value
        let value = if let Some(argument) = arguments.get(index) {
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
fn copy_values_with_plan(
    values: &mut [Value],
    source_frame: &Frame,
    dest_frame: &Frame,
    pairs: &[CopyPair],
) {
    // fast path: contiguous copy pairs
    if let Some((src_start, dest_start, len)) = contiguous_copy_plan(pairs) {
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

/// Detect contiguous copy pairs for bulk copying.
fn contiguous_copy_plan(pairs: &[CopyPair]) -> Option<(u32, u32, usize)> {
    // require at least one pair
    let first = pairs.first()?;

    // reject missing sources
    if first.src == INVALID_VALUE_ID {
        return None;
    }

    let src_start = first.src;
    let dest_start = first.dest;

    // validate contiguous sequence
    for (offset, pair) in pairs.iter().enumerate() {
        let expected_src = src_start + offset as u32;
        let expected_dest = dest_start + offset as u32;
        if pair.src != expected_src || pair.dest != expected_dest {
            return None;
        }
    }

    Some((src_start, dest_start, pairs.len()))
}

/// Collect argument values from a frame into a smallvec.
fn collect_argument_values(
    values: &[Value],
    frame: &Frame,
    arguments: &[mir::Value],
) -> SmallVec<[Value; 16]> {
    // allocate argument buffer
    let mut args = SmallVec::with_capacity(arguments.len());

    // resolve argument values
    for argument in arguments {
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
    pairs: &[CopyPair],
) -> SmallVec<[Value; 16]> {
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
    params: &[mir::Value],
    arguments: &[Value],
) {
    // use raw pointer to avoid repeated bounds checks
    let values_ptr = values.as_mut_ptr();

    // write parameter values
    for (index, param) in params.iter().enumerate() {
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
fn argument_slice(argument_pool: &[mir::Value], arguments: ArgumentRange) -> &[mir::Value] {
    arguments.slice(argument_pool)
}

/// Resolve a copy range into a slice.
fn copy_pairs(copy_pool: &[CopyPair], copies: CopyRange) -> &[CopyPair] {
    copies.slice(copy_pool)
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
        // resolve function id by name
        let func_id = self.function_name_map.get(name).copied().ok_or_else(|| {
            self.make_error(Error::ExternalFunctionNotFound {
                name: name.to_string(),
            })
        })?;

        // execute function
        self.run_function(func_id, arguments)
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
        // reset statistics for this call
        self.statistics.reset();
        self.call_stack.clear();
        self.value_stack.clear();
        self.local_stack.clear();

        // load function metadata
        let function = self.tree.get(func_id);

        // handle imported or external functions
        if function.is_import() {
            // resolve external handler
            let name = self.strings.get(function.name).to_string();
            let handler = self
                .externals
                .get(&name)
                .ok_or_else(|| self.make_error(Error::ExternalFunctionNotFound { name }))?;

            // execute external handler
            let value = handler(arguments).map_err(|e| self.make_error(e))?;

            // return external result
            return Ok(ExecutionOutput {
                value,
                statistics: self.statistics.clone(),
                heap_cells: self.managed_heap.cell_count(),
                raw_heap_cells: self.raw_heap.cell_count(),
            });
        }

        // execute using threaded dispatch
        let value = self.execute_threaded(func_id, arguments)?;

        // return threaded result
        Ok(ExecutionOutput {
            value,
            statistics: self.statistics.clone(),
            heap_cells: self.managed_heap.cell_count(),
            raw_heap_cells: self.raw_heap.cell_count(),
        })
    }

    /// Execute a function using direct-threaded dispatch.
    ///
    /// This is the core execution loop. Each iteration executes one basic block,
    /// with instruction handlers chaining via tail calls within blocks.
    fn execute_threaded(
        &mut self,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Value],
    ) -> RuntimeResult<Value> {
        // get pre threaded function
        let threaded_index = self
            .threaded_functions
            .index_for(func_id)
            .ok_or_else(|| RuntimeError::new(Error::UndefinedFunction { function: func_id }))?;
        let threaded = self
            .threaded_functions
            .get_by_index(threaded_index)
            .ok_or_else(|| RuntimeError::new(Error::UndefinedFunction { function: func_id }))?
            .clone();

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
        let threaded_ptr = NonNull::from(threaded.as_ref());
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
        self.statistics.max_stack_depth =
            self.statistics.max_stack_depth.max(self.call_stack.len());

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
            self.statistics.threaded_instructions_executed += (block_len - start_pc) as u64;
            // only count MIR instructions on first entry to block (start_pc == 0)
            // to avoid double-counting when resuming after calls
            if start_pc == 0 {
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
                    let copy_pairs = copy_pairs(current_func.copy_pool.as_slice(), copies);
                    let frame = self
                        .call_stack
                        .last()
                        .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                    copy_values_with_plan(&mut self.value_stack, frame, frame, copy_pairs);

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
                    let func: &mir::Function = self.tree.get(function_id);
                    if func.is_import() {
                        let caller = self
                            .call_stack
                            .last()
                            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

                        // resolve arguments from caller
                        let args = if let Some(copies) = copies {
                            let copy_pairs = copy_pairs(current_func.copy_pool.as_slice(), copies);
                            collect_argument_values_from_copies(
                                &self.value_stack,
                                caller,
                                copy_pairs,
                            )
                        } else {
                            let argument_values =
                                argument_slice(current_func.argument_pool.as_slice(), arguments);
                            collect_argument_values(&self.value_stack, caller, argument_values)
                        };

                        // resolve external handler
                        let name = self.strings.get(func.name).to_string();
                        let handler = self.externals.get(&name).ok_or_else(|| {
                            self.make_error(Error::ExternalFunctionNotFound { name })
                        })?;

                        // execute external handler
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

                    // resolve argument values if needed
                    let argument_values = if copies.is_some() {
                        &[]
                    } else {
                        argument_slice(current_func.argument_pool.as_slice(), arguments)
                    };

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
                    caller_frame.return_destination = if is_invalid_value(destination) {
                        None
                    } else {
                        Some(destination)
                    };
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
                    let callee_ptr = NonNull::from(callee.as_ref());
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
                        let copy_pairs = copy_pairs(current_func.copy_pool.as_slice(), copies);
                        copy_values_with_plan(
                            &mut self.value_stack,
                            caller,
                            &new_frame,
                            copy_pairs,
                        );
                    } else {
                        // copy with parameter slices
                        let parameter_slice =
                            argument_slice(callee.argument_pool.as_slice(), callee.parameters);
                        copy_values_between_frames(
                            &mut self.value_stack,
                            caller,
                            &new_frame,
                            parameter_slice,
                            argument_values,
                        );
                    }

                    // push callee frame
                    self.call_stack.push(new_frame);
                    self.statistics.calls_made += 1;
                    self.statistics.max_stack_depth =
                        self.statistics.max_stack_depth.max(self.call_stack.len());
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
                        let copy_pairs = copy_pairs(current_func.copy_pool.as_slice(), copies);
                        collect_argument_values_from_copies(&self.value_stack, caller, copy_pairs)
                    } else {
                        let argument_values =
                            argument_slice(current_func.argument_pool.as_slice(), arguments);
                        collect_argument_values(&self.value_stack, caller, argument_values)
                    };

                    // check for external or imported function
                    let func: &mir::Function = self.tree.get(function_id);
                    if func.is_import() {
                        // resolve external handler
                        let name = self.strings.get(func.name).to_string();
                        let handler = self.externals.get(&name).ok_or_else(|| {
                            self.make_error(Error::ExternalFunctionNotFound { name })
                        })?;

                        // execute external handler
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
                            return Ok(result);
                        }

                        // store return value in caller's frame
                        let caller = self
                            .call_stack
                            .last_mut()
                            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                        if let Some(dest) = caller.return_destination.take() {
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
                    self.value_stack.resize(value_end, Value::VOID);
                    self.local_stack.resize(local_end, Value::VOID);

                    // clear reused stack slots
                    self.value_stack[value_base..value_end].fill(Value::VOID);
                    self.local_stack[local_base..local_end].fill(Value::VOID);

                    // update frame metadata
                    let entry_block = &callee.blocks[callee.entry as usize];
                    frame.function = function_id;
                    frame.threaded = NonNull::from(callee.as_ref());
                    frame.block_ptr = NonNull::from(entry_block);
                    frame.entry_block = entry_block.mir_block;
                    frame.current_block = entry_block.mir_block;
                    frame.block_index = callee.entry as usize;
                    frame.resume_pc = 0;
                    frame.value_count = callee.value_count;
                    frame.local_count = callee.local_count;

                    // bind callee parameters
                    let parameter_slice =
                        argument_slice(callee.argument_pool.as_slice(), callee.parameters);
                    bind_parameters_from_values(
                        &mut self.value_stack,
                        frame,
                        parameter_slice,
                        &argument_values,
                    );

                    // update statistics
                    self.statistics.calls_made += 1;
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
                        return Ok(value);
                    }

                    // store return value in caller's frame
                    let caller = self
                        .call_stack
                        .last_mut()
                        .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                    if let Some(dest) = caller.return_destination.take() {
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
