use std::fmt;
use std::sync::Arc;

use tspp_bytecode::{Code, CodeRange, Function};
use tspp_memory::MemoryMap;
use tspp_mir as mir;
use tspp_program as program;
use tspp_program::{
    FunctionId, Outcome, Profile, Program, ResumeSkip, Runtime, StopSet, Value, WatchSet, Word,
};

use crate::diagnostic::{Error, ExecutionError, Result};
use crate::options::MachineLimits;

use super::{Activation, Callee, Fiber, Frame, Return};

/// One bytecode machine bound to a Program, executing over borrowed fibers.
pub struct Machine {
    /// The immutable linked Program.
    pub(crate) program: Arc<Program>,
    /// The copied section view of linked bytecode selected for this machine.
    pub(crate) bytecode: Code,
    /// The machine resource limits.
    pub(crate) limits: MachineLimits,
    /// Reusable storage for flattened runtime binding calls.
    pub(crate) binding_buffer: Vec<Word>,
}

impl Machine {
    /// Create one bytecode machine.
    pub fn new(program: Arc<Program>, limits: MachineLimits) -> Result<Self> {
        let bytecode = *program.bytecode();
        let host_pointer_bytes = size_of::<usize>() as u8;
        if program.pointer_bytes() != host_pointer_bytes {
            return Err(Error::incompatible_pointer_width(
                program.pointer_bytes(),
                host_pointer_bytes,
            ));
        }

        Ok(Self {
            program,
            bytecode,
            limits,
            binding_buffer: Vec::new(),
        })
    }

    /// Reserve one idle fiber sized by this machine's limits.
    pub fn reserve_fiber(&self, memory: Arc<MemoryMap>) -> Result<Fiber> {
        Fiber::new(memory, self.limits.stack_bytes)
    }

    /// Fork this machine for one forked world.
    pub fn fork(&self) -> Self {
        Self {
            program: self.program.clone(),
            bytecode: self.bytecode,
            limits: self.limits,
            binding_buffer: Vec::new(),
        }
    }

    /// Execute one linked function on an idle fiber.
    pub fn run<'run, R>(
        &mut self,
        fiber: &mut Fiber,
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
        if !fiber.is_idle() {
            return Err(Error::execution_active().into());
        }

        let arguments = self.encode_arguments(function, environment, arguments)?;

        // execute encoded arguments without crossing the typed host boundary again
        let outcome = Activation::new(self, fiber, activation)
            .instrument(stop_points, watch_points, profile, None)
            .run(function, &arguments)
            .map_err(ExecutionError::into_error);

        self.settle(fiber, function, outcome)
    }

    /// Decode one outcome and release the fiber at terminal outcomes.
    fn settle<E: From<Error>>(
        &mut self,
        fiber: &mut Fiber,
        function: FunctionId,
        outcome: std::result::Result<Outcome<Vec<Word>>, E>,
    ) -> std::result::Result<Outcome<Value>, E> {
        self.binding_buffer.clear();
        let outcome = match outcome {
            Ok(outcome) => self.decode_outcome(function, outcome).map_err(E::from),
            Err(error) => Err(error),
        };

        // keep parked and stopped fibers intact for later resumption
        match &outcome {
            Ok(Outcome::Parked | Outcome::Stopped { .. }) => {}
            Ok(Outcome::Completed { .. } | Outcome::Cancelled) | Err(_) => fiber.clear(),
        }

        outcome
    }

    /// Resume one parked fiber with a delivered wake value.
    pub fn resume<'run, R>(
        &mut self,
        fiber: &mut Fiber,
        activation: program::Activation<'run, 'run, R>,
        value: &Value,
        stop_points: Option<&'run StopSet>,
        watch_points: Option<&'run WatchSet>,
        profile: Option<&'run mut Profile>,
    ) -> std::result::Result<Outcome<Value>, R::Error>
    where
        R: Runtime + ?Sized,
        R::Error: From<Error>,
    {
        let Some(frame) = fiber.frames.first().copied() else {
            return Err(Error::execution_not_stopped().into());
        };
        let function = frame.function;
        self.validate_frames(fiber)?;

        // deliver the wake value into the parked call's result registers
        let Some(wake_to) = fiber.wake_to.take() else {
            return Err(Error::execution_not_stopped().into());
        };
        self.deliver_wake(fiber, wake_to, value)?;

        // continue the retained physical frame stack
        let outcome = Activation::new(self, fiber, activation)
            .instrument(stop_points, watch_points, profile, None)
            .execute()
            .map_err(ExecutionError::into_error);

        self.settle(fiber, function, outcome)
    }

    /// Destroy one type-erased runtime value to completion.
    pub fn destroy_value<'run, R>(
        &mut self,
        fiber: &mut Fiber,
        activation: program::Activation<'run, 'run, R>,
        value: Value,
    ) -> std::result::Result<(), R::Error>
    where
        R: Runtime + ?Sized,
        R::Error: From<Error>,
    {
        if !fiber.is_idle() {
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
        let payload = fiber
            .stack
            .push_bytes(bytes.len(), layout.alignment as usize)?;
        fiber.stack.write_bytes(payload, bytes)?;

        // enter the generated destructor through its relative reference ABI
        let return_to = Return::Exit {
            completion: program::Completion::Return,
        };
        let frame = self.allocate_frame(fiber, function, 1, return_to)?;
        let reference = fiber.stack.memory_offset(payload);
        let reference = Word::from_bits(reference as u64);
        fiber.stack.write(frame.register_offset, reference);
        fiber.frames.push(frame);
        let outcome = Activation::new(self, fiber, activation).execute();

        // accept only complete synchronous destructor execution
        let result = match outcome {
            Ok(Outcome::Completed { .. }) => Ok(()),
            Ok(Outcome::Cancelled | Outcome::Stopped { .. } | Outcome::Parked) => {
                Err(Error::invalid_destructor(function).into())
            }
            Err(error) => Err(error.into_error()),
        };
        fiber.clear();

        result
    }

    /// Continue execution retained at one debugger stop.
    pub fn continue_execution<'run, R>(
        &mut self,
        fiber: &mut Fiber,
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
        let Some(frame) = fiber.frames.first().copied() else {
            return Err(Error::execution_not_stopped().into());
        };
        let function = frame.function;
        self.validate_frames(fiber)?;

        // continue the retained physical frame stack
        let outcome = Activation::new(self, fiber, activation)
            .instrument(stop_points, watch_points, profile, resume_skip)
            .execute()
            .map_err(ExecutionError::into_error);

        self.settle(fiber, function, outcome)
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

    /// Allocate one fixed register window and frame descriptor on a fiber.
    pub(crate) fn allocate_frame(
        &mut self,
        fiber: &mut Fiber,
        function: FunctionId,
        initialized_word_count: usize,
        return_to: Return,
    ) -> Result<Frame> {
        let depth = fiber.frames.len();

        self.reserve_frame(fiber, depth, function, initialized_word_count, return_to)
    }

    /// Reserve one fixed register window at one post-transition frame depth.
    pub(crate) fn reserve_frame(
        &mut self,
        fiber: &mut Fiber,
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
        let register_offset = fiber.stack.push_words(register_count as usize)?;

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

    /// Require every retained frame to match its linked bytecode function.
    fn validate_frames(&self, fiber: &Fiber) -> Result<()> {
        let sections = self.program.sections();
        for frame in &fiber.frames {
            let linked = self
                .bytecode
                .function(sections, frame.function.index())
                .ok_or_else(Error::invalid_image)?;
            let code = linked.code().ok_or_else(Error::invalid_image)?;
            if frame.code != code
                || frame.register_count as usize != linked.register_count()
                || frame.pc.0 >= code.byte_len
            {
                return Err(Error::invalid_image());
            }
        }

        Ok(())
    }

    /// Write one wake value into a parked call's result registers.
    fn deliver_wake(
        &self,
        fiber: &mut Fiber,
        wake_to: tspp_bytecode::RegisterSpan,
        value: &Value,
    ) -> Result<()> {
        let words = value.words();
        if words.len() != wake_to.word_count as usize {
            return Err(Error::invalid_instruction());
        }
        let Some(frame) = fiber.frames.last() else {
            return Err(Error::execution_not_stopped());
        };

        // wake registers never leave the parked frame's register window
        let end = wake_to.start.0 as usize + words.len();
        if end > frame.register_count as usize {
            return Err(Error::invalid_image());
        }
        let byte_end = (frame.register_offset + end) * Word::BYTE_LEN;
        if byte_end > fiber.stack.byte_len() {
            return Err(Error::invalid_image());
        }

        // write wake words into the parked frame's register window
        let offset = frame.register_offset + wake_to.start.0 as usize;
        for (index, word) in words.iter().enumerate() {
            fiber.stack.write(offset + index, *word);
        }

        Ok(())
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
            Outcome::Parked => Ok(Outcome::Parked),
            Outcome::Stopped { reason } => Ok(Outcome::Stopped { reason }),
        }
    }
}

impl fmt::Debug for Machine {
    /// Format one bytecode machine without traversing the Program image.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Machine")
            .field("limits", &self.limits)
            .finish()
    }
}
