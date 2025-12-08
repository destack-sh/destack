//! Function call and execution logic.

use destack_mir as mir;

use crate::diagnostic::{DiagnosticAnchor, Error, RuntimeResult};
use crate::memory::Value;

use super::{ExecutionOutput, Frame, Interpreter};

/// Result of executing a terminator.
enum TerminatorResult {
    /// Continue execution in the next block.
    Continue,
    /// Return from the current function with a value.
    Return(Value),
}

impl Interpreter {
    /// Execute a function by name.
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
    pub fn run_function(
        &mut self,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutput> {
        // reset statistics for this call
        self.statistics.reset();

        let function = self.tree.get(func_id);

        // check for external function
        if function.is_external {
            let name = self.strings.get(function.name).to_string();
            let handler = self
                .externals
                .get(&name)
                .ok_or_else(|| self.make_error(Error::ExternalFunctionNotFound { name }))?;

            let value = handler(arguments).map_err(|e| self.make_error(e))?;
            return Ok(ExecutionOutput {
                value,
                statistics: self.statistics.clone(),
                heap_cells: self.heap.cell_count(),
            });
        }

        // get entry block
        let entry_block = function.entry.ok_or_else(|| {
            self.make_error(Error::UndefinedFunction { function: func_id })
                .with_anchor(DiagnosticAnchor::Function(func_id))
        })?;

        // create initial frame
        let mut frame = Frame::new(func_id, entry_block);

        // bind parameters
        for (i, param) in function.parameters.iter().enumerate() {
            let value = arguments.get(i).cloned().unwrap_or(Value::Void);
            frame.set_value(param.value, value);
        }

        // push frame onto call stack
        self.call_stack.push(frame);
        self.statistics.calls_made += 1;
        self.update_max_stack_depth();

        // execute until we get a return value
        let result = self.run();

        // pop frame
        self.call_stack.pop();

        result
    }

    /// Update max stack depth statistic.
    fn update_max_stack_depth(&mut self) {
        let depth = self.call_stack.len();
        if depth > self.statistics.max_stack_depth {
            self.statistics.max_stack_depth = depth;
        }
    }

    /// Run the interpreter until the top frame returns.
    fn run(&mut self) -> RuntimeResult<ExecutionOutput> {
        loop {
            // check step limit
            if let Some(max) = self.options.max_instructions
                && self.statistics.instructions_executed >= max
            {
                return Err(self.make_error(Error::StepLimitExceeded));
            }

            let stack_depth_before = self.call_stack.len();

            // get current block info
            let (instruction_ids, terminator, start_index) = {
                let frame = self
                    .call_stack
                    .last()
                    .ok_or_else(|| self.make_error(Error::InvalidInstruction))?;
                let block = self.tree.get(frame.current_block);
                (
                    block.instructions.clone(),
                    block.terminator.clone(),
                    frame.instruction_index,
                )
            };

            // execute instructions starting from where we left off
            let mut did_call = false;
            for (idx, inst_id) in instruction_ids.iter().enumerate().skip(start_index) {
                self.statistics.instructions_executed += 1;

                // check step limit
                if let Some(max) = self.options.max_instructions
                    && self.statistics.instructions_executed >= max
                {
                    return Err(self.make_error(Error::StepLimitExceeded));
                }

                let instruction = self.tree.get(*inst_id).clone();
                self.execute_instruction(*inst_id, &instruction)?;

                // check if a call pushed a new frame
                if self.call_stack.len() > stack_depth_before {
                    // save where to resume when we return
                    if let Some(caller_frame) = self.call_stack.get_mut(stack_depth_before - 1) {
                        caller_frame.instruction_index = idx + 1;
                    }
                    did_call = true;
                    break;
                }
            }

            // if a call happened, restart the loop to execute the new frame
            if did_call {
                continue;
            }

            // execute terminator
            self.statistics.instructions_executed += 1;
            match self.execute_terminator(&terminator)? {
                TerminatorResult::Continue => {
                    // continue to next block (reset instruction index)
                    if let Some(frame) = self.call_stack.last_mut() {
                        frame.instruction_index = 0;
                    }
                }
                TerminatorResult::Return(value) => {
                    // check if this is the last frame
                    if self.call_stack.len() == 1 {
                        return Ok(ExecutionOutput {
                            value,
                            statistics: self.statistics.clone(),
                            heap_cells: self.heap.cell_count(),
                        });
                    } else {
                        // pop the returning frame and push value to caller
                        self.call_stack.pop();
                        // the caller should handle storing the return value
                        if let Some(caller_frame) = self.call_stack.last_mut()
                            && let Some(dest) = caller_frame.return_destination.take()
                        {
                            caller_frame.set_value(dest, value);
                        }
                    }
                }
            }
        }
    }

    /// Execute a terminator, returning what to do next.
    fn execute_terminator(
        &mut self,
        terminator: &mir::Terminator,
    ) -> RuntimeResult<TerminatorResult> {
        match terminator {
            mir::Terminator::Return { value } => {
                let return_value = if let Some(v) = value {
                    let frame = self.current_frame()?;
                    frame.get_value(*v)?
                } else {
                    Value::Void
                };
                Ok(TerminatorResult::Return(return_value))
            }

            mir::Terminator::Jump { target, arguments } => {
                let args = {
                    let frame = self.current_frame()?;
                    arguments
                        .iter()
                        .map(|v| frame.get_value(*v))
                        .collect::<Result<Vec<_>, _>>()?
                };
                self.jump_to_block(*target, &args)?;
                Ok(TerminatorResult::Continue)
            }

            mir::Terminator::Branch {
                condition,
                then_target,
                then_arguments,
                else_target,
                else_arguments,
            } => {
                let (is_truthy, then_args, else_args) = {
                    let frame = self.current_frame()?;
                    let cond = frame.get_value(*condition)?;
                    let then_args: Vec<Value> = then_arguments
                        .iter()
                        .map(|v| frame.get_value(*v))
                        .collect::<Result<_, _>>()?;
                    let else_args: Vec<Value> = else_arguments
                        .iter()
                        .map(|v| frame.get_value(*v))
                        .collect::<Result<_, _>>()?;
                    (cond.is_truthy(), then_args, else_args)
                };

                if is_truthy {
                    self.jump_to_block(*then_target, &then_args)?;
                } else {
                    self.jump_to_block(*else_target, &else_args)?;
                }
                Ok(TerminatorResult::Continue)
            }

            mir::Terminator::Switch {
                value,
                default,
                default_arguments,
                cases,
            } => {
                let (target, args) = {
                    let frame = self.current_frame()?;
                    let val = frame.get_value(*value)?;
                    let int_val = val.as_int().unwrap_or(0);

                    cases
                        .iter()
                        .find(|c| c.value == int_val)
                        .map(|c| {
                            let args: RuntimeResult<Vec<Value>> =
                                c.arguments.iter().map(|v| frame.get_value(*v)).collect();
                            (c.target, args)
                        })
                        .unwrap_or_else(|| {
                            let args: RuntimeResult<Vec<Value>> = default_arguments
                                .iter()
                                .map(|v| frame.get_value(*v))
                                .collect();
                            (*default, args)
                        })
                };

                let args = args?;
                self.jump_to_block(target, &args)?;
                Ok(TerminatorResult::Continue)
            }

            mir::Terminator::Unreachable => Err(self.make_error(Error::Unreachable)),
        }
    }

    /// Jump to a block with arguments.
    fn jump_to_block(
        &mut self,
        target: mir::LocalNodeId<mir::Block>,
        args: &[Value],
    ) -> RuntimeResult<()> {
        let block = self.tree.get(target);
        let parameters: Vec<_> = block.parameters.iter().map(|p| p.value).collect();

        let frame = match self.call_stack.last_mut() {
            Some(f) => f,
            None => return Err(self.make_error(Error::InvalidInstruction)),
        };

        // bind block parameters
        for (i, param_value) in parameters.iter().enumerate() {
            let value = args.get(i).cloned().unwrap_or(Value::Void);
            frame.set_value(*param_value, value);
        }

        frame.current_block = target;
        Ok(())
    }

    /// Execute a function call instruction.
    pub(super) fn execute_call(
        &mut self,
        destination: Option<mir::Value>,
        function: mir::LocalNodeId<mir::Function>,
        arguments: &[mir::Value],
    ) -> RuntimeResult<()> {
        // collect arguments from current frame
        let arguments = {
            let frame = self.current_frame()?;
            arguments
                .iter()
                .map(|v| frame.get_value(*v))
                .collect::<Result<Vec<_>, _>>()?
        };

        let func = self.tree.get(function);

        // execute external function
        if func.is_external {
            let name = self.strings.get(func.name).to_string();
            let external_handler = self.externals.get(&name).ok_or_else(|| {
                self.make_error(Error::ExternalFunctionNotFound { name: name.clone() })
            })?;
            let result = external_handler(&arguments).map_err(|e| self.make_error(e))?;

            if let Some(destination) = destination {
                let frame = self.current_frame_mut()?;
                frame.set_value(destination, result);
            }
            return Ok(());
        }

        // check stack depth
        if self.call_stack.len() >= self.options.max_stack_depth {
            return Err(self.make_error(Error::StackOverflow));
        }

        // get entry block
        let entry_block = func
            .entry
            .ok_or_else(|| self.make_error(Error::UndefinedFunction { function }))?;

        // store where to put the return value on the CALLER's frame
        if let Some(caller_frame) = self.call_stack.last_mut() {
            caller_frame.return_destination = destination;
        }

        // create new frame
        let mut new_frame = Frame::new(function, entry_block);

        // bind parameters
        for (i, parameter) in func.parameters.iter().enumerate() {
            let value = arguments.get(i).cloned().unwrap_or(Value::Void);
            new_frame.set_value(parameter.value, value);
        }

        // push new frame and update statistics
        self.call_stack.push(new_frame);
        self.statistics.calls_made += 1;
        self.update_max_stack_depth();

        Ok(())
    }
}
