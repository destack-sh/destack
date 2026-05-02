use std::collections::HashMap;
use std::mem;
use std::ptr::NonNull;

use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::interpreter::{Continuation, Interpreter, Outcome, Stack};
use crate::isolate::ExternalFn;
use crate::options::IsolateOptions;
use crate::program::{Function, MoveRange, Program, Transfer};
use crate::{SharedHeap, Word};
use destack_heap::{Heap, SharedAllocator};
use {destack_engine as engine, destack_mir as mir};

use super::frame::move_values_within_frame;

impl Interpreter {
    /// Capture execution state into a continuation.
    pub(crate) fn capture_continuation(
        &mut self,
        isolate_id: engine::EngineId,
        resume_frame_index: usize,
        frame_state: engine::FrameStateId,
        options: &IsolateOptions,
    ) -> RuntimeResult<Continuation> {
        // move execution stack into the continuation
        let stack = mem::replace(
            &mut self.stack,
            Stack::reserve(options.limits.max_stack_bytes)?,
        );
        let frames = mem::take(&mut self.frames);

        Ok(Continuation {
            isolate_id,
            stack,
            frames,
            resume_frame_index,
            frame_state,
        })
    }

    /// Complete one jump transfer within the current frame.
    fn complete_jump(
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
            .last_mut()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        move_values_within_frame(program, frame, moves, current_func.move_pool.as_slice())
            .map_err(RuntimeError::new)?;

        // retarget the frame to the destination block
        let frame = self
            .frames
            .last_mut()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        frame.block_ptr = NonNull::from(target_block);

        Ok(())
    }

    /// Complete one yield transfer and return the yielded outcome.
    fn complete_yield(
        &mut self,
        program: &Program,
        options: &IsolateOptions,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_allocator: &mut SharedAllocator,
        isolate_id: engine::EngineId,
        value: Word,
        source: mir::Value,
        frame_state: engine::FrameStateId,
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
                .and_then(|value| {
                    super::frame::materialize_value(program, heap, shared, shared_allocator, value)
                })
                .map_err(RuntimeError::new)?;

        // capture the continuation after packaging the yielded result
        let continuation =
            self.capture_continuation(isolate_id, resume_frame_index, frame_state, options)?;

        Ok(Outcome::Yielded {
            continuation,
            value,
        })
    }

    /// Complete one control transfer produced by instruction execution.
    pub(crate) fn complete_transfer(
        &mut self,
        isolate_id: engine::EngineId,
        program: &Program,
        options: &IsolateOptions,
        externals: &HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_allocator: &mut SharedAllocator,
        current_func: &Function,
        transfer: Transfer,
    ) -> RuntimeResult<Option<Outcome>> {
        // dispatch the transfer to the matching machine opcode
        match transfer {
            Transfer::Continue => Err(self.runtime_error(program, Error::InvalidInstruction)),
            Transfer::Jump { block, moves } => {
                self.complete_jump(program, current_func, block, moves)?;
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
                self.complete_call(
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
                normal_state,
                unwind_state,
            } => {
                self.complete_call_branch(
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
                frame_state,
            } => self
                .complete_yield(
                    program,
                    options,
                    heap,
                    shared,
                    shared_allocator,
                    isolate_id,
                    value,
                    source,
                    frame_state,
                )
                .map(Some),
            Transfer::Throw(value) => self.complete_throw(program, value),
            Transfer::Return(value) => {
                self.complete_return(program, heap, shared, shared_allocator, value)
            }
            Transfer::Error(error) => Err(self.runtime_error(program, error)),
        }
    }
}
