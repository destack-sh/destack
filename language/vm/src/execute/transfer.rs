use std::mem;

use crate::Cell;
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::machine::{Activation, Continuation, Outcome, Stack};
use crate::options::LimitOptions;
use destack_mir as mir;
use destack_program as program;
use destack_program::Program;
use destack_program::vm::{ArgumentRange, CallTarget, Executable, Function, MoveRange};

use super::frame::move_values_within_frame;

/// Control transfer requested by one lowered instruction.
#[derive(Debug)]
pub(crate) enum Transfer {
    /// Continue at the current frame's block.
    Enter,
    /// Jump to another block.
    Jump {
        /// Target block index.
        block: u32,
        /// Move plan for block parameters.
        moves: MoveRange,
    },
    /// Call another function.
    Call {
        /// Function to call.
        function: u32,
        /// Lowered or imported call target.
        target: CallTarget,
        /// Arguments to pass.
        arguments: ArgumentRange,
        /// Optional function environment to pass.
        env: Option<Cell>,
        /// Move plan for callee parameters.
        moves: Option<MoveRange>,
        /// PC to resume at after call returns.
        resume_pc: usize,
    },
    /// Call another function and enter an explicit continuation.
    CallBranch {
        /// Function to call.
        function: u32,
        /// Lowered or imported call target.
        target: CallTarget,
        /// Arguments to pass.
        arguments: ArgumentRange,
        /// Optional function environment to pass.
        env: Option<Cell>,
        /// The continuation frame state.
        target_state: program::FrameStateId,
    },
    /// Tail call another function.
    TailCall {
        /// Function to call.
        function: u32,
        /// Lowered or imported call target.
        target: CallTarget,
        /// Arguments to pass.
        arguments: ArgumentRange,
        /// Optional function environment to pass.
        env: Option<Cell>,
        /// Move plan for callee parameters.
        moves: Option<MoveRange>,
    },
    /// Yield from the current function.
    Yield {
        /// The value yielded to the caller.
        value: Cell,
        /// The yielded value type.
        source_type: mir::LocalNodeId<mir::Type>,
        /// The frame state captured in the continuation.
        frame_state: program::FrameStateId,
    },
    /// Return from current function.
    Return(Cell),
    /// Runtime error.
    Error(Error),
}

impl Activation<'_> {
    /// Capture execution machine into a continuation.
    pub(crate) fn capture_continuation(
        &mut self,
        resume_frame_index: usize,
        frame_state: program::FrameStateId,
        limits: LimitOptions,
    ) -> RuntimeResult<Continuation> {
        // move execution stack into the continuation
        let stack = mem::replace(&mut self.machine.stack, Stack::new(limits.stack_bytes)?);
        let frames = mem::take(&mut self.machine.frames);

        Ok(Continuation {
            stack,
            frames,
            resume_frame_index,
            frame_state,
        })
    }

    /// Complete one jump transfer within the current frame.
    fn complete_jump(
        &mut self,
        current_func: &Function,
        target: u32,
        moves: MoveRange,
    ) -> RuntimeResult<()> {
        // move block parameters before updating the frame position
        let frame = self
            .machine
            .frames
            .last_mut()
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        move_values_within_frame(frame, moves, current_func.move_pool.as_slice())
            .map_err(RuntimeError::new)?;

        // retarget the frame to the destination block
        let frame = self
            .machine
            .frames
            .last_mut()
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        frame.block = target;

        Ok(())
    }

    /// Complete one yield transfer and return the yielded outcome.
    fn complete_yield(
        &mut self,
        program: &Program<Executable>,
        limits: LimitOptions,
        value: Cell,
        source_type: mir::LocalNodeId<mir::Type>,
        frame_state: program::FrameStateId,
    ) -> RuntimeResult<Outcome> {
        // capture the logical yield position first
        let resume_frame_index = self.machine.frames.len() - 1;

        // capture the yielded result before moving the stack into the continuation
        let value = super::frame::frame_value_from_cell(
            program,
            self.machine.frames.as_slice(),
            source_type,
            value,
        )
        .and_then(|value| {
            super::frame::materialize_value(
                program,
                self.heap,
                self.shared,
                self.shared_cache,
                self.shared_gc,
                value,
            )
        })
        .map_err(RuntimeError::new)?;

        // capture the continuation after packaging the yielded result
        let continuation = self.capture_continuation(resume_frame_index, frame_state, limits)?;

        Ok(Outcome::Yielded {
            continuation,
            value,
        })
    }

    /// Complete one control transfer produced by instruction execution.
    pub(crate) fn complete_transfer(
        &mut self,
        program: &Program<Executable>,
        limits: LimitOptions,
        current_func: &Function,
        transfer: Transfer,
    ) -> RuntimeResult<Option<Outcome>> {
        // complete the concrete transfer
        match transfer {
            Transfer::Enter => Ok(None),
            Transfer::Jump { block, moves } => {
                self.complete_jump(current_func, block, moves)?;
                Ok(None)
            }
            Transfer::Call {
                function,
                target,
                arguments,
                env,
                moves,
                resume_pc,
            } => {
                self.complete_call(
                    program,
                    limits,
                    current_func,
                    function,
                    target,
                    arguments,
                    env,
                    moves,
                    resume_pc,
                )?;
                Ok(None)
            }
            Transfer::CallBranch {
                function,
                target,
                arguments,
                env,
                target_state,
            } => {
                self.complete_call_branch(
                    program,
                    limits,
                    current_func,
                    function,
                    target,
                    arguments,
                    env,
                    target_state,
                )?;
                Ok(None)
            }
            Transfer::TailCall {
                function,
                target,
                arguments,
                env,
                moves,
            } => self.complete_tail_call(
                program,
                current_func,
                function,
                target,
                arguments,
                env,
                moves,
            ),
            Transfer::Yield {
                value,
                source_type,
                frame_state,
            } => self
                .complete_yield(program, limits, value, source_type, frame_state)
                .map(Some),
            Transfer::Return(value) => self.complete_return(program, value),
            Transfer::Error(error) => Err(self.machine.runtime_error(error)),
        }
    }
}
