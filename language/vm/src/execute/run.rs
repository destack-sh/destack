use engine::StaticSpace;
use {destack_engine as engine, destack_mir as mir};

use super::frame::{dematerialize_value, frame_value_type};
use super::{dispatch_block, dispatch_block_counted};
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::interpreter::{Continuation, Frame, Interpreter, Machine, Outcome};
use crate::options::IsolateOptions;
use crate::program::{CallTarget, Program};
use crate::{SharedHeap, Word};
use destack_heap::{Heap, SharedAllocationCache, SharedGcWorker};

impl Interpreter {
    /// Execute a function by id.
    ///
    /// Uses the lowered op loop for maximum performance.
    /// Functions are lowered when the interpreter is created.
    pub(crate) fn run_function(
        &mut self,
        isolate_id: engine::EngineId,
        program: &Program,
        options: &IsolateOptions,
        statics: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut SharedAllocationCache,
        shared_gc: &SharedGcWorker,
        function_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Word],
    ) -> RuntimeResult<engine::Value> {
        let outcome = self.run_function_yielding(
            isolate_id,
            program,
            options,
            statics,
            heap,
            shared,
            shared_cache,
            shared_gc,
            function_id,
            arguments,
        )?;

        match outcome {
            Outcome::Completed { value } => Ok(value),
            Outcome::Yielded { .. } => Err(self.runtime_error(program, Error::UnexpectedYield)),
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
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut SharedAllocationCache,
        shared_gc: &SharedGcWorker,
        function_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Word],
    ) -> RuntimeResult<Outcome> {
        self.reset_stack(options)?;

        // resolve the function target before entering the main loop
        match program.functions.call_target(function_id) {
            Some(CallTarget::Import) => {
                let function = program.tree.get(function_id);
                let name = program.strings.get(function.name).to_string();

                return Err(self.runtime_error(program, Error::BindingCallForbidden { name }));
            }
            Some(CallTarget::Local(_)) => {}

            // reject missing functions loudly
            None => {
                return Err(self.runtime_error(
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
            heap,
            shared,
            shared_cache,
            shared_gc,
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
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut SharedAllocationCache,
        shared_gc: &SharedGcWorker,
        continuation: Continuation,
        received_value: engine::Value,
    ) -> RuntimeResult<Outcome> {
        if continuation.isolate_id != isolate_id {
            return Err(self.runtime_error(program, Error::InvalidContinuation));
        }

        if !self.frames.is_empty() {
            return Err(self.runtime_error(program, Error::InvalidContinuation));
        }

        self.stack = continuation.stack;
        self.frames = continuation.frames;

        self.resume_continuation(
            isolate_id,
            program,
            options,
            statics,
            heap,
            shared,
            shared_cache,
            shared_gc,
            continuation.resume_frame_index,
            continuation.frame_state,
            received_value,
        )
    }

    /// Resume execution from one suspended yield point.
    fn resume_continuation(
        &mut self,
        isolate_id: engine::EngineId,
        program: &Program,
        options: &IsolateOptions,
        statics: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut SharedAllocationCache,
        shared_gc: &SharedGcWorker,
        resume_frame_index: usize,
        frame_state: engine::FrameStateId,
        received_value: engine::Value,
    ) -> RuntimeResult<Outcome> {
        let frame_entry = program.frame_entry(frame_state);
        let received_value_slot = frame_entry
            .and_then(|frame_entry| frame_entry.received_value)
            .ok_or_else(|| self.runtime_error(program, Error::InvalidContinuation))?;
        let frame = self
            .frames
            .get(resume_frame_index)
            .ok_or_else(|| self.runtime_error(program, Error::InvalidContinuation))?;
        let layout = program
            .frame_layout_by_id(frame.frame_layout())
            .ok_or_else(|| self.runtime_error(program, Error::InvalidContinuation))?;
        let received_value_id = layout
            .value_for_slot(received_value_slot)
            .ok_or_else(|| self.runtime_error(program, Error::InvalidContinuation))?;
        let received_type = frame_value_type(program, frame, mir::Value::new(received_value_id))
            .map_err(|_| self.runtime_error(program, Error::InvalidContinuation))?;
        let received_value =
            dematerialize_value(program, heap, shared, received_type, &received_value)
                .map_err(|_| self.runtime_error(program, Error::InvalidContinuation))?;

        self.enter_frame_state(
            program,
            resume_frame_index,
            frame_state,
            Some(received_value),
        )
        .map_err(|error| RuntimeError {
            error: Error::InvalidContinuation,
            stack: error.stack,
            anchor: error.anchor,
        })?;

        self.run_loop(
            isolate_id,
            program,
            options,
            statics,
            heap,
            shared,
            shared_cache,
            shared_gc,
        )
    }

    /// Assemble a completed execution outcome.
    pub(crate) fn complete_execution(&mut self, value: engine::Value) -> Outcome {
        Outcome::Completed { value }
    }

    /// Run one lowered function from its entry block.
    fn run_function_body(
        &mut self,
        isolate_id: engine::EngineId,
        program: &Program,
        options: &IsolateOptions,
        statics: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut SharedAllocationCache,
        shared_gc: &SharedGcWorker,
        function_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Word],
    ) -> RuntimeResult<Outcome> {
        // resolve the lowered entry metadata
        let function = program
            .functions
            .function_by_id(function_id)
            .ok_or_else(|| {
                RuntimeError::new(Error::UndefinedFunction {
                    function: function_id,
                })
            })?;
        let entry_block = function.entry;
        let frame_layout = function.frame_layout;
        let frame_layout_ref = program
            .frame_layout_by_id(frame_layout)
            .ok_or_else(|| self.runtime_error(program, Error::InvalidInstruction))?;
        let (stack_offset, frame_base) = self.allocate_frame(frame_layout_ref)?;

        // create the entry frame
        let frame = Frame::new(
            function,
            entry_block,
            frame_layout_ref,
            stack_offset,
            frame_base,
        );

        // collect entry parameters before moving the frame onto the stack
        let parameter_slice = function.parameters.slice(function.argument_pool.as_slice());

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
            let mut machine = Machine::new(
                program,
                options,
                statics,
                heap,
                shared,
                shared_gc,
                shared_cache,
                self,
                frame_index,
                function,
            )
            .map_err(RuntimeError::new)?;
            for (index, param) in parameter_slice.iter().enumerate() {
                let value = arguments[index];
                let is_word = machine.value_is_word(*param).map_err(RuntimeError::new)?;
                if is_word {
                    machine.store_value_word(*param, value);
                    continue;
                }

                let ty = machine.value_type(*param).map_err(RuntimeError::new)?;
                let bytes = super::frame::encode_argument_bytes(&mut machine, ty, value)
                    .map_err(RuntimeError::new)?;
                let destination = machine.value_bytes_mut(*param).map_err(RuntimeError::new)?;
                if destination.len() != bytes.len() {
                    return Err(RuntimeError::new(Error::InvalidInstruction));
                }
                destination.copy_from_slice(&bytes);
            }
        }

        self.run_loop(
            isolate_id,
            program,
            options,
            statics,
            heap,
            shared,
            shared_cache,
            shared_gc,
        )
    }

    /// Run the interpreter loop from the current stack.
    fn run_loop(
        &mut self,
        isolate_id: engine::EngineId,
        program: &Program,
        options: &IsolateOptions,
        statics: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut SharedAllocationCache,
        shared_gc: &SharedGcWorker,
    ) -> RuntimeResult<Outcome> {
        let mut lowered_instructions_executed = 0;

        // require at least one live frame before stepping
        if self.frames.is_empty() {
            return Err(self.runtime_error(program, Error::InvalidInstruction));
        }

        loop {
            // enforce the instruction limit before stepping again
            if let Some(max) = options.limits.max_instructions
                && lowered_instructions_executed >= max
            {
                return Err(self.runtime_error(program, Error::StepLimitExceeded));
            }

            // load the current frame position and clear any pending pc
            let (function_id, block_index, start_pc) = {
                let frame = self
                    .frames
                    .last_mut()
                    .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                let pc = frame.pc;
                frame.pc = 0;
                (frame.function(), frame.block, pc)
            };

            let current_func = program
                .functions
                .function_by_id(function_id)
                .ok_or_else(|| {
                    RuntimeError::new(Error::UndefinedFunction {
                        function: function_id,
                    })
                })?;

            // run the current lowered block from the chosen instruction offset
            let block_run = {
                let frame_index = self.frames.len() - 1;
                let mut machine = Machine::new(
                    program,
                    options,
                    statics,
                    heap,
                    shared,
                    shared_gc,
                    shared_cache,
                    self,
                    frame_index,
                    current_func,
                )
                .map_err(RuntimeError::new)?;

                if options.limits.max_instructions.is_some() {
                    let block_run =
                        dispatch_block_counted(&mut machine, current_func, block_index, start_pc);

                    (block_run.transfer, block_run.executed)
                } else {
                    (
                        dispatch_block(&mut machine, current_func, block_index, start_pc),
                        0,
                    )
                }
            };

            // record the instructions covered by this block run
            if options.limits.max_instructions.is_some() {
                lowered_instructions_executed += block_run.1;
            }

            let transfer = block_run.0;

            // reload the current function after any direct call path rewrites
            let current_func = {
                let frame = self
                    .frames
                    .last()
                    .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
                let function_id = frame.function();

                program
                    .functions
                    .function_by_id(function_id)
                    .ok_or_else(|| {
                        RuntimeError::new(Error::UndefinedFunction {
                            function: function_id,
                        })
                    })?
            };

            // apply the transfer and stop once it produces an outcome
            if let Some(outcome) = self.complete_transfer(
                isolate_id,
                program,
                options,
                heap,
                shared,
                shared_cache,
                shared_gc,
                current_func,
                transfer,
            )? {
                return Ok(outcome);
            }
        }
    }
}
