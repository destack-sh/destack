use crate::Cell;

use super::frame::{
    FrameValue, load_arguments, load_moved_arguments, move_arguments_between_frames, move_values,
    store_parameters,
};
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::machine::{Activation, Frame, Outcome};
use crate::options::LimitOptions;
use destack_program::vm::{ArgumentRange, CallTarget, FunctionCode, MoveRange};
use destack_program::{FrameStateId, FunctionId, Program};

/// Local lowered function target.
struct LocalFunction<'a> {
    /// The lowered function body.
    function: FunctionCode<'a>,
}

impl Activation<'_> {
    /// Require one local function from one call target.
    fn require_local_function<'a>(
        program: &'a Program,
        function_id: FunctionId,
        target: CallTarget,
    ) -> RuntimeResult<LocalFunction<'a>> {
        // reject imports before touching program storage
        let Some(function_index) = target.local_index() else {
            return Err(RuntimeError::new(Error::undefined_function(function_id)));
        };

        // load the lowered function body
        let function = program
            .vm_function_by_index(function_index)
            .ok_or_else(|| RuntimeError::new(Error::undefined_function(function_id)))?;

        Ok(LocalFunction { function })
    }

    /// Return the runtime boundary error for one imported call.
    fn imported_call_error(&self, program: &Program, function_id: FunctionId) -> RuntimeError {
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
        current_func: &FunctionCode<'_>,
        callee: LocalFunction<'_>,
        arguments: ArgumentRange,
        env: Option<Cell>,
        moves: Option<MoveRange>,
        resume_pc: usize,
        return_state: Option<FrameStateId>,
    ) -> RuntimeResult<()> {
        // reject stack overflow before allocating anything
        if self.machine.frames.len() >= limits.max_stack_depth {
            return Err(self.machine.runtime_error(Error::stack_overflow()));
        }

        // load callee entry tables
        let entry_block = callee.function.function.entry;
        let frame_layout = callee.function.function.frame_layout;
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
        caller_frame.return_state = return_state;

        let mut new_frame = Frame::new(
            &callee.function,
            entry_block,
            frame_layout,
            stack_offset,
            frame_base,
        );
        new_frame
            .store_environment(program, frame_layout, env)
            .map_err(|error| self.machine.runtime_error(error))?;

        // bind arguments from the caller into the new frame
        let caller = self
            .machine
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        if let Some(moves) = moves {
            move_values(caller, &mut new_frame, moves, current_func.move_pool)
                .map_err(RuntimeError::new)?;
        } else {
            move_arguments_between_frames(
                caller,
                &mut new_frame,
                callee.function.argument_pool,
                callee.function.function.parameters,
                current_func.argument_pool,
                arguments,
            )
            .map_err(RuntimeError::new)?;
        }

        // push the new frame
        self.machine.frames.push(new_frame);
        Ok(())
    }

    /// Reuse the current frame for one lowered tail call.
    fn reuse_tail_call_frame(
        &mut self,
        program: &Program,
        callee: LocalFunction<'_>,
        arguments: &[FrameValue],
        env: Option<Cell>,
    ) -> RuntimeResult<()> {
        // load the callee entry tables first
        let entry_block = callee.function.function.entry;
        let frame_layout = callee.function.function.frame_layout;
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
            &callee.function,
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
            callee.function.argument_pool,
            callee.function.function.parameters,
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
        // complete binding calls immediately in the caller frame
        if target.is_import() {
            return Err(self.imported_call_error(program, function));
        }

        // otherwise enter the local callee on a new frame
        let callee = Self::require_local_function(program, function, target)?;
        self.push_call_frame(
            program,
            limits,
            current_func,
            callee,
            arguments,
            env,
            moves,
            resume_pc,
            None,
        )
    }

    /// Complete one call terminator from the current frame.
    pub(crate) fn complete_call_branch(
        &mut self,
        program: &Program,
        limits: LimitOptions,
        current_func: &FunctionCode<'_>,
        function: FunctionId,
        target: CallTarget,
        arguments: ArgumentRange,
        env: Option<Cell>,
        target_state: FrameStateId,
    ) -> RuntimeResult<()> {
        // imported calls resume the continuation immediately
        if target.is_import() {
            return Err(self.imported_call_error(program, function));
        }

        // otherwise push the local callee and record the pending continuation
        let callee = Self::require_local_function(program, function, target)?;
        let caller = self
            .machine
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;

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
            current_func,
            callee,
            arguments,
            env,
            None,
            resume_pc,
            Some(target_state),
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

        // complete binding tail calls before returning to the caller
        if target.is_import() {
            return Err(self.imported_call_error(program, function));
        }

        // otherwise reuse the current frame for the local callee
        let callee = Self::require_local_function(program, function, target)?;
        self.reuse_tail_call_frame(program, callee, &argument_values, env)?;

        Ok(None)
    }
}
