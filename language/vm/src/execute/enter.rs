use std::collections::HashMap;
use std::ptr::NonNull;

use crate::Word;
use {destack_engine as engine, destack_mir as mir};

use super::frame::{
    FrameValue, function_return_type, load_arguments, load_planned_arguments, materialize_word,
    move_arguments_between_frames, move_values, store_parameters,
};
use crate::SharedHeap;
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::interpreter::{ExceptionalCall, Frame, Interpreter, Outcome};
use crate::isolate::{ExternalCallContext, ExternalFn};
use crate::options::IsolateOptions;
use crate::program::{ArgumentRange, CallTarget, Function, MoveRange, Program};
use destack_heap::{Heap, SharedRawLimits};

/// The lowered callee entry for one call.
struct LoweredCallee {
    /// The lowered program function pointer.
    function_ptr: NonNull<Function>,
}

impl Interpreter {
    /// Require one lowered callee from one call target.
    fn require_lowered_callee(
        program: &Program,
        function_id: mir::LocalNodeId<mir::Function>,
        target: CallTarget,
    ) -> RuntimeResult<LoweredCallee> {
        // require a lowered target first
        let function_index = match target {
            CallTarget::Local(index) => index,
            CallTarget::Import => {
                return Err(RuntimeError::new(Error::UndefinedFunction {
                    function: function_id,
                }));
            }
        };

        // load the lowered function pointer
        let function_ptr = program.functions.pointer(function_index).ok_or_else(|| {
            RuntimeError::new(Error::UndefinedFunction {
                function: function_id,
            })
        })?;

        Ok(LoweredCallee { function_ptr })
    }

    /// Call one imported function with pre-collected argument values.
    fn call_imported_function(
        &mut self,
        program: &Program,
        function_id: mir::LocalNodeId<mir::Function>,
        externals: &HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        arguments: &[FrameValue],
    ) -> RuntimeResult<Word> {
        // load the external handler first
        let function = program.tree.get(function_id);
        let name = program.strings.get(function.name).to_string();
        let handler = externals
            .get(&name)
            .cloned()
            .ok_or_else(|| self.runtime_error(program, Error::ExternalFunctionNotFound { name }))?;

        // encode argument values before crossing the runtime boundary
        let arguments = arguments
            .iter()
            .cloned()
            .map(|argument| argument.into_word().map_err(RuntimeError::new))
            .collect::<RuntimeResult<Vec<_>>>()?;

        // call through the external context
        let result = {
            let mut context =
                ExternalCallContext::new(program, heap, shared, SharedRawLimits::default());
            let result = handler(&mut context, &arguments);
            context
                .release_pins()
                .map_err(|error| self.runtime_error(program, error))?;
            result
        }
        .map_err(|error| self.runtime_error(program, error))?;

        Ok(result)
    }

    /// Push one lowered callee frame on the stack.
    fn push_lowered_call_frame(
        &mut self,
        program: &Program,
        options: &IsolateOptions,
        current_func: &Function,
        callee: LoweredCallee,
        arguments: ArgumentRange,
        env: Option<Word>,
        moves: Option<MoveRange>,
        resume_pc: usize,
        exceptional_call: Option<ExceptionalCall>,
    ) -> RuntimeResult<()> {
        // reject stack overflow before allocating anything
        if self.frames.len() >= options.limits.max_stack_depth {
            return Err(self.runtime_error(program, Error::StackOverflow));
        }

        // load the lowered callee entry metadata
        let (entry_block_ptr, frame_layout) = unsafe {
            let callee = callee.function_ptr.as_ref();
            let entry = callee.entry;
            let entry_block = &callee.blocks[entry as usize];

            (NonNull::from(entry_block), callee.frame_layout)
        };
        let frame_layout = program
            .frame_layout_by_id(frame_layout)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let (stack_offset, frame_base) = self.allocate_frame(frame_layout)?;

        // record the caller edge before mutating the stacks
        let caller_frame = self
            .frames
            .last_mut()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        caller_frame.resume_pc = resume_pc;
        caller_frame.exceptional_call = exceptional_call;

        let mut new_frame = Frame::new(
            unsafe { callee.function_ptr.as_ref().frame_layout },
            callee.function_ptr,
            entry_block_ptr,
            frame_layout,
            stack_offset,
            frame_base,
        );
        new_frame.set_environment(frame_layout, env);

        // bind arguments from the caller into the new frame
        let caller = self
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        if let Some(moves) = moves {
            move_values(
                program,
                caller,
                &mut new_frame,
                moves,
                current_func.move_pool.as_slice(),
            )
            .map_err(RuntimeError::new)?;
        } else {
            let callee_function = unsafe { callee.function_ptr.as_ref() };
            move_arguments_between_frames(
                program,
                caller,
                &mut new_frame,
                callee_function.argument_pool.as_slice(),
                callee_function.parameters,
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
        callee: LoweredCallee,
        arguments: &[FrameValue],
        env: Option<Word>,
    ) -> RuntimeResult<()> {
        // load the callee entry metadata first
        let (entry_block_ptr, frame_layout) = unsafe {
            let callee_function = callee.function_ptr.as_ref();
            let entry = callee_function.entry;
            let entry_block = &callee_function.blocks[entry as usize];

            (NonNull::from(entry_block), callee_function.frame_layout)
        };
        let frame_layout = program
            .frame_layout_by_id(frame_layout)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

        // replace the top frame bytes
        let stack_offset = self
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?
            .stack_offset;
        self.truncate_stack(stack_offset);
        let (stack_offset, frame_base) = self.allocate_frame(frame_layout)?;

        // retarget the frame to the callee
        let frame = self
            .frames
            .last_mut()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

        frame.frame_layout = unsafe { callee.function_ptr.as_ref().frame_layout };
        frame.function_ptr = callee.function_ptr;
        frame.block_ptr = entry_block_ptr;
        frame.resume_pc = 0;
        frame.exceptional_call = None;
        frame.replace_bytes(stack_offset, frame_layout.byte_len as usize, frame_base);
        frame.set_environment(frame_layout, env);

        // bind the new arguments into the reused frame
        let callee_function = unsafe { callee.function_ptr.as_ref() };
        store_parameters(
            program,
            frame,
            callee_function.argument_pool.as_slice(),
            callee_function.parameters,
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
        externals: &HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        current_func: &Function,
        function: u32,
        target: CallTarget,
        destination: Option<mir::Value>,
        arguments: ArgumentRange,
        env: Option<Word>,
        moves: Option<MoveRange>,
        resume_pc: usize,
    ) -> RuntimeResult<()> {
        // classify the call target
        let function_id = mir::LocalNodeId::<mir::Function>::new(function);

        // complete imported calls immediately in the caller frame
        if matches!(target, CallTarget::Import) {
            let caller = self
                .frames
                .last()
                .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

            // collect explicit call arguments in caller order
            let arguments = if let Some(moves) = moves {
                load_planned_arguments(
                    program,
                    self.frames.as_slice(),
                    caller,
                    current_func.move_pool.as_slice(),
                    moves,
                )?
            } else {
                load_arguments(
                    program,
                    self.frames.as_slice(),
                    caller,
                    current_func.argument_pool.as_slice(),
                    arguments,
                )?
            };

            // call the imported callee outside the lowered machine
            let result = self.call_imported_function(
                program,
                function_id,
                externals,
                heap,
                shared,
                &arguments,
            )?;

            // store the return value into the caller result
            let frame = self
                .frames
                .last_mut()
                .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
            let point = program.point(frame.function(), frame.current_block(), resume_pc as u32);
            let return_destination = program.return_destination_at(point)?.or(destination);

            if let Some(return_destination) = return_destination {
                let frame_layout = program
                    .frame_layout_by_id(frame.frame_layout)
                    .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                frame
                    .write_value_word(frame_layout, return_destination, result)
                    .map_err(RuntimeError::new)?;
            }

            // leave the caller positioned after the imported call
            frame.resume_pc = resume_pc;
            return Ok(());
        }

        // otherwise enter the lowered callee on a new frame
        let callee = Self::require_lowered_callee(program, function_id, target)?;
        self.push_lowered_call_frame(
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

    /// Complete one exceptional call from the current frame.
    pub(crate) fn complete_call_branch(
        &mut self,
        program: &Program,
        options: &IsolateOptions,
        externals: &HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        current_func: &Function,
        function: u32,
        target: CallTarget,
        arguments: ArgumentRange,
        env: Option<Word>,
        normal_state: engine::FrameStateId,
        unwind_state: engine::FrameStateId,
    ) -> RuntimeResult<()> {
        // classify the call target
        let function_id = mir::LocalNodeId::<mir::Function>::new(function);

        // imported exceptional calls resume the normal branch immediately
        if matches!(target, CallTarget::Import) {
            let caller = self
                .frames
                .last()
                .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

            // collect explicit branch-call arguments first
            let arguments = load_arguments(
                program,
                self.frames.as_slice(),
                caller,
                current_func.argument_pool.as_slice(),
                arguments,
            )?;

            // call the imported callee and continue through the normal branch
            let result = self.call_imported_function(
                program,
                function_id,
                externals,
                heap,
                shared,
                &arguments,
            )?;

            self.enter_caller_state_word(program, normal_state, result)?;
            return Ok(());
        }

        // otherwise push the lowered callee and record the exceptional edge
        let callee = Self::require_lowered_callee(program, function_id, target)?;
        let caller = self
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

        // resume after the terminator once the branch call completes
        let current_block = unsafe { caller.block_ptr.as_ref() };
        let resume_pc = current_block.instructions.len();
        let exceptional_call = ExceptionalCall {
            normal_state,
            unwind_state,
        };

        self.push_lowered_call_frame(
            program,
            options,
            current_func,
            callee,
            arguments,
            env,
            None,
            resume_pc,
            Some(exceptional_call),
        )
    }

    /// Complete one tail call in the current frame.
    pub(crate) fn complete_tail_call(
        &mut self,
        program: &Program,
        externals: &HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
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
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        let argument_values = if let Some(moves) = moves {
            load_planned_arguments(
                program,
                self.frames.as_slice(),
                caller,
                current_func.move_pool.as_slice(),
                moves,
            )?
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

        // complete imported tail calls before returning to the caller
        if matches!(target, CallTarget::Import) {
            let result = self.call_imported_function(
                program,
                function_id,
                externals,
                heap,
                shared,
                &argument_values,
            )?;

            // discard the current frame before delivering the tail-call result
            let frame = self
                .frames
                .pop()
                .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
            self.truncate_stack(frame.stack_offset);

            // complete execution immediately when there is no caller left
            if self.frames.is_empty() {
                let return_type =
                    function_return_type(program, function_id).map_err(RuntimeError::new)?;
                let result =
                    materialize_word(program, return_type, result).map_err(RuntimeError::new)?;

                return Ok(Some(self.complete_execution(result)));
            }

            // otherwise store the result into the caller return destination
            let caller = self
                .frames
                .last_mut()
                .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
            let point = program.point(
                caller.function(),
                caller.current_block(),
                caller.resume_pc as u32,
            );
            if let Some(destination) = program.return_destination_at(point)? {
                let frame_layout = program
                    .frame_layout_by_id(caller.frame_layout)
                    .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                caller
                    .write_value_word(frame_layout, destination, result)
                    .map_err(RuntimeError::new)?;
            }

            return Ok(None);
        }

        // otherwise reuse the current frame for the lowered callee
        let callee = Self::require_lowered_callee(program, function_id, target)?;
        self.reuse_tail_call_frame(program, callee, &argument_values, env)?;

        Ok(None)
    }
}
