use destack_program::vm::{ArgumentRange, CallTarget, Cell, FunctionCode, MoveRange};
use destack_program::{FrameStateId, FunctionId, Program, StopReason, TypeId};

use super::frame::move_values_within_frame;
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::machine::{Activation, Continuation, Outcome};
use crate::options::LimitOptions;

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
        function: FunctionId,
        /// Lowered call target.
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
    /// Drop one value through its destructor.
    Drop {
        /// Function to call.
        function: FunctionId,
        /// Lowered call target.
        target: CallTarget,
        /// Address of the value being dropped.
        address: Cell,
        /// PC to resume at after drop returns.
        resume_pc: usize,
    },
    /// Invoke another function with normal and unwind continuations.
    Invoke {
        /// Function to call.
        function: FunctionId,
        /// Lowered call target.
        target: CallTarget,
        /// Arguments to pass.
        arguments: ArgumentRange,
        /// Optional function environment to pass.
        env: Option<Cell>,
        /// The normal continuation frame state.
        normal_state: FrameStateId,
        /// The unwind continuation frame state.
        unwind_state: FrameStateId,
    },
    /// Tail call another function.
    TailCall {
        /// Function to call.
        function: FunctionId,
        /// Lowered call target.
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
        source_type: TypeId,
        /// The frame state captured in the continuation.
        frame_state: FrameStateId,
    },
    /// Stop for host inspection.
    Stop {
        /// The stop reason.
        reason: StopReason,
        /// The frame state captured in the continuation.
        frame_state: FrameStateId,
    },
    /// Return from current function.
    Return(Cell),
    /// Begin unwinding one language panic.
    Panic(Error),
    /// Continue the active language panic unwind.
    ResumeUnwind,
    /// Runtime error.
    Error(Error),
}

impl Transfer {
    /// Create one invocation transfer.
    pub(crate) fn invoke(
        function: FunctionId,
        target: CallTarget,
        arguments: ArgumentRange,
        env: Option<Cell>,
        normal_state: FrameStateId,
        unwind_state: FrameStateId,
    ) -> Self {
        Self::Invoke {
            function,
            target,
            arguments,
            env,
            normal_state,
            unwind_state,
        }
    }
}

impl Activation<'_> {
    /// Capture execution machine into a continuation.
    pub(crate) fn capture_continuation(
        &mut self,
        program: &Program,
        resume_frame_index: usize,
        frame_state: FrameStateId,
    ) -> RuntimeResult<Continuation> {
        self.machine
            .capture_continuation(program, resume_frame_index, frame_state)
    }

    /// Complete one jump transfer within the current frame.
    fn complete_jump(
        &mut self,
        current_func: &FunctionCode<'_>,
        target: u32,
        moves: MoveRange,
    ) -> RuntimeResult<()> {
        // move block parameters before updating the frame position
        let frame = self
            .machine
            .frames
            .last_mut()
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        move_values_within_frame(frame, moves, current_func.move_pool)
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
        program: &Program,
        value: Cell,
        source_type: TypeId,
        frame_state: FrameStateId,
    ) -> RuntimeResult<Outcome> {
        // capture the logical yield position first
        let resume_frame_index = self.machine.frames.len() - 1;

        // capture the yielded result before moving the stack into the continuation
        let value = super::frame::frame_value_from_cell(
            program,
            self.machine.frames.as_slice(),
            self.machine.stack.memory_base_address(),
            source_type,
            value,
        )
        .and_then(|value| {
            super::frame::materialize_value(
                program,
                self.heap,
                self.shared,
                self.shared_cache,
                self.shared_mark_worker,
                value,
            )
        })
        .map_err(RuntimeError::new)?;

        // capture the continuation after packaging the yielded result
        let continuation = self.capture_continuation(program, resume_frame_index, frame_state)?;
        self.record_profile_continuation_capture(frame_state)
            .map_err(RuntimeError::new)?;

        Ok(Outcome::Yielded {
            continuation,
            value,
        })
    }

    /// Complete one stop transfer and return the stopped outcome.
    fn complete_stop(
        &mut self,
        program: &Program,
        reason: StopReason,
        frame_state: FrameStateId,
    ) -> RuntimeResult<Outcome> {
        let resume_frame_index = self.machine.frames.len() - 1;
        let continuation = self.capture_continuation(program, resume_frame_index, frame_state)?;

        Ok(Outcome::Stopped {
            continuation,
            reason,
        })
    }

    /// Complete one control transfer produced by instruction execution.
    pub(crate) fn complete_transfer(
        &mut self,
        program: &Program,
        limits: LimitOptions,
        current_func: &FunctionCode<'_>,
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
            Transfer::Drop {
                function,
                target,
                address,
                resume_pc,
            } => {
                self.complete_drop(program, limits, function, target, address, resume_pc)?;
                Ok(None)
            }
            Transfer::Invoke {
                function,
                target,
                arguments,
                env,
                normal_state,
                unwind_state,
            } => {
                self.complete_invoke(
                    program,
                    limits,
                    current_func,
                    function,
                    target,
                    arguments,
                    env,
                    normal_state,
                    unwind_state,
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
                .complete_yield(program, value, source_type, frame_state)
                .map(Some),
            Transfer::Stop {
                reason,
                frame_state,
            } => self.complete_stop(program, reason, frame_state).map(Some),
            Transfer::Return(value) => self.complete_return(program, value),
            Transfer::Panic(error) => self.complete_panic(program, error),
            Transfer::ResumeUnwind => self.complete_unwind(program),
            Transfer::Error(error) => Err(self.machine.runtime_error(error)),
        }
    }
}
