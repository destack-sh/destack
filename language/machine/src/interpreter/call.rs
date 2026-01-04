use destack_mir as mir;

use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::memory::Value;

use super::threaded::{ControlFlow, ThreadedState};
use super::{ExecutionOutput, Interpreter};

/// Execution frame for the threaded interpreter.
///
/// Tracks the state of a single function invocation including SSA values,
/// local variables, and return destination.
struct ExecutionFrame {
    /// Function being executed.
    function: mir::LocalNodeId<mir::Function>,
    /// Current block index within the threaded function.
    block: usize,
    /// PC to resume at within current block (for continuing after calls).
    resume_pc: usize,
    /// SSA values in this frame, indexed by mir::Value id.
    values: Vec<Value>,
    /// Local variables (stack slots).
    locals: Vec<Value>,
    /// Where to store the return value when callee returns.
    return_dest: Option<mir::Value>,
}

impl ExecutionFrame {
    /// Create a new frame for executing a function.
    fn new(func_id: mir::LocalNodeId<mir::Function>, entry_block: usize) -> Self {
        Self {
            function: func_id,
            block: entry_block,
            resume_pc: 0,
            values: Vec::with_capacity(64),
            locals: Vec::with_capacity(16),
            return_dest: None,
        }
    }
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

        let function = self.tree.get(func_id);

        // handle imported/external functions
        if function.is_import() {
            let name = self.strings.get(function.name).to_string();
            let handler = self
                .externals
                .get(&name)
                .ok_or_else(|| self.make_error(Error::ExternalFunctionNotFound { name }))?;

            let value = handler(arguments).map_err(|e| self.make_error(e))?;

            return Ok(ExecutionOutput {
                value,
                statistics: self.statistics.clone(),
                heap_cells: self.managed_heap.cell_count(),
                raw_heap_cells: self.raw_heap.cell_count(),
            });
        }

        // execute using threaded dispatch
        let value = self.execute_threaded(func_id, arguments)?;

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
        // get pre-threaded function
        let threaded = self
            .threaded_functions
            .get(&func_id)
            .ok_or_else(|| RuntimeError::new(Error::UndefinedFunction { function: func_id }))?
            .clone();

        // create initial frame
        let mut frame = ExecutionFrame::new(func_id, threaded.entry);

        // bind function parameters to SSA values
        for (i, param) in threaded.parameters.iter().enumerate() {
            let value = arguments.get(i).copied().unwrap_or(Value::VOID);
            let idx = param.0 as usize;
            if idx >= frame.values.len() {
                frame.values.resize(idx + 1, Value::VOID);
            }
            frame.values[idx] = value;
        }

        // call stack for nested function calls
        let mut call_stack: Vec<ExecutionFrame> = vec![frame];

        // main execution loop (trampoline pattern)
        loop {
            // check step limit
            if let Some(max) = self.options.max_instructions {
                if self.statistics.instructions_executed >= max {
                    return Err(RuntimeError::new(Error::StepLimitExceeded));
                }
            }

            // get current frame info
            let (current_func_id, block_idx, start_pc) = {
                let frame = call_stack.last_mut().unwrap();
                let pc = frame.resume_pc;
                frame.resume_pc = 0;
                (frame.function, frame.block, pc)
            };

            // get threaded function (clone to avoid holding borrow)
            let current_func = self
                .threaded_functions
                .get(&current_func_id)
                .ok_or_else(|| {
                    RuntimeError::new(Error::UndefinedFunction { function: current_func_id })
                })?
                .clone();

            // get current block
            let block = &current_func.blocks[block_idx];
            let block_len = block.instructions.len();

            // create execution state for handlers
            let frame = call_stack.last_mut().unwrap();
            let mut state = ThreadedState {
                values: &mut frame.values,
                locals: &mut frame.locals,
                interp: self,
            };

            // execute block starting from resume_pc
            let control =
                (block.instructions[start_pc].handler)(&mut state, &block.instructions, start_pc);

            // update statistics
            self.statistics.instructions_executed += (block_len - start_pc) as u64;

            // handle control flow
            match control {
                ControlFlow::Jump { block: target, arguments } => {
                    // bind block parameters for target block
                    let target_block = &current_func.blocks[target];
                    for (i, param) in target_block.parameters.iter().enumerate() {
                        let value = arguments.get(i).copied().unwrap_or(Value::VOID);
                        let idx = param.0 as usize;
                        let frame = call_stack.last_mut().unwrap();
                        if idx >= frame.values.len() {
                            frame.values.resize(idx + 1, Value::VOID);
                        }
                        frame.values[idx] = value;
                    }

                    // update current block
                    let frame = call_stack.last_mut().unwrap();
                    frame.block = target;
                }

                ControlFlow::Call { function, destination, arguments, resume_pc } => {
                    // check for external/imported function
                    let func = self.tree.get(function);
                    if func.is_import() {
                        let name = self.strings.get(func.name).to_string();
                        let handler = self.externals.get(&name).ok_or_else(|| {
                            RuntimeError::new(Error::ExternalFunctionNotFound {
                                name: name.clone(),
                            })
                        })?;

                        let result = handler(&arguments).map_err(RuntimeError::new)?;

                        // store result and continue from resume_pc
                        let frame = call_stack.last_mut().unwrap();
                        if let Some(dest) = destination {
                            let idx = dest.0 as usize;
                            if idx >= frame.values.len() {
                                frame.values.resize(idx + 1, Value::VOID);
                            }
                            frame.values[idx] = result;
                        }
                        frame.resume_pc = resume_pc;
                        continue;
                    }

                    // get callee's threaded function
                    let callee = self
                        .threaded_functions
                        .get(&function)
                        .ok_or_else(|| RuntimeError::new(Error::UndefinedFunction { function }))?;

                    // store return destination and resume_pc in caller frame
                    let frame = call_stack.last_mut().unwrap();
                    frame.return_dest = destination;
                    frame.resume_pc = resume_pc;

                    // create new frame for callee
                    let mut new_frame = ExecutionFrame::new(function, callee.entry);

                    // bind callee's parameters
                    for (i, param) in callee.parameters.iter().enumerate() {
                        let value = arguments.get(i).copied().unwrap_or(Value::VOID);
                        let idx = param.0 as usize;
                        if idx >= new_frame.values.len() {
                            new_frame.values.resize(idx + 1, Value::VOID);
                        }
                        new_frame.values[idx] = value;
                    }

                    // check stack overflow
                    if call_stack.len() >= self.options.max_stack_depth {
                        return Err(RuntimeError::new(Error::StackOverflow));
                    }

                    call_stack.push(new_frame);
                    self.statistics.calls_made += 1;
                }

                ControlFlow::Return(value) => {
                    // pop completed frame
                    call_stack.pop();

                    // if stack is empty, execution is complete
                    if call_stack.is_empty() {
                        return Ok(value);
                    }

                    // store return value in caller's frame
                    let caller = call_stack.last_mut().unwrap();
                    if let Some(dest) = caller.return_dest.take() {
                        let idx = dest.0 as usize;
                        if idx >= caller.values.len() {
                            caller.values.resize(idx + 1, Value::VOID);
                        }
                        caller.values[idx] = value;
                    }
                }

                ControlFlow::Error(e) => {
                    return Err(RuntimeError::new(e));
                }
            }
        }
    }
}
