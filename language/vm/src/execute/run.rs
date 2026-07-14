use destack_heap::{AllocationCache, Heap, SharedHeap, SharedMarkWorker};
use destack_program as program;
use destack_program::Program;
use destack_program::vm::Cell;
use program::{FrameStateId, FunctionId, StaticSpace};

use super::frame::dematerialize_value;
use super::{dispatch_block, dispatch_block_limited};
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::machine::{Activation, Continuation, Frame, Machine, Outcome};
use crate::options::LimitOptions;

impl Machine {
    /// Execute a function by id.
    ///
    /// Uses the lowered op loop for maximum performance.
    /// Functions are linked into VM code before the machine is created.
    pub(crate) fn execute_function_cells(
        &mut self,
        program: &Program,
        limits: LimitOptions,
        statics: &mut StaticSpace,
        shared_static: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut AllocationCache,
        shared_mark_worker: &SharedMarkWorker,
        stop_points: Option<&program::StopSet>,
        watch_points: Option<&program::WatchSet>,
        profile: Option<&mut program::Profile>,
        function_id: FunctionId,
        arguments: &[Cell],
    ) -> RuntimeResult<program::Value> {
        let outcome = self.execute_function_cells_yielding(
            program,
            limits,
            statics,
            shared_static,
            heap,
            shared,
            shared_cache,
            shared_mark_worker,
            stop_points,
            watch_points,
            profile,
            function_id,
            arguments,
        )?;

        match outcome {
            Outcome::Completed { value } => Ok(value),
            Outcome::Yielded { .. } => Err(self.runtime_error(Error::unexpected_yield())),
            Outcome::Stopped { .. } => Err(self.runtime_error(Error::unexpected_stop())),
        }
    }

    /// Execute a function by id with yield support.
    ///
    /// Returns a yielded value when the coroutine suspends.
    pub(crate) fn execute_function_cells_yielding(
        &mut self,
        program: &Program,
        limits: LimitOptions,
        statics: &mut StaticSpace,
        shared_static: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut AllocationCache,
        shared_mark_worker: &SharedMarkWorker,
        stop_points: Option<&program::StopSet>,
        watch_points: Option<&program::WatchSet>,
        profile: Option<&mut program::Profile>,
        function_id: FunctionId,
        arguments: &[Cell],
    ) -> RuntimeResult<Outcome> {
        self.reset_stack(limits)?;

        // resolve the function target before entering the main loop
        match program.vm_call_target(function_id) {
            Some(target) if target.is_binding() => {
                let name = program
                    .function(function_id)
                    .map(|function| {
                        program
                            .string(function.name)
                            .map(str::to_owned)
                            .ok_or_else(|| {
                                self.runtime_error(Error::invalid_program(format!(
                                    "missing function name string {:?}",
                                    function.name
                                )))
                            })
                    })
                    .transpose()?
                    .ok_or_else(|| {
                        self.runtime_error(Error::invalid_program(format!(
                            "missing function tables for {function_id:?}"
                        )))
                    })?;

                return Err(self.runtime_error(Error::import_forbidden(name)));
            }
            Some(_) => {}

            // reject missing functions loudly
            None => {
                return Err(self.runtime_error(Error::undefined_function(function_id)));
            }
        }

        self.run_function_body(
            program,
            limits,
            statics,
            shared_static,
            heap,
            shared,
            shared_cache,
            shared_mark_worker,
            stop_points,
            watch_points,
            profile,
            function_id,
            arguments,
        )
    }

    /// Resume a previously yielded coroutine.
    ///
    /// The resume value is bound before explicit resume arguments.
    pub(crate) fn execute_resume(
        &mut self,
        program: &Program,
        limits: LimitOptions,
        statics: &mut StaticSpace,
        shared_static: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut AllocationCache,
        shared_mark_worker: &SharedMarkWorker,
        stop_points: Option<&program::StopSet>,
        watch_points: Option<&program::WatchSet>,
        profile: Option<&mut program::Profile>,
        continuation: Continuation,
        received_value: program::Value,
    ) -> RuntimeResult<Outcome> {
        if !self.frames.is_empty() {
            return Err(self.runtime_error(Error::invalid_continuation()));
        }

        let (frames, resume_frame_index, frame_state) =
            self.restore_continuation(&continuation, program)?;
        self.frames = frames;

        self.resume_continuation(
            program,
            limits,
            statics,
            shared_static,
            heap,
            shared,
            shared_cache,
            shared_mark_worker,
            stop_points,
            watch_points,
            profile,
            resume_frame_index,
            frame_state,
            received_value,
        )
    }

    /// Continue a restored continuation without a received value.
    pub(crate) fn execute_continue(
        &mut self,
        program: &Program,
        limits: LimitOptions,
        statics: &mut StaticSpace,
        shared_static: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut AllocationCache,
        shared_mark_worker: &SharedMarkWorker,
        stop_points: Option<&program::StopSet>,
        watch_points: Option<&program::WatchSet>,
        profile: Option<&mut program::Profile>,
        resume_skip: Option<program::ResumeSkip>,
        continuation: Continuation,
    ) -> RuntimeResult<Outcome> {
        if !self.frames.is_empty() {
            return Err(self.runtime_error(Error::invalid_continuation()));
        }

        let (frames, resume_frame_index, frame_state) =
            self.restore_continuation(&continuation, program)?;
        self.frames = frames;

        self.continue_restored_continuation(
            program,
            limits,
            statics,
            shared_static,
            heap,
            shared,
            shared_cache,
            shared_mark_worker,
            stop_points,
            watch_points,
            profile,
            resume_skip,
            resume_frame_index,
            frame_state,
        )
    }

    /// Resume execution from one suspended yield point.
    fn resume_continuation(
        &mut self,
        program: &Program,
        limits: LimitOptions,
        statics: &mut StaticSpace,
        shared_static: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut AllocationCache,
        shared_mark_worker: &SharedMarkWorker,
        stop_points: Option<&program::StopSet>,
        watch_points: Option<&program::WatchSet>,
        mut profile: Option<&mut program::Profile>,
        resume_frame_index: usize,
        frame_state: FrameStateId,
        received_value: program::Value,
    ) -> RuntimeResult<Outcome> {
        let frame_entry = program.frame_entry(frame_state);
        let received_value_slot = frame_entry
            .and_then(|frame_entry| frame_entry.received_value())
            .ok_or_else(|| self.runtime_error(Error::invalid_continuation()))?;
        let frame = self
            .frames
            .get(resume_frame_index)
            .ok_or_else(|| self.runtime_error(Error::invalid_continuation()))?;
        let layout = program
            .frame_layout_by_id(frame.frame_layout())
            .ok_or_else(|| self.runtime_error(Error::invalid_continuation()))?;
        let received_slot = program
            .frame_slot(layout, received_value_slot)
            .ok_or_else(|| self.runtime_error(Error::invalid_continuation()))?;
        let received_type = received_slot.ty;
        let received_value =
            dematerialize_value(program, heap, shared, received_type, &received_value)
                .map_err(|_| self.runtime_error(Error::invalid_continuation()))?;

        self.enter_frame_state(
            program,
            resume_frame_index,
            frame_state,
            Some(received_value),
        )
        .map_err(|error| RuntimeError {
            error: Error::invalid_continuation(),
            stack: error.stack,
            anchor: error.anchor,
        })?;

        if let Some(profile) = profile.as_deref_mut() {
            let Some((site, _)) = program
                .sites()
                .continuation_state(program.sections(), frame_state)
            else {
                return Err(RuntimeError::new(Error::invalid_instruction()));
            };

            profile.record_continuation_resume(site);
        }

        let mut activation = Activation::new(
            program,
            self,
            statics,
            shared_static,
            heap,
            shared,
            shared_mark_worker,
            shared_cache,
            stop_points,
            watch_points,
            profile,
            None,
        );

        activation.run_loop(limits)
    }

    /// Assemble a completed execution outcome.
    pub(crate) fn complete_execution(&mut self, value: program::Value) -> Outcome {
        Outcome::Completed { value }
    }

    /// Continue execution from one materialized frame state.
    fn continue_restored_continuation(
        &mut self,
        program: &Program,
        limits: LimitOptions,
        statics: &mut StaticSpace,
        shared_static: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut AllocationCache,
        shared_mark_worker: &SharedMarkWorker,
        stop_points: Option<&program::StopSet>,
        watch_points: Option<&program::WatchSet>,
        profile: Option<&mut program::Profile>,
        resume_skip: Option<program::ResumeSkip>,
        frame_index: usize,
        frame_state: FrameStateId,
    ) -> RuntimeResult<Outcome> {
        self.enter_frame_state(program, frame_index, frame_state, None)
            .map_err(|error| RuntimeError {
                error: Error::invalid_continuation(),
                stack: error.stack,
                anchor: error.anchor,
            })?;

        let mut activation = Activation::new(
            program,
            self,
            statics,
            shared_static,
            heap,
            shared,
            shared_mark_worker,
            shared_cache,
            stop_points,
            watch_points,
            profile,
            resume_skip,
        );

        activation.run_loop(limits)
    }

    /// Run one lowered function from its entry block.
    fn run_function_body(
        &mut self,
        program: &Program,
        limits: LimitOptions,
        statics: &mut StaticSpace,
        shared_static: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut AllocationCache,
        shared_mark_worker: &SharedMarkWorker,
        stop_points: Option<&program::StopSet>,
        watch_points: Option<&program::WatchSet>,
        profile: Option<&mut program::Profile>,
        function_id: FunctionId,
        arguments: &[Cell],
    ) -> RuntimeResult<Outcome> {
        // resolve the lowered entry tables
        let function = program
            .vm_function_by_id(function_id)
            .ok_or_else(|| RuntimeError::new(Error::undefined_function(function_id)))?;
        let entry_block = function.function.entry;
        let frame_layout = function.function.frame_layout;
        let frame_layout_ref = program
            .frame_layout_by_id(frame_layout)
            .ok_or_else(|| self.runtime_error(Error::invalid_instruction()))?;
        let (stack_offset, frame_base) = self.allocate_frame(frame_layout_ref)?;

        // create the entry frame
        let frame = Frame::new(
            &function,
            entry_block,
            frame_layout_ref,
            stack_offset,
            frame_base,
        );

        // collect entry parameters before moving the frame onto the stack
        let parameter_slice = function.function.parameters.slice(function.argument_pool);

        // push the entry frame
        self.frames.push(frame);

        // entry calls must provide exactly the function parameters
        if arguments.len() != parameter_slice.len() {
            return Err(RuntimeError::new(Error::type_mismatch(
                format!("{} function arguments", parameter_slice.len()),
                arguments.len().to_string(),
            )));
        }

        let mut activation = Activation::new(
            program,
            self,
            statics,
            shared_static,
            heap,
            shared,
            shared_mark_worker,
            shared_cache,
            stop_points,
            watch_points,
            profile,
            None,
        );

        // bind explicit entry arguments through the same frame move path as MIR values
        let frame_index = activation.machine.frames.len() - 1;
        activation
            .bind_frame(frame_index)
            .map_err(RuntimeError::new)?;
        for (index, param) in parameter_slice.iter().enumerate() {
            let value = arguments[index];

            if param.is_cell() {
                activation
                    .active_frame_mut()
                    .write_cell_at(param.offset, value);
                continue;
            }

            let bytes = super::frame::encode_argument_bytes(&mut activation, param.ty, value)
                .map_err(RuntimeError::new)?;
            let start = param.offset as usize;
            let end = start + param.byte_len() as usize;
            let destination = activation
                .active_frame_mut()
                .bytes_mut()
                .get_mut(start..end)
                .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
            if destination.len() != bytes.len() {
                return Err(RuntimeError::new(Error::invalid_instruction()));
            }
            destination.copy_from_slice(&bytes);
        }

        activation.run_loop(limits)
    }
}

impl Activation<'_> {
    /// Run the machine loop from the current stack.
    fn run_loop(&mut self, limits: LimitOptions) -> RuntimeResult<Outcome> {
        let program = self.program;
        let mut lowered_instructions_executed = 0;

        // require at least one live frame before stepping
        if self.machine.frames.is_empty() {
            return Err(self.machine.runtime_error(Error::invalid_instruction()));
        }

        loop {
            // enforce the instruction limit before stepping again
            if let Some(max) = limits.max_instructions
                && lowered_instructions_executed >= max
            {
                return Err(self.machine.runtime_error(Error::step_limit_exceeded()));
            }

            // load the current frame position and clear any pending pc
            let (function_id, block_index, start_pc) = {
                let frame = self
                    .machine
                    .frames
                    .last_mut()
                    .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
                let pc = frame.pc;
                frame.pc = 0;
                (frame.function(), frame.block, pc)
            };

            let current_func = program
                .vm_function_by_id(function_id)
                .ok_or_else(|| RuntimeError::new(Error::undefined_function(function_id)))?;

            // run the current lowered block from the chosen instruction offset
            let block_run = {
                let frame_index = self.machine.frames.len() - 1;
                self.bind_frame(frame_index).map_err(RuntimeError::new)?;

                let block_run = if limits.max_instructions.is_some() {
                    dispatch_block_limited(self, current_func, block_index, start_pc)
                } else {
                    dispatch_block(self, current_func, block_index, start_pc)
                };

                (block_run.transfer, block_run.executed)
            };

            // record the instructions covered by this block cache
            if limits.max_instructions.is_some() {
                lowered_instructions_executed += block_run.1;
            }

            let transfer = block_run.0;

            // reload the current function after any direct call path rewrites
            let current_func = {
                let frame = self
                    .machine
                    .frames
                    .last()
                    .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
                let function_id = frame.function();

                program
                    .vm_function_by_id(function_id)
                    .ok_or_else(|| RuntimeError::new(Error::undefined_function(function_id)))?
            };

            // apply the transfer and stop once it produces an outcome
            if let Some(outcome) =
                self.complete_transfer(program, limits, &current_func, transfer)?
            {
                return Ok(outcome);
            }
        }
    }
}
