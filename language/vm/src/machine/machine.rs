use std::sync::Arc;
use std::{fmt, process};

use destack_bytecode::{Code, CodeRange, Function};
use destack_memory::MemoryMap;
use destack_mir as mir;
use destack_program as program;
use destack_program::{
    ActivationImage, ContinuationTable, FunctionId, Outcome, Profile, Program, ResumeSkip, Runtime,
    StopSet, Value, WatchSet, Word,
};

use crate::diagnostic::{Error, ExecutionError, Result};
use crate::options::MachineLimits;

use super::{Activation, Callee, Frame, Return, Stack};

/// One bytecode machine bound to a Program and world memory.
pub struct Machine {
    /// The immutable linked Program.
    pub(crate) program: Arc<Program>,
    /// The copied section view of linked bytecode selected for this machine.
    pub(crate) bytecode: Code,
    /// The machine resource limits.
    pub(crate) limits: MachineLimits,
    /// The active call frames.
    pub(crate) frames: Vec<Frame>,
    /// The contiguous register stack.
    pub(crate) stack: Stack,
    /// Canonical execution captured at a runtime or debugger stop.
    pub(crate) activation: Option<ActivationImage>,
    /// Reusable storage for flattened runtime binding calls.
    pub(crate) binding_buffer: Vec<Word>,
}

impl Machine {
    /// Create one bytecode machine.
    pub fn new(
        program: Arc<Program>,
        memory: Arc<MemoryMap>,
        limits: MachineLimits,
    ) -> Result<Self> {
        let bytecode = program
            .bytecode()
            .copied()
            .ok_or_else(Error::bytecode_unavailable)?;
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
            bytecode,
            limits,
            frames: Vec::new(),
            stack,
            activation: None,
            binding_buffer: Vec::new(),
        })
    }

    /// Fork this machine over one already-forked world memory map.
    pub fn fork(&self, memory: Arc<MemoryMap>) -> Self {
        let stack = self.stack.fork(memory);
        Self {
            program: self.program.clone(),
            bytecode: self.bytecode,
            limits: self.limits,
            frames: self.frames.clone(),
            stack,
            activation: self.activation.as_ref().map(ActivationImage::inherit),
            binding_buffer: Vec::new(),
        }
    }

    /// Create one empty machine over the same Program and world memory.
    pub fn spawn(&self) -> Result<Self> {
        Self::new(self.program.clone(), self.stack.memory(), self.limits)
    }

    /// Clear all execution state.
    pub fn clear(&mut self) -> Result<()> {
        let release = self.release_activation();
        self.clear_physical();

        release
    }

    /// Move the captured activation out of this VM.
    pub fn take_activation(&mut self) -> Option<ActivationImage> {
        self.activation.take()
    }

    /// Release only transient physical execution storage.
    pub(crate) fn clear_physical(&mut self) {
        self.frames.clear();
        self.stack.clear();
        self.binding_buffer.clear();
    }

    /// Release the captured activation when present.
    fn release_activation(&mut self) -> Result<()> {
        let Some(image) = self.activation.take() else {
            return Ok(());
        };

        image.release(&self.stack.memory()).map_err(Error::program)
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
        if !self.frames.is_empty() || self.activation.is_some() {
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
        if !self.frames.is_empty() || self.activation.is_some() {
            return Err(Error::execution_active().into());
        }

        let ty = value.ty();
        let Some(function) = self
            .program
            .destructor(ty, mir::Storage::Frame)
            .map_err(Error::program)?
        else {
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
        let payload = self
            .stack
            .push_bytes(bytes.len(), layout.alignment as usize)?;
        self.stack.write_bytes(payload, bytes)?;

        // enter the generated destructor through its relative reference ABI
        let return_to = Return::Exit {
            completion: program::Completion::Return,
        };
        let frame = self.allocate_frame(function, 1, return_to)?;
        let reference = Word::from_bits((payload + 2) as u64);
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

        // discard an invalid captured activation before releasing its traced payload
        self.clear()?;

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
        if self.frames.is_empty() && self.activation.is_some() {
            self.materialize()?;
        }
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
        match Callee::resolve(&self.program, self.bytecode, function)? {
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
    pub(crate) fn decode_outcome(
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
}

impl Drop for Machine {
    /// Release retained canonical execution owned by this machine.
    fn drop(&mut self) {
        if self.release_activation().is_err() {
            process::abort();
        }
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
