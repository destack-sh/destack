use std::fmt;
use std::sync::Arc;

use destack_bytecode::{CodeRange, Function};
use destack_memory::MemoryMap;
use destack_program as program;
use destack_program::{
    Continuation, ContinuationTable, FunctionId, Outcome, Profile, Program, ResumeSkip, Runtime,
    StopSet, SuspensionSite, Value, WatchSet, Word,
};

use crate::diagnostic::{Error, ExecutionError, Result};
use crate::options::MachineLimits;

use super::{Activation, Callee, Frame, Return, Stack};

/// One bytecode machine bound to a Program and world memory.
pub struct Machine {
    /// The immutable linked Program.
    pub(crate) program: Arc<Program>,
    /// The machine resource limits.
    pub(crate) limits: MachineLimits,
    /// The active call frames.
    pub(crate) frames: Vec<Frame>,
    /// The contiguous register stack.
    pub(crate) stack: Stack,
    /// Reusable flattened runtime binding values.
    pub(crate) binding_words: Vec<Word>,
}

impl Machine {
    /// Create one bytecode machine.
    pub fn new(
        program: Arc<Program>,
        memory: Arc<MemoryMap>,
        limits: MachineLimits,
    ) -> Result<Self> {
        let host_pointer_bytes = size_of::<usize>() as u8;
        if program.pointer_bytes() != host_pointer_bytes {
            return Err(Error::incompatible_pointer_width(
                program.pointer_bytes(),
                host_pointer_bytes,
            ));
        }

        // reserve the reusable execution stack
        let stack = Stack::new(memory, limits.stack_bytes)?;

        Ok(Self {
            program,
            limits,
            frames: Vec::new(),
            stack,
            binding_words: Vec::new(),
        })
    }

    /// Fork this machine over one already-forked world memory map.
    pub fn fork(&self, memory: Arc<MemoryMap>) -> Self {
        let stack = self.stack.fork(memory);
        Self {
            program: self.program.clone(),
            limits: self.limits,
            frames: self.frames.clone(),
            stack,
            binding_words: Vec::new(),
        }
    }

    /// Create one empty machine over the same Program and world memory.
    pub fn spawn(&self) -> Result<Self> {
        Self::new(self.program.clone(), self.stack.memory(), self.limits)
    }

    /// Clear retained physical execution state.
    pub fn clear(&mut self) {
        self.frames.clear();
        self.stack.clear();
        self.binding_words.clear();
    }

    /// Execute one linked function.
    pub fn run<'run, R>(
        &mut self,
        continuations: &mut ContinuationTable,
        activation: program::Activation<'run, 'run, R>,
        function: FunctionId,
        environment: Option<&Value>,
        arguments: &[Value],
        stop_points: Option<&'run StopSet>,
        watch_points: Option<&'run WatchSet>,
        profile: Option<&'run mut Profile>,
    ) -> std::result::Result<Outcome<Value>, R::Error>
    where
        R: Runtime + ?Sized,
        R::Error: From<Error>,
    {
        if !self.frames.is_empty() {
            return Err(Error::execution_active().into());
        }
        let arguments = self.encode_arguments(function, environment, arguments)?;

        // execute encoded arguments without crossing the typed host boundary again
        let outcome = Activation::new(self, continuations, activation)
            .instrument(stop_points, watch_points, profile, None)
            .run(function, &arguments)
            .map_err(ExecutionError::into_error)?;

        self.decode_outcome(function, outcome).map_err(Into::into)
    }

    /// Cancel one suspended asynchronous continuation through its cleanup path.
    pub fn cancel<'run, R>(
        &mut self,
        continuations: &mut ContinuationTable,
        activation: program::Activation<'run, 'run, R>,
        continuation: Continuation,
        stop_points: Option<&'run StopSet>,
        watch_points: Option<&'run WatchSet>,
        profile: Option<&'run mut Profile>,
    ) -> std::result::Result<Outcome<Value>, R::Error>
    where
        R: Runtime + ?Sized,
        R::Error: From<Error>,
    {
        if !self.frames.is_empty() {
            return Err(Error::execution_active().into());
        }
        let (function, site) = self.suspension(&continuation)?;
        if site.operation != program::Suspension::Await {
            return Err(Error::invalid_image().into());
        }

        // restore and enter the cancellation successor of the await
        self.restore_continuation_for_cancel(&continuation)?;
        let outcome = Activation::new(self, continuations, activation)
            .instrument(stop_points, watch_points, profile, None)
            .cancel()
            .map_err(ExecutionError::into_error)?;

        self.decode_outcome(function, outcome).map_err(Into::into)
    }

    /// Resume one suspended coroutine with one value.
    pub fn resume<'run, R>(
        &mut self,
        continuations: &mut ContinuationTable,
        activation: program::Activation<'run, 'run, R>,
        continuation: Continuation,
        value: &Value,
        stop_points: Option<&'run StopSet>,
        watch_points: Option<&'run WatchSet>,
        profile: Option<&'run mut Profile>,
    ) -> std::result::Result<Outcome<Value>, R::Error>
    where
        R: Runtime + ?Sized,
        R::Error: From<Error>,
    {
        if !self.frames.is_empty() {
            return Err(Error::execution_active().into());
        }
        let (function, site) = self.suspension(&continuation)?;
        let values = self
            .program
            .value_words(site.resume_type, value)
            .map_err(Error::program)?;

        // restore and enter the normal successor of the suspension operation
        self.restore_continuation(&continuation)?;
        let outcome = Activation::new(self, continuations, activation)
            .instrument(stop_points, watch_points, profile, None)
            .resume(values)
            .map_err(ExecutionError::into_error)?;

        self.decode_outcome(function, outcome).map_err(Into::into)
    }

    /// Complete one suspended generator with one value.
    pub fn complete<'run, R>(
        &mut self,
        continuations: &mut ContinuationTable,
        activation: program::Activation<'run, 'run, R>,
        continuation: Continuation,
        value: &Value,
        stop_points: Option<&'run StopSet>,
        watch_points: Option<&'run WatchSet>,
        profile: Option<&'run mut Profile>,
    ) -> std::result::Result<Outcome<Value>, R::Error>
    where
        R: Runtime + ?Sized,
        R::Error: From<Error>,
    {
        if !self.frames.is_empty() {
            return Err(Error::execution_active().into());
        }
        let (function, site) = self.suspension(&continuation)?;
        if site.operation != program::Suspension::Yield {
            return Err(Error::invalid_image().into());
        }
        let ty = site.complete_type.get().ok_or_else(Error::invalid_image)?;
        let values = self
            .program
            .value_words(ty, value)
            .map_err(Error::program)?;

        // restore and enter the explicit completion successor of the yield
        self.restore_continuation(&continuation)?;
        let outcome = Activation::new(self, continuations, activation)
            .instrument(stop_points, watch_points, profile, None)
            .complete(values)
            .map_err(ExecutionError::into_error)?;

        self.decode_outcome(function, outcome).map_err(Into::into)
    }

    /// Destroy one type-erased runtime value to completion.
    pub fn destroy_value<'run, R>(
        &mut self,
        continuations: &mut ContinuationTable,
        activation: program::Activation<'run, 'run, R>,
        value: Value,
    ) -> std::result::Result<(), R::Error>
    where
        R: Runtime + ?Sized,
        R::Error: From<Error>,
    {
        if !self.frames.is_empty() {
            return Err(Error::execution_active().into());
        }
        let ty = value.ty();
        let Some(function) = self.program.destructor(ty).map_err(Error::program)? else {
            return Ok(());
        };
        let words = self
            .program
            .value_words(ty, &value)
            .map_err(Error::program)?;
        let bytes = Word::bytes(words);
        let layout = self
            .program
            .layout(ty)
            .ok_or_else(|| Error::invalid_destructor(function))?;

        // retain the complete value below its synchronous destructor frame
        let stack_byte_len = self.stack.byte_len();
        let payload = self
            .stack
            .push_bytes(bytes.len(), layout.alignment as usize)?;
        self.stack.write_bytes(payload, bytes)?;

        // enter the generated destructor through its relative reference ABI
        let return_to = Return::Exit {
            completion: program::Completion::Return,
        };
        let frame = self.allocate_frame(function, 1, return_to)?;
        let reference = Word::from_bits((payload + 1) as u64);
        self.stack.write(frame.register_offset, reference);
        self.frames.push(frame);
        let outcome = Activation::new(self, continuations, activation).execute();

        // accept only complete synchronous destructor execution
        let result = match outcome {
            Ok(Outcome::Completed { .. }) => Ok(()),
            Ok(
                Outcome::Cancelled
                | Outcome::Stopped { .. }
                | Outcome::Awaited { .. }
                | Outcome::Yielded { .. },
            ) => Err(Error::invalid_destructor(function).into()),
            Err(error) => Err(error.into_error()),
        };

        // discard invalid retained execution before releasing its traced payload
        self.frames.clear();
        self.stack.truncate(stack_byte_len);

        result
    }

    /// Continue execution retained at one debugger stop.
    pub fn continue_execution<'run, R>(
        &mut self,
        continuations: &mut ContinuationTable,
        activation: program::Activation<'run, 'run, R>,
        stop_points: Option<&'run StopSet>,
        watch_points: Option<&'run WatchSet>,
        profile: Option<&'run mut Profile>,
        resume_skip: Option<ResumeSkip>,
    ) -> std::result::Result<Outcome<Value>, R::Error>
    where
        R: Runtime + ?Sized,
        R::Error: From<Error>,
    {
        let Some(frame) = self.frames.first().copied() else {
            return Err(Error::execution_not_stopped().into());
        };
        let function = frame.function;

        // continue the retained physical frame stack
        let outcome = Activation::new(self, continuations, activation)
            .instrument(stop_points, watch_points, profile, resume_skip)
            .execute()
            .map_err(ExecutionError::into_error)?;

        self.decode_outcome(function, outcome).map_err(Into::into)
    }

    /// Return the immutable Program.
    pub fn program(&self) -> &Program {
        &self.program
    }

    /// Return the machine resource limits.
    pub const fn limits(&self) -> &MachineLimits {
        &self.limits
    }

    /// Return executable code for one bytecode function.
    pub(crate) fn bytecode(&self, function: FunctionId) -> Result<(&Function, CodeRange)> {
        match Callee::resolve(&self.program, function)? {
            Callee::Bytecode { function, code } => Ok((function, code)),
            Callee::Binding(_) => Err(Error::invalid_instruction()),
        }
    }

    /// Allocate one fixed register window and frame descriptor.
    pub(crate) fn allocate_frame(
        &mut self,
        function: FunctionId,
        initialized_word_count: usize,
        return_to: Return,
    ) -> Result<Frame> {
        self.reserve_frame(
            self.frames.len(),
            function,
            initialized_word_count,
            return_to,
        )
    }

    /// Reserve one fixed register window at one post-transition frame depth.
    pub(crate) fn reserve_frame(
        &mut self,
        depth: usize,
        function: FunctionId,
        initialized_word_count: usize,
        return_to: Return,
    ) -> Result<Frame> {
        if depth >= self.limits.max_frames {
            return Err(Error::frame_limit_exceeded());
        }

        // resolve the immutable function body and register window
        let (linked, code) = self.bytecode(function)?;
        let register_count = linked.register_count;
        if initialized_word_count > register_count as usize {
            return Err(Error::invalid_instruction());
        }

        // allocate the function's sole register window
        let register_offset = self.stack.push_words(register_count as usize)?;

        Ok(Frame::new(
            function,
            code,
            register_offset,
            register_count,
            return_to,
        ))
    }

    /// Encode typed host arguments into function register words.
    fn encode_arguments(
        &self,
        function: FunctionId,
        environment: Option<&Value>,
        arguments: &[Value],
    ) -> Result<Vec<Word>> {
        let function_entry = self
            .program
            .function(function)
            .ok_or_else(|| Error::undefined_function(function))?;
        let parameters = self
            .program
            .function_parameters(function)
            .ok_or_else(|| Error::undefined_function(function))?;
        if parameters.len() != arguments.len() {
            return Err(Error::invalid_instruction());
        }

        let word_count = environment.map_or(0, |value| value.words().len())
            + arguments
                .iter()
                .map(|value| value.words().len())
                .sum::<usize>();
        let mut words = Vec::with_capacity(word_count);

        // encode the hidden closure environment before source parameters
        match (function_entry.environment(), environment) {
            (Some(ty), Some(value)) => {
                let environment_words = self
                    .program
                    .value_words(ty, value)
                    .map_err(Error::program)?;
                words.extend_from_slice(environment_words);
            }
            (None, None) => {}
            (Some(_), None) | (None, Some(_)) => {
                return Err(Error::invalid_instruction());
            }
        }

        // encode arguments in register order
        for (ty, value) in parameters.iter().copied().zip(arguments) {
            let value_words = self
                .program
                .value_words(ty, value)
                .map_err(Error::program)?;
            words.extend_from_slice(value_words);
        }

        Ok(words)
    }

    /// Decode raw register words through linked Program types.
    fn decode_outcome(
        &self,
        function: FunctionId,
        outcome: Outcome<Vec<Word>>,
    ) -> Result<Outcome<Value>> {
        match outcome {
            Outcome::Completed { value } => {
                let ty = self
                    .program
                    .function_result(function)
                    .ok_or_else(|| Error::undefined_function(function))?;
                let value = self.program.value(ty, value).map_err(Error::program)?;

                Ok(Outcome::Completed { value })
            }
            Outcome::Cancelled => Ok(Outcome::Cancelled),
            Outcome::Awaited {
                park,
                awaitable,
                continuation,
            } => {
                let (_, site) = self.suspension(&continuation)?;
                let awaitable = self
                    .program
                    .value(site.value_type, awaitable)
                    .map_err(Error::program)?;

                Ok(Outcome::Awaited {
                    park,
                    awaitable,
                    continuation,
                })
            }
            Outcome::Yielded {
                value,
                continuation,
            } => {
                let (_, site) = self.suspension(&continuation)?;
                let value = self
                    .program
                    .value(site.value_type, value)
                    .map_err(Error::program)?;

                Ok(Outcome::Yielded {
                    value,
                    continuation,
                })
            }
            Outcome::Stopped { reason } => Ok(Outcome::Stopped { reason }),
        }
    }

    /// Resolve the linked site captured by one continuation.
    fn suspension(&self, continuation: &Continuation) -> Result<(FunctionId, SuspensionSite)> {
        let root_state = continuation
            .states()
            .first()
            .copied()
            .ok_or_else(Error::invalid_image)?;
        let root = self
            .program
            .frame_state(root_state)
            .ok_or_else(Error::invalid_image)?;
        let suspension_state = continuation.innermost().ok_or_else(Error::invalid_image)?;
        let suspension = self
            .program
            .frame_state(suspension_state)
            .ok_or_else(Error::invalid_image)?;
        let point = suspension
            .point
            .operation_point()
            .ok_or_else(Error::invalid_image)?;
        let (_, site) = self
            .program
            .suspension(point)
            .ok_or_else(Error::invalid_image)?;
        if site.frame_state != suspension_state {
            return Err(Error::invalid_image());
        }

        Ok((root.point.function(), *site))
    }
}

impl fmt::Debug for Machine {
    /// Format one bytecode machine without traversing the Program image.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Machine")
            .field("limits", &self.limits)
            .field("frame_count", &self.frames.len())
            .finish()
    }
}
