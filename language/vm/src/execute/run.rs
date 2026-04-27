use std::ptr::NonNull;

use destack_engine::{self as engine, StaticSpace};
use destack_mir as mir;

use super::dispatch_instruction;
use super::frame::{
    frame_value_from_materialized, frame_value_type, function_return_type, materialize_word,
};
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::interpreter::{Continuation, DispatchState, Frame, Interpreter, Outcome, Output};
use crate::isolate::{ExternalCallContext, ExternalFn};
use crate::options::IsolateOptions;
use crate::program::{Block, CallTarget, Function, Program};
use crate::{SharedHeap, Word};
use destack_heap::{Heap, SharedRawLimits};

impl Interpreter {
    /// Execute a function by name.
    ///
    /// Looks up a function in the MIR tree by name and executes it.
    pub(crate) fn run_function_by_name(
        &mut self,
        isolate_id: engine::EngineId,
        program: &Program,
        options: &IsolateOptions,
        statics: &mut StaticSpace,
        externals: &std::collections::HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        name: &str,
        arguments: &[Word],
    ) -> RuntimeResult<Output> {
        let outcome = self.run_function_by_name_yielding(
            isolate_id, program, options, statics, externals, heap, shared, name, arguments,
        )?;

        match outcome {
            Outcome::Completed { output } => Ok(output),
            Outcome::Yielded { .. } => Err(self.make_error(program, Error::UnexpectedYield)),
        }
    }

    /// Execute a function by name with yield support.
    ///
    /// Returns a yielded value when the coroutine suspends.
    pub(crate) fn run_function_by_name_yielding(
        &mut self,
        isolate_id: engine::EngineId,
        program: &Program,
        options: &IsolateOptions,
        statics: &mut StaticSpace,
        externals: &std::collections::HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        name: &str,
        arguments: &[Word],
    ) -> RuntimeResult<Outcome> {
        let function_id = program
            .function_id_by_name
            .get(name)
            .copied()
            .ok_or_else(|| {
                self.make_error(
                    program,
                    Error::ExternalFunctionNotFound {
                        name: name.to_string(),
                    },
                )
            })?;

        self.run_function_yielding(
            isolate_id,
            program,
            options,
            statics,
            externals,
            heap,
            shared,
            function_id,
            arguments,
        )
    }

    /// Execute a function by id.
    ///
    /// Uses direct dispatch for maximum performance.
    /// Functions are lowered when the interpreter is created.
    pub(crate) fn run_function(
        &mut self,
        isolate_id: engine::EngineId,
        program: &Program,
        options: &IsolateOptions,
        statics: &mut StaticSpace,
        externals: &std::collections::HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        function_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Word],
    ) -> RuntimeResult<Output> {
        let outcome = self.run_function_yielding(
            isolate_id,
            program,
            options,
            statics,
            externals,
            heap,
            shared,
            function_id,
            arguments,
        )?;

        match outcome {
            Outcome::Completed { output } => Ok(output),
            Outcome::Yielded { .. } => Err(self.make_error(program, Error::UnexpectedYield)),
        }
    }

    /// Execute a function by id with yield support.
    ///
    /// Returns a yielded value when the coroutine suspends.
    pub(crate) fn run_function_yielding(
        &mut self,
        isolate_id: engine::EngineId,
        program: &Program,
        options: &IsolateOptions,
        statics: &mut StaticSpace,
        externals: &std::collections::HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        function_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Word],
    ) -> RuntimeResult<Outcome> {
        self.reset_stack(options);

        // resolve the function target before entering the main loop
        match program.functions.resolve(function_id) {
            Some(CallTarget::Import) => {
                // call imports directly without entering the lowered machine
                let function = program.tree.get(function_id);
                let name = program.strings.get(function.name).to_string();
                let handler = externals.get(&name).cloned().ok_or_else(|| {
                    self.make_error(program, Error::ExternalFunctionNotFound { name })
                })?;
                let value = {
                    let mut context =
                        ExternalCallContext::new(program, heap, shared, SharedRawLimits::default());
                    let value = handler(&mut context, arguments);
                    context
                        .release_pins()
                        .map_err(|error| self.make_error(program, error))?;
                    value
                }
                .map_err(|error| self.make_error(program, error))?;

                let return_type =
                    function_return_type(program, function_id).map_err(RuntimeError::new)?;
                let value =
                    materialize_word(program, return_type, value).map_err(RuntimeError::new)?;

                return Ok(self.complete_execution(heap, value));
            }
            Some(CallTarget::Local(_)) => {}

            // reject missing functions loudly
            None => {
                return Err(self.make_error(
                    program,
                    Error::UndefinedFunction {
                        function: function_id,
                    },
                ));
            }
        }

        self.run_function_body(
            isolate_id,
            program,
            options,
            statics,
            externals,
            heap,
            shared,
            function_id,
            arguments,
        )
    }

    /// Resume a previously yielded coroutine.
    ///
    /// The resume value is appended after explicit resume arguments.
    pub(crate) fn resume(
        &mut self,
        isolate_id: engine::EngineId,
        program: &Program,
        options: &IsolateOptions,
        statics: &mut StaticSpace,
        externals: &std::collections::HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        continuation: Continuation,
        resume_value: engine::Value,
    ) -> RuntimeResult<Outcome> {
        if continuation.isolate_id != isolate_id {
            return Err(self.make_error(program, Error::InvalidContinuation));
        }

        if !self.frames.is_empty() {
            return Err(self.make_error(program, Error::InvalidContinuation));
        }

        self.stack = continuation.stack;
        self.frames = continuation.frames;

        self.resume_continuation(
            isolate_id,
            program,
            options,
            statics,
            externals,
            heap,
            shared,
            continuation.resume_frame_index,
            continuation.resume_point,
            resume_value,
        )
    }

    /// Resume execution from one suspended yield point.
    fn resume_continuation(
        &mut self,
        isolate_id: engine::EngineId,
        program: &Program,
        options: &IsolateOptions,
        statics: &mut StaticSpace,
        externals: &std::collections::HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        resume_frame_index: usize,
        resume_point: engine::ResumePointId,
        resume_value: engine::Value,
    ) -> RuntimeResult<Outcome> {
        let resume_point_metadata = program
            .resume_point(resume_point)
            .ok_or_else(|| self.make_error(program, Error::InvalidContinuation))?;
        let resume_transfer = resume_point_metadata
            .transfer
            .and_then(|resume_transfer| program.resume_transfer(resume_transfer));
        let resume_value_region = resume_transfer
            .and_then(|resume_transfer| resume_transfer.resume_value)
            .ok_or_else(|| self.make_error(program, Error::InvalidContinuation))?;
        let frame = self
            .frames
            .get(resume_frame_index)
            .ok_or_else(|| self.make_error(program, Error::InvalidContinuation))?;
        let layout = program
            .frame_layout_by_id(frame.frame_layout)
            .ok_or_else(|| self.make_error(program, Error::InvalidContinuation))?;
        let resume_value_id = layout
            .value_for_region(resume_value_region)
            .ok_or_else(|| self.make_error(program, Error::InvalidContinuation))?;
        let resume_type = frame_value_type(program, frame, mir::Value::new(resume_value_id.0))
            .map_err(|_| self.make_error(program, Error::InvalidContinuation))?;
        let resume_value = frame_value_from_materialized(resume_type, &resume_value)
            .map_err(|_| self.make_error(program, Error::InvalidContinuation))?;

        self.apply_resume_point_to_frame(
            program,
            resume_frame_index,
            resume_point,
            Some(resume_value),
        )
        .map_err(|error| RuntimeError {
            error: Error::InvalidContinuation,
            stack: error.stack,
            anchor: error.anchor,
        })?;

        self.run_loop(
            isolate_id, program, options, statics, externals, heap, shared,
        )
    }

    /// Assemble a completed execution outcome.
    pub(crate) fn complete_execution(&mut self, _heap: &Heap, value: engine::Value) -> Outcome {
        // package the final runtime output
        let output = Output { value };

        Outcome::Completed { output }
    }

    /// Run one lowered function from its entry block.
    fn run_function_body(
        &mut self,
        isolate_id: engine::EngineId,
        program: &Program,
        options: &IsolateOptions,
        statics: &mut StaticSpace,
        externals: &std::collections::HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
        function_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Word],
    ) -> RuntimeResult<Outcome> {
        // resolve the lowered entry metadata
        let function_ptr = program.functions.get_ptr_for(function_id).ok_or_else(|| {
            RuntimeError::new(Error::UndefinedFunction {
                function: function_id,
            })
        })?;
        let (entry_block_id, entry_block_ptr) = unsafe {
            let function = function_ptr.as_ref();
            let entry = function.entry;
            let entry_block = &function.blocks[entry as usize];

            (entry_block.mir_block, NonNull::from(entry_block))
        };
        let frame_layout = unsafe { function_ptr.as_ref().frame_layout };
        let frame_layout_ref = program
            .frame_layout_by_id(frame_layout)
            .ok_or_else(|| self.make_error(program, Error::InvalidInstruction))?;
        let (stack_offset, frame_base) = self.allocate_frame(frame_layout_ref, options)?;

        // allocate the entry frame
        let frame = Frame::new(
            frame_layout,
            function_id,
            function_ptr,
            entry_block_ptr,
            entry_block_id,
            frame_layout_ref,
            stack_offset,
            frame_base,
        );

        // collect entry parameters before moving the frame onto the stack
        let parameter_slice = unsafe {
            let function = function_ptr.as_ref();
            function.parameters.slice(function.argument_pool.as_slice())
        };

        // push the entry frame
        self.frames.push(frame);

        // entry calls must provide exactly the function parameters
        if arguments.len() != parameter_slice.len() {
            return Err(RuntimeError::new(Error::TypeMismatch {
                expected: format!("{} function arguments", parameter_slice.len()),
                actual: arguments.len().to_string(),
            }));
        }

        // bind explicit entry arguments through the same frame move path as MIR values
        {
            let frame_index = self.frames.len() - 1;
            let argument_pool = unsafe { function_ptr.as_ref().argument_pool.as_slice() };
            let mut state = DispatchState::new(
                program,
                options,
                statics,
                heap,
                shared,
                self,
                frame_index,
                argument_pool,
            )
            .map_err(RuntimeError::new)?;
            for (index, param) in parameter_slice.iter().enumerate() {
                let value = arguments[index];
                let is_word = state.value_is_word(*param).map_err(RuntimeError::new)?;
                if is_word {
                    state.set_word(*param, value);
                    continue;
                }

                let ty = state.value_type(*param).map_err(RuntimeError::new)?;
                let bytes = super::bytes::encode_argument_bytes(&mut state, ty, value)
                    .map_err(RuntimeError::new)?;
                let target = state.value_bytes_mut(*param).map_err(RuntimeError::new)?;
                if target.len() != bytes.len() {
                    return Err(RuntimeError::new(Error::InvalidInstruction));
                }
                target.copy_from_slice(&bytes);
            }
        }

        self.run_loop(
            isolate_id, program, options, statics, externals, heap, shared,
        )
    }

    /// Run the interpreter loop from the current stack.
    fn run_loop(
        &mut self,
        isolate_id: engine::EngineId,
        program: &Program,
        options: &IsolateOptions,
        statics: &mut StaticSpace,
        externals: &std::collections::HashMap<String, ExternalFn>,
        heap: &mut Heap,
        shared: &SharedHeap,
    ) -> RuntimeResult<Outcome> {
        let mut lowered_instructions_executed = 0;

        // require at least one live frame before stepping
        if self.frames.is_empty() {
            return Err(self.make_error(program, Error::InvalidInstruction));
        }

        loop {
            // enforce the instruction limit before stepping again
            if let Some(max) = options.limits.max_instructions
                && lowered_instructions_executed >= max
            {
                return Err(self.make_error(program, Error::StepLimitExceeded));
            }

            // load the current frame position and clear any pending resume pc
            let (function_ptr, block_ptr, start_pc) = {
                let frame = self
                    .frames
                    .last_mut()
                    .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                let pc = frame.resume_pc;
                frame.resume_pc = 0;
                (frame.function_ptr, frame.block_ptr, pc)
            };

            let current_func: &Function = unsafe { function_ptr.as_ref() };
            let block: &Block = unsafe { block_ptr.as_ref() };
            let block_len = block.instructions.len();

            // dispatch the current lowered block from the chosen instruction offset
            let transfer = {
                let frame_index = self.frames.len() - 1;
                let mut state = DispatchState::new(
                    program,
                    options,
                    statics,
                    heap,
                    shared,
                    self,
                    frame_index,
                    current_func.argument_pool.as_slice(),
                )
                .map_err(RuntimeError::new)?;
                dispatch_instruction(&mut state, &block.instructions, start_pc)
            };

            // record the instructions covered by this dispatch
            if options.limits.max_instructions.is_some() {
                lowered_instructions_executed += (block_len - start_pc) as u64;
            }
            // reload the current function after any direct call fast path rewrites
            let current_func = {
                let frame = self
                    .frames
                    .last()
                    .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                unsafe { frame.function_ptr.as_ref() }
            };

            // apply the transfer and stop once it produces an outcome
            if let Some(outcome) = self.apply_transfer(
                isolate_id,
                program,
                options,
                externals,
                heap,
                shared,
                current_func,
                transfer,
            )? {
                return Ok(outcome);
            }
        }
    }
}
