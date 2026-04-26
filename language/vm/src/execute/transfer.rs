use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::interpreter::{Continuation, Interpreter, Outcome};
use crate::isolate::ExternalFn;
use crate::options::IsolateOptions;
use crate::program::{Function, MoveRange, Program, Transfer};
use crate::{SharedHeap, Word};
use destack_heap::Heap;
use {destack_engine as engine, destack_mir as mir};

use super::frame::move_values;

impl Interpreter {
    /// Capture execution state into a continuation.
    pub(crate) fn capture_continuation(
        &mut self,
        isolate_id: engine::EngineId,
        resume_frame_index: usize,
        resume_point: engine::ResumePointId,
    ) -> Continuation {
        // move execution stack into the continuation
        let stack = std::mem::take(&mut self.stack);
        let frames = std::mem::take(&mut self.frames);

        Continuation {
            isolate_id,
            stack,
            frames,
            resume_frame_index,
            resume_point,
        }
    }

    /// Apply one jump transfer within the current frame.
    fn apply_jump_transfer(
        &mut self,
        program: &Program,
        current_func: &Function,
        target: u32,
        moves: MoveRange,
    ) -> RuntimeResult<()> {
        // move block parameters before updating the frame position
        let target_block = &current_func.blocks[target as usize];
        let frame = self
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let copied = frame.clone_for_fork();
        let frame = self
            .frames
            .last_mut()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        move_values(
            program,
            &copied,
            frame,
            moves,
            current_func.move_pool.as_slice(),
        )
        .map_err(RuntimeError::new)?;

        // retarget the frame to the destination block
        let frame = self
            .frames
            .last_mut()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        frame.block_ptr = std::ptr::NonNull::from(target_block);
        frame.current_block = target_block.mir_block;

        Ok(())
    }

    /// Apply one yield transfer and return the yielded outcome.
    fn apply_yield_transfer(
        &mut self,
        program: &Program,
        heap: &mut Heap,
        isolate_id: engine::EngineId,
        value: Word,
        source: mir::Value,
        resume_point: engine::ResumePointId,
    ) -> RuntimeResult<Outcome> {
        // capture the logical yield position first
        let resume_frame_index = self.frames.len() - 1;

        // capture the yielded result before moving the stack into the continuation
        let frame = self
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let yield_type =
            super::frame::frame_value_type(program, frame, source).map_err(RuntimeError::new)?;
        let value =
            super::frame::frame_value_from_word(program, self.frames.as_slice(), yield_type, value)
                .and_then(|value| super::frame::materialize_frame_value(program, heap, value))
                .map_err(RuntimeError::new)?;

        // capture the continuation after packaging the yielded result
        let continuation = self.capture_continuation(isolate_id, resume_frame_index, resume_point);

        Ok(Outcome::Yielded {
            continuation,
            value,
        })
    }

    /// Apply one control transfer produced by instruction execution.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_transfer(
        &mut self,
        isolate_id: engine::EngineId,
        program: &Program,
        options: &IsolateOptions,
        externals: &std::collections::HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        current_func: &Function,
        transfer: Transfer,
    ) -> RuntimeResult<Option<Outcome>> {
        // dispatch the transfer to the matching machine opcode
        match transfer {
            Transfer::Continue => Err(self.make_error(program, Error::InvalidInstruction)),
            Transfer::Jump { block, moves } => {
                self.apply_jump_transfer(program, current_func, block, moves)?;
                Ok(None)
            }
            Transfer::Call {
                function,
                target,
                destination,
                arguments,
                env,
                moves,
                resume_pc,
            } => {
                self.apply_call_transfer(
                    program,
                    options,
                    externals,
                    heap,
                    shared,
                    current_func,
                    function,
                    target,
                    destination,
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
                normal_resume_point,
                unwind_resume_point,
            } => {
                self.apply_call_branch_transfer(
                    program,
                    options,
                    externals,
                    heap,
                    shared,
                    current_func,
                    function,
                    target,
                    arguments,
                    env,
                    normal_resume_point,
                    unwind_resume_point,
                )?;
                Ok(None)
            }
            Transfer::TailCall {
                function,
                target,
                arguments,
                env,
                moves,
            } => self.apply_tail_call_transfer(
                program,
                options,
                externals,
                heap,
                shared,
                current_func,
                function,
                target,
                arguments,
                env,
                moves,
            ),
            Transfer::Yield {
                value,
                source,
                resume_point,
            } => self
                .apply_yield_transfer(program, heap, isolate_id, value, source, resume_point)
                .map(Some),
            Transfer::Throw(value) => self.apply_throw_transfer(program, value),
            Transfer::Return(value) => self.apply_return_transfer(program, heap, value),
            Transfer::Error(error) => Err(self.make_error(program, error)),
        }
    }
}
