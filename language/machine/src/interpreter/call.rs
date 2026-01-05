use destack_mir as mir;
use smallvec::SmallVec;

use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::memory::Value;

use super::threaded::{ControlFlow, ThreadedState};
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

/// Collect argument values from a frame into a smallvec.
fn collect_argument_values(
    values: &[Value],
    frame: &Frame,
    arguments: &[mir::Value],
) -> SmallVec<[Value; 8]> {
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
        let func_id = self
            .tree
            .iter_nodes::<mir::Function>()
            .find(|(_, f)| self.strings.get(f.name) == name)
            .map(|(id, _)| id)
            .ok_or_else(|| {
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
        let threaded = self
            .threaded_functions
            .get(&func_id)
            .ok_or_else(|| RuntimeError::new(Error::UndefinedFunction { function: func_id }))?
            .clone();

        // create initial frame
        let entry_block = threaded.blocks[threaded.entry].mir_block;
        let value_base = self.value_stack.len();
        let local_base = self.local_stack.len();
        self.value_stack
            .resize(value_base + threaded.value_count, Value::VOID);
        self.local_stack
            .resize(local_base + threaded.local_count, Value::VOID);
        let frame = Frame::new(
            func_id,
            entry_block,
            threaded.entry,
            value_base,
            threaded.value_count,
            local_base,
            threaded.local_count,
        );

        // bind function parameters to SSA values
        for (i, param) in threaded.parameters.iter().enumerate() {
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
                && self.statistics.instructions_executed >= max
            {
                return Err(self.make_error(Error::StepLimitExceeded));
            }

            // get current frame info
            let (current_func_id, block_idx, start_pc) = {
                let frame = self
                    .call_stack
                    .last_mut()
                    .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                let pc = frame.resume_pc;
                frame.resume_pc = 0;
                (frame.function, frame.block_index, pc)
            };

            // get threaded function (clone to avoid holding borrow)
            let current_func = self
                .threaded_functions
                .get(&current_func_id)
                .ok_or_else(|| {
                    RuntimeError::new(Error::UndefinedFunction {
                        function: current_func_id,
                    })
                })?
                .clone();

            // get current block
            let block = &current_func.blocks[block_idx];
            let block_len = block.instructions.len();

            // execute block starting from resume_pc
            let control = {
                let frame_index = self.call_stack.len() - 1;
                let mut state = ThreadedState::new(self, frame_index);
                (block.instructions[start_pc].handler)(&mut state, &block.instructions, start_pc)
            };

            // update statistics
            self.statistics.instructions_executed += (block_len - start_pc) as u64;

            // handle control flow
            match control {
                ControlFlow::Jump {
                    block: target,
                    arguments,
                } => {
                    // bind block parameters for target block
                    let target_block = &current_func.blocks[target];
                    let frame = self
                        .call_stack
                        .last()
                        .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                    copy_values_between_frames(
                        &mut self.value_stack,
                        frame,
                        frame,
                        &target_block.parameters,
                        &arguments,
                    );

                    // update current block
                    let frame = self
                        .call_stack
                        .last_mut()
                        .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                    frame.block_index = target;
                    frame.current_block = target_block.mir_block;
                }

                ControlFlow::Call {
                    function,
                    destination,
                    arguments,
                    resume_pc,
                } => {
                    // check for external or imported function
                    let func: &mir::Function = self.tree.get(function);
                    if func.is_import() {
                        // resolve arguments from caller
                        let caller = self
                            .call_stack
                            .last()
                            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                        let args = collect_argument_values(&self.value_stack, caller, &arguments);

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
                        if let Some(dest) = destination {
                            frame.set_value(&mut self.value_stack, dest, result);
                        }
                        frame.resume_pc = resume_pc;
                        continue;
                    }

                    // get callee's threaded function
                    let callee = self
                        .threaded_functions
                        .get(&function)
                        .ok_or_else(|| RuntimeError::new(Error::UndefinedFunction { function }))?;

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
                    let entry_block = callee.blocks[callee.entry].mir_block;
                    let new_frame = Frame::new(
                        function,
                        entry_block,
                        callee.entry,
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
                    copy_values_between_frames(
                        &mut self.value_stack,
                        caller,
                        &new_frame,
                        &callee.parameters,
                        &arguments,
                    );

                    // push callee frame
                    self.call_stack.push(new_frame);
                    self.statistics.calls_made += 1;
                    self.statistics.max_stack_depth =
                        self.statistics.max_stack_depth.max(self.call_stack.len());
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
