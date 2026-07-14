use destack_program::vm::{ArgumentRange, CallTarget, Cell, FunctionCode, MoveRange};
use destack_program::{FrameStateId, FunctionId, Program};

use super::frame::{
    FrameValue, load_arguments, load_moved_arguments, move_arguments_between_frames, move_values,
    store_parameters,
};
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::machine::{Activation, Frame, Invocation, Outcome};
use crate::options::LimitOptions;

/// Source used to initialize one callee's parameters.
enum ParameterSource<'a> {
    /// Values already stored in the caller frame.
    Frame {
        /// The caller function tables.
        function: &'a FunctionCode<'a>,
        /// The caller argument slots.
        arguments: ArgumentRange,
        /// Optional direct parameter moves.
        moves: Option<MoveRange>,
    },
    /// Values produced directly by the execution engine.
    Values(&'a [FrameValue]),
}

impl Activation<'_> {
    /// Require one local function from one call target.
    fn require_local_function<'a>(
        program: &'a Program,
        function_id: FunctionId,
        target: CallTarget,
    ) -> RuntimeResult<FunctionCode<'a>> {
        // reject binding targets before touching program storage
        let Some(function_index) = target.local_index() else {
            return Err(RuntimeError::new(Error::undefined_function(function_id)));
        };

        // load the lowered function body
        let function = program
            .vm_function_by_index(function_index)
            .ok_or_else(|| RuntimeError::new(Error::undefined_function(function_id)))?;

        Ok(function)
    }

    /// Return the runtime boundary error for one binding call.
    fn binding_call_error(&self, program: &Program, function_id: FunctionId) -> RuntimeError {
        let Some(function) = program.function(function_id) else {
            return self.machine.runtime_error(Error::invalid_program(format!(
                "missing function tables for {function_id:?}"
            )));
        };

        let Some(name) = program.string(function.name) else {
            return self.machine.runtime_error(Error::invalid_program(format!(
                "missing function name string {:?}",
                function.name
            )));
        };
        let name = name.to_owned();

        self.machine.runtime_error(Error::import_forbidden(name))
    }

    /// Push one local call frame on the stack.
    fn push_call_frame(
        &mut self,
        program: &Program,
        limits: LimitOptions,
        callee: FunctionCode<'_>,
        parameters: ParameterSource<'_>,
        env: Option<Cell>,
        resume_pc: usize,
        invocation: Option<Invocation>,
    ) -> RuntimeResult<()> {
        // reject stack overflow before allocating anything
        if self.machine.frames.len() >= limits.max_stack_depth {
            return Err(self.machine.runtime_error(Error::stack_overflow()));
        }

        // load callee entry tables
        let entry_block = callee.function.entry;
        let frame_layout = callee.function.frame_layout;
        let frame_layout = program
            .frame_layout_by_id(frame_layout)
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        let (stack_offset, frame_base) = self.machine.allocate_frame(frame_layout)?;

        // record the caller edge before mutating the stacks
        let caller_frame = self
            .machine
            .frames
            .last_mut()
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        caller_frame.pc = resume_pc;
        caller_frame.invocation = invocation;

        let mut new_frame =
            Frame::new(&callee, entry_block, frame_layout, stack_offset, frame_base);
        new_frame
            .store_environment(program, frame_layout, env)
            .map_err(|error| self.machine.runtime_error(error))?;

        // bind parameters from their concrete source
        match parameters {
            ParameterSource::Frame {
                function,
                arguments,
                moves,
            } => {
                let caller = self
                    .machine
                    .frames
                    .last()
                    .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;

                if let Some(moves) = moves {
                    move_values(caller, &mut new_frame, moves, function.move_pool)
                        .map_err(RuntimeError::new)?;
                } else {
                    move_arguments_between_frames(
                        caller,
                        &mut new_frame,
                        callee.argument_pool,
                        callee.function.parameters,
                        function.argument_pool,
                        arguments,
                    )
                    .map_err(RuntimeError::new)?;
                }
            }
            ParameterSource::Values(values) => store_parameters(
                &mut new_frame,
                callee.argument_pool,
                callee.function.parameters,
                values,
            )
            .map_err(RuntimeError::new)?,
        }

        // push the new frame
        self.machine.frames.push(new_frame);
        Ok(())
    }

    /// Reuse the current frame for one lowered tail call.
    fn reuse_tail_call_frame(
        &mut self,
        program: &Program,
        callee: FunctionCode<'_>,
        arguments: &[FrameValue],
        env: Option<Cell>,
    ) -> RuntimeResult<()> {
        // load the callee entry tables first
        let entry_block = callee.function.entry;
        let frame_layout = callee.function.frame_layout;
        let frame_layout = program
            .frame_layout_by_id(frame_layout)
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;

        // replace the top frame bytes
        let stack_offset = self
            .machine
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?
            .stack_offset;
        self.machine.truncate_stack(stack_offset);
        let (stack_offset, frame_base) = self.machine.allocate_frame(frame_layout)?;

        // retarget the frame to the callee
        let frame = self
            .machine
            .frames
            .last_mut()
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;

        frame.retarget(
            &callee,
            entry_block,
            stack_offset,
            frame_layout.byte_len() as usize,
            frame_base,
        );
        frame
            .store_environment(program, frame_layout, env)
            .map_err(RuntimeError::new)?;

        // bind the new arguments into the reused frame
        store_parameters(
            frame,
            callee.argument_pool,
            callee.function.parameters,
            arguments,
        )
        .map_err(RuntimeError::new)?;

        Ok(())
    }

    /// Complete one call from the current frame.
    pub(crate) fn complete_call(
        &mut self,
        program: &Program,
        limits: LimitOptions,
        current_func: &FunctionCode<'_>,
        function: FunctionId,
        target: CallTarget,
        arguments: ArgumentRange,
        env: Option<Cell>,
        moves: Option<MoveRange>,
        resume_pc: usize,
    ) -> RuntimeResult<()> {
        // reject binding calls until runtime dispatch is wired
        if target.is_binding() {
            return Err(self.binding_call_error(program, function));
        }

        // otherwise enter the local callee on a new frame
        let callee = Self::require_local_function(program, function, target)?;
        self.push_call_frame(
            program,
            limits,
            callee,
            ParameterSource::Frame {
                function: current_func,
                arguments,
                moves,
            },
            env,
            resume_pc,
            None,
        )
    }

    /// Complete one value drop from the current frame.
    pub(crate) fn complete_drop(
        &mut self,
        program: &Program,
        limits: LimitOptions,
        function: FunctionId,
        target: CallTarget,
        address: Cell,
        resume_pc: usize,
    ) -> RuntimeResult<()> {
        // destructors are always local program functions
        let callee = Self::require_local_function(program, function, target)?;
        let parameters = callee.function.parameters.slice(callee.argument_pool);
        let [address_parameter] = parameters else {
            return Err(RuntimeError::new(Error::invalid_instruction()));
        };
        let arguments = [FrameValue::cell(address_parameter.ty, address)];

        self.push_call_frame(
            program,
            limits,
            callee,
            ParameterSource::Values(&arguments),
            None,
            resume_pc,
            None,
        )
    }

    /// Complete one invocation from the current frame.
    pub(crate) fn complete_invoke(
        &mut self,
        program: &Program,
        limits: LimitOptions,
        current_func: &FunctionCode<'_>,
        function: FunctionId,
        target: CallTarget,
        arguments: ArgumentRange,
        env: Option<Cell>,
        normal_state: FrameStateId,
        unwind_state: FrameStateId,
    ) -> RuntimeResult<()> {
        // reject binding calls until runtime dispatch is wired
        if target.is_binding() {
            return Err(self.binding_call_error(program, function));
        }

        // otherwise push the local callee and record the pending continuation
        let callee = Self::require_local_function(program, function, target)?;
        let caller = self
            .machine
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;

        // capture the caller at the invoke operation
        let point = caller.point(program)?;
        let frame_state = program
            .frame_state_at(point)
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        let invocation = Invocation {
            frame_state,
            normal_state,
            unwind_state,
        };

        // resume after the terminator once the callee returns
        let function = program
            .vm_function_by_id(caller.function())
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        let resume_pc = function
            .block_len(caller.block)
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        self.push_call_frame(
            program,
            limits,
            callee,
            ParameterSource::Frame {
                function: current_func,
                arguments,
                moves: None,
            },
            env,
            resume_pc,
            Some(invocation),
        )
    }

    /// Complete one tail call in the current frame.
    pub(crate) fn complete_tail_call(
        &mut self,
        program: &Program,
        current_func: &FunctionCode<'_>,
        function: FunctionId,
        target: CallTarget,
        arguments: ArgumentRange,
        env: Option<Cell>,
        moves: Option<MoveRange>,
    ) -> RuntimeResult<Option<Outcome>> {
        // collect tail call arguments before reusing or popping the frame
        let caller = self
            .machine
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        let argument_values = if let Some(moves) = moves {
            load_moved_arguments(caller, current_func.move_pool, moves)?
        } else {
            load_arguments(caller, current_func.argument_pool, arguments)?
        };

        // reject binding tail calls until runtime dispatch is wired
        if target.is_binding() {
            return Err(self.binding_call_error(program, function));
        }

        // otherwise reuse the current frame for the local callee
        let callee = Self::require_local_function(program, function, target)?;
        self.reuse_tail_call_frame(program, callee, &argument_values, env)?;

        Ok(None)
    }
}
