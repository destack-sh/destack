use crate::Word;
use destack_engine as engine;
use destack_mir as mir;

use super::frame::{
    FrameValue, load_arguments, load_moved_arguments, move_arguments_between_frames, move_values,
    store_parameters,
};
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::interpreter::{Frame, Interpreter, Outcome};
use crate::options::IsolateOptions;
use crate::program::{ArgumentRange, CallTarget, Function, MoveRange, Program};

/// Local lowered function target.
struct LocalFunction<'a> {
    /// The lowered function body.
    function: &'a Function,
}

impl Interpreter {
    /// Require one local function from one call target.
    fn require_local_function<'a>(
        program: &'a Program,
        function_id: mir::LocalNodeId<mir::Function>,
        target: CallTarget,
    ) -> RuntimeResult<LocalFunction<'a>> {
        // reject imports before touching program storage
        let function_index = match target {
            CallTarget::Local(index) => index,
            CallTarget::Import => {
                return Err(RuntimeError::new(Error::undefined_function(function_id)));
            }
        };

        // load the lowered function body
        let function = program
            .functions
            .function_by_index(function_index)
            .ok_or_else(|| RuntimeError::new(Error::undefined_function(function_id)))?;

        Ok(LocalFunction { function })
    }

    /// Return the runtime boundary error for one imported call.
    fn imported_call_error(
        &self,
        program: &Program,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> RuntimeError {
        let function = program.tree.get(function_id);
        let name = program.strings.get(function.name).to_string();

        self.runtime_error(program, Error::import_forbidden(name))
    }

    /// Push one local call frame on the stack.
    fn push_call_frame(
        &mut self,
        program: &Program,
        options: &IsolateOptions,
        current_func: &Function,
        callee: LocalFunction<'_>,
        arguments: ArgumentRange,
        env: Option<Word>,
        moves: Option<MoveRange>,
        resume_pc: usize,
        return_state: Option<engine::FrameStateId>,
    ) -> RuntimeResult<()> {
        // reject stack overflow before allocating anything
        if self.frames.len() >= options.limits.max_stack_depth {
            return Err(self.runtime_error(program, Error::stack_overflow()));
        }

        // load callee entry metadata
        let entry_block = callee.function.entry;
        let frame_layout = callee.function.frame_layout;
        let frame_layout = program
            .frame_layout_by_id(frame_layout)
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        let (stack_offset, frame_base) = self.allocate_frame(frame_layout)?;

        // record the caller edge before mutating the stacks
        let caller_frame = self
            .frames
            .last_mut()
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        caller_frame.pc = resume_pc;
        caller_frame.return_state = return_state;

        let mut new_frame = Frame::new(
            callee.function,
            entry_block,
            frame_layout,
            stack_offset,
            frame_base,
        );
        new_frame
            .store_environment(frame_layout, env)
            .map_err(|error| self.runtime_error(program, error))?;

        // bind arguments from the caller into the new frame
        let caller = self
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        if let Some(moves) = moves {
            move_values(
                caller,
                &mut new_frame,
                moves,
                current_func.move_pool.as_slice(),
            )
            .map_err(RuntimeError::new)?;
        } else {
            move_arguments_between_frames(
                program,
                caller,
                &mut new_frame,
                callee.function.argument_pool.as_slice(),
                callee.function.parameters,
                current_func.argument_pool.as_slice(),
                arguments,
            )
            .map_err(RuntimeError::new)?;
        }

        // push the new frame
        self.frames.push(new_frame);
        Ok(())
    }

    /// Reuse the current frame for one lowered tail call.
    fn reuse_tail_call_frame(
        &mut self,
        program: &Program,
        callee: LocalFunction<'_>,
        arguments: &[FrameValue],
        env: Option<Word>,
    ) -> RuntimeResult<()> {
        // load the callee entry metadata first
        let entry_block = callee.function.entry;
        let frame_layout = callee.function.frame_layout;
        let frame_layout = program
            .frame_layout_by_id(frame_layout)
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;

        // replace the top frame bytes
        let stack_offset = self
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?
            .stack_offset;
        self.truncate_stack(stack_offset);
        let (stack_offset, frame_base) = self.allocate_frame(frame_layout)?;

        // retarget the frame to the callee
        let frame = self
            .frames
            .last_mut()
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;

        frame.retarget(
            callee.function,
            entry_block,
            stack_offset,
            frame_layout.byte_len as usize,
            frame_base,
        );
        frame
            .store_environment(frame_layout, env)
            .map_err(RuntimeError::new)?;

        // bind the new arguments into the reused frame
        store_parameters(
            program,
            frame,
            callee.function.argument_pool.as_slice(),
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
        options: &IsolateOptions,
        current_func: &Function,
        function: u32,
        target: CallTarget,
        arguments: ArgumentRange,
        env: Option<Word>,
        moves: Option<MoveRange>,
        resume_pc: usize,
    ) -> RuntimeResult<()> {
        // classify the call target
        let function_id = mir::LocalNodeId::<mir::Function>::new(function);

        // complete binding calls immediately in the caller frame
        if matches!(target, CallTarget::Import) {
            return Err(self.imported_call_error(program, function_id));
        }

        // otherwise enter the local callee on a new frame
        let callee = Self::require_local_function(program, function_id, target)?;
        self.push_call_frame(
            program,
            options,
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
        options: &IsolateOptions,
        current_func: &Function,
        function: u32,
        target: CallTarget,
        arguments: ArgumentRange,
        env: Option<Word>,
        target_state: engine::FrameStateId,
    ) -> RuntimeResult<()> {
        // classify the call target
        let function_id = mir::LocalNodeId::<mir::Function>::new(function);

        // imported calls resume the continuation immediately
        if matches!(target, CallTarget::Import) {
            return Err(self.imported_call_error(program, function_id));
        }

        // otherwise push the local callee and record the pending continuation
        let callee = Self::require_local_function(program, function_id, target)?;
        let caller = self
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;

        // resume after the terminator once the callee returns
        let function = program
            .functions
            .function_by_id(caller.function())
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        let resume_pc = function
            .block_len(caller.block)
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        self.push_call_frame(
            program,
            options,
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
        current_func: &Function,
        function: u32,
        target: CallTarget,
        arguments: ArgumentRange,
        env: Option<Word>,
        moves: Option<MoveRange>,
    ) -> RuntimeResult<Option<Outcome>> {
        // collect tail call arguments before reusing or popping the frame
        let caller = self
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        let argument_values = if let Some(moves) = moves {
            load_moved_arguments(program, caller, current_func.move_pool.as_slice(), moves)?
        } else {
            load_arguments(
                program,
                self.frames.as_slice(),
                caller,
                current_func.argument_pool.as_slice(),
                arguments,
            )?
        };

        // classify the call target after arguments are collected
        let function_id = mir::LocalNodeId::<mir::Function>::new(function);

        // complete binding tail calls before returning to the caller
        if matches!(target, CallTarget::Import) {
            return Err(self.imported_call_error(program, function_id));
        }

        // otherwise reuse the current frame for the local callee
        let callee = Self::require_local_function(program, function_id, target)?;
        self.reuse_tail_call_frame(program, callee, &argument_values, env)?;

        Ok(None)
    }
}
