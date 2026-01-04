use destack_mir as mir;
use smallvec::SmallVec;

use crate::diagnostic::{DiagnosticAnchor, Error, RuntimeResult};
use crate::memory::Value;

use super::{ExecutionOutput, Frame, Interpreter};

/// Inline capacity for terminator arguments.
const TERM_ARGS_INLINE_CAP: usize = 4;

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

        // check for imported function
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

        // get entry block
        let entry_block = function.entry.ok_or_else(|| {
            self.make_error(Error::UndefinedFunction { function: func_id })
                .with_anchor(DiagnosticAnchor::Function(func_id))
        })?;

        // create initial frame
        let mut frame = Frame::new(func_id, entry_block);

        // bind parameters
        for (i, param) in function.parameters.iter().enumerate() {
            let value = arguments.get(i).copied().unwrap_or(Value::Void);
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
    #[inline]
    fn update_max_stack_depth(&mut self) {
        let depth = self.call_stack.len();
        if depth > self.statistics.max_stack_depth {
            self.statistics.max_stack_depth = depth;
        }
    }

    /// Run the interpreter until the top frame returns.
    fn run(&mut self) -> RuntimeResult<ExecutionOutput> {
        // check if step limit is enabled: use slow path if so
        if self.options.max_instructions.is_some() {
            return self.run_with_step_limit();
        }

        loop {
            let stack_depth_before = self.call_stack.len();

            // current block and instruction index
            let (current_block, start_index) = {
                let frame = self
                    .call_stack
                    .last()
                    .ok_or_else(|| self.make_error(Error::InvalidInstruction))?;
                (frame.current_block, frame.instruction_idx)
            };

            // get instruction count
            let instruction_count = self.tree.get(current_block).instructions.len();

            // fast path: execute all instructions without step limit checks
            let mut did_call = false;
            for idx in start_index..instruction_count {
                self.statistics.instructions_executed += 1;

                // fetch instruction ID (tree lookup is cheap, just an index)
                let inst_id = self.tree.get(current_block).instructions[idx];
                self.execute_instruction(inst_id)?;

                // check if a call pushed a new frame
                if self.call_stack.len() > stack_depth_before {
                    if let Some(caller_frame) = self.call_stack.get_mut(stack_depth_before - 1) {
                        caller_frame.instruction_idx = idx + 1;
                    }
                    did_call = true;
                    break;
                }
            }

            if did_call {
                continue;
            }

            // execute terminator
            self.statistics.instructions_executed += 1;
            match self.execute_terminator_inline(current_block)? {
                TerminatorResult::Continue => {
                    if let Some(frame) = self.call_stack.last_mut() {
                        frame.instruction_idx = 0;
                    }
                }
                TerminatorResult::Return(value) => {
                    if self.call_stack.len() == 1 {
                        return Ok(ExecutionOutput {
                            value,
                            statistics: self.statistics.clone(),
                            heap_cells: self.managed_heap.cell_count(),
                            raw_heap_cells: self.raw_heap.cell_count(),
                        });
                    } else {
                        self.call_stack.pop();
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

    /// Slow path with per-instruction step limit checking.
    fn run_with_step_limit(&mut self) -> RuntimeResult<ExecutionOutput> {
        let max = self.options.max_instructions.unwrap();

        loop {
            let stack_depth_before = self.call_stack.len();

            let (current_block, start_index) = {
                let frame = self
                    .call_stack
                    .last()
                    .ok_or_else(|| self.make_error(Error::InvalidInstruction))?;
                (frame.current_block, frame.instruction_idx)
            };

            let instruction_count = self.tree.get(current_block).instructions.len();

            let mut did_call = false;
            for idx in start_index..instruction_count {
                if self.statistics.instructions_executed >= max {
                    return Err(self.make_error(Error::StepLimitExceeded));
                }
                self.statistics.instructions_executed += 1;

                let inst_id = self.tree.get(current_block).instructions[idx];
                self.execute_instruction(inst_id)?;

                if self.call_stack.len() > stack_depth_before {
                    if let Some(caller_frame) = self.call_stack.get_mut(stack_depth_before - 1) {
                        caller_frame.instruction_idx = idx + 1;
                    }
                    did_call = true;
                    break;
                }
            }

            if did_call {
                continue;
            }

            // terminator
            if self.statistics.instructions_executed >= max {
                return Err(self.make_error(Error::StepLimitExceeded));
            }
            self.statistics.instructions_executed += 1;

            match self.execute_terminator_inline(current_block)? {
                TerminatorResult::Continue => {
                    if let Some(frame) = self.call_stack.last_mut() {
                        frame.instruction_idx = 0;
                    }
                }
                TerminatorResult::Return(value) => {
                    if self.call_stack.len() == 1 {
                        return Ok(ExecutionOutput {
                            value,
                            statistics: self.statistics.clone(),
                            heap_cells: self.managed_heap.cell_count(),
                            raw_heap_cells: self.raw_heap.cell_count(),
                        });
                    } else {
                        self.call_stack.pop();
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

    /// Execute a terminator without cloning.
    ///
    /// Uses a two-phase borrow pattern: extract scalar values first, do mutable
    /// work, then re-borrow to get only the arguments needed.
    fn execute_terminator_inline(
        &mut self,
        block: mir::LocalNodeId<mir::Block>,
    ) -> RuntimeResult<TerminatorResult> {
        let term = &self.tree.get(block).terminator;
        match term {
            mir::Terminator::Return { value } => {
                let value = *value;

                let return_value = if let Some(v) = value {
                    self.current_frame()?.get_value(v)?
                } else {
                    Value::Void
                };

                Ok(TerminatorResult::Return(return_value))
            }

            mir::Terminator::Jump { target, .. } => {
                let target = *target;

                // resolve arguments
                let arguments = self.resolve_terminator_args(block, |t| match t {
                    mir::Terminator::Jump { arguments, .. } => arguments,
                    _ => unreachable!(),
                })?;

                self.jump_to_block(target, &arguments)?;
                Ok(TerminatorResult::Continue)
            }

            mir::Terminator::Branch {
                condition,
                then_target,
                else_target,
                ..
            } => {
                let condition = *condition;
                let then_target = *then_target;
                let else_target = *else_target;

                self.statistics.branches += 1;

                // evaluate condition
                let is_truthy = self.current_frame()?.get_value(condition)?.is_truthy();

                // resolve only the taken branch's arguments
                let arguments = self.resolve_terminator_args(block, |t| match t {
                    mir::Terminator::Branch {
                        then_arguments,
                        else_arguments,
                        ..
                    } => {
                        if is_truthy {
                            then_arguments
                        } else {
                            else_arguments
                        }
                    }
                    _ => unreachable!(),
                })?;

                let target = if is_truthy { then_target } else { else_target };
                self.jump_to_block(target, &arguments)?;
                Ok(TerminatorResult::Continue)
            }

            mir::Terminator::Switch { value, default, .. } => {
                let switch_value = *value;
                let default_target = *default;

                self.statistics.branches += 1;

                // evaluate switch value
                let int_val = self
                    .current_frame()?
                    .get_value(switch_value)?
                    .as_int()
                    .unwrap_or(0);

                // find matching case and resolve its arguments
                let (target, arguments) = {
                    let term = &self.tree.get(block).terminator;
                    let mir::Terminator::Switch {
                        cases,
                        default_arguments,
                        ..
                    } = term
                    else {
                        unreachable!()
                    };

                    let (target, args) = cases
                        .iter()
                        .find(|c| c.value == int_val)
                        .map(|c| (c.target, &c.arguments))
                        .unwrap_or((default_target, default_arguments));

                    let resolved = {
                        let frame = self.current_frame()?;
                        args.iter()
                            .map(|v| frame.get_value(*v))
                            .collect::<RuntimeResult<Vec<_>>>()?
                    };
                    (target, resolved)
                };

                self.jump_to_block(target, &arguments)?;
                Ok(TerminatorResult::Continue)
            }

            mir::Terminator::Unreachable => Err(self.make_error(Error::Unreachable)),

            mir::Terminator::Yield { .. } => Err(self.make_error(Error::UnsupportedInstruction {
                name: "yield".to_string(),
            })),
        }
    }

    /// Resolve terminator arguments using a selector function.
    ///
    /// Re-borrows the terminator and extracts arguments via the selector, then
    /// resolves them to runtime values.
    fn resolve_terminator_args<F>(
        &mut self,
        block: mir::LocalNodeId<mir::Block>,
        selector: F,
    ) -> RuntimeResult<Vec<Value>>
    where
        F: FnOnce(&mir::Terminator) -> &[mir::Value],
    {
        let args: SmallVec<[mir::Value; TERM_ARGS_INLINE_CAP]> = {
            let term = &self.tree.get(block).terminator;
            selector(term).iter().copied().collect()
        };

        let frame = self.current_frame()?;
        args.iter()
            .map(|v| frame.get_value(*v))
            .collect::<RuntimeResult<Vec<_>>>()
    }

    /// Jump to a block with arguments.
    fn jump_to_block(
        &mut self,
        target: mir::LocalNodeId<mir::Block>,
        arguments: &[Value],
    ) -> RuntimeResult<()> {
        let block = self.tree.get(target);
        let parameters: Vec<_> = block.parameters.iter().map(|p| p.value).collect();

        let frame = match self.call_stack.last_mut() {
            Some(f) => f,
            None => return Err(self.make_error(Error::InvalidInstruction)),
        };

        // bind block parameters
        for (i, param_value) in parameters.iter().enumerate() {
            let value = arguments.get(i).copied().unwrap_or(Value::Void);
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
        let func = self.tree.get(function);

        // collect arguments from current frame
        let arguments = {
            let frame = self.current_frame()?;
            arguments
                .iter()
                .map(|v| frame.get_value(*v))
                .collect::<Result<Vec<_>, _>>()?
        };

        // execute imported function
        if func.is_import() {
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

        // store where to put the return value on the *caller*'s frame
        if let Some(caller_frame) = self.call_stack.last_mut() {
            caller_frame.return_destination = destination;
        }

        // create new frame
        let mut new_frame = Frame::new(function, entry_block);

        // bind parameters
        for (i, parameter) in func.parameters.iter().enumerate() {
            let value = arguments.get(i).copied().unwrap_or(Value::Void);
            new_frame.set_value(parameter.value, value);
        }

        // push new frame and update statistics
        self.call_stack.push(new_frame);
        self.statistics.calls_made += 1;
        self.update_max_stack_depth();

        Ok(())
    }
}
