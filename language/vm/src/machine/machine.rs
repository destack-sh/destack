use std::fmt;
use std::sync::Arc;

use destack_bytecode::CodeOffset;
use destack_memory::MemoryMap;
use destack_program as program;
use destack_program::{
    Continuation, FunctionId, Outcome, Profile, Program, ResumeSkip, StopSet, Value, WatchSet, Word,
};

use crate::diagnostic::{Error, Result};
use crate::options::MachineLimits;

use super::{Activation, Frame, Stack};

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
        })
    }

    /// Fork this machine over one already-forked world memory map.
    pub fn fork(&self, memory: Arc<MemoryMap>) -> Self {
        let stack = self.stack.fork(memory);

        Self {
            program: self.program.clone(),
            limits: self.limits,
            frames: Vec::new(),
            stack,
        }
    }

    /// Execute one linked function.
    pub fn run(
        &mut self,
        activation: program::Activation<'_>,
        function: FunctionId,
        arguments: &[Value],
        stop_points: Option<&StopSet>,
        watch_points: Option<&WatchSet>,
        profile: Option<&mut Profile>,
    ) -> Result<Outcome<Continuation, Value>> {
        let arguments = self.encode_arguments(function, arguments)?;

        // execute encoded arguments without crossing the typed host boundary again
        let outcome = Activation::new(self, activation, stop_points, watch_points, profile, None)
            .run(function, &arguments)?;

        self.decode_outcome(function, outcome)
    }

    /// Resume one suspended continuation.
    pub fn resume(
        &mut self,
        activation: program::Activation<'_>,
        continuation: Continuation,
        received: Value,
        stop_points: Option<&StopSet>,
        watch_points: Option<&WatchSet>,
        profile: Option<&mut Profile>,
    ) -> Result<Outcome<Continuation, Value>> {
        let function = self
            .program
            .continuation_function(&continuation)
            .map_err(Error::program)?;
        let received_type = self
            .program
            .continuation_resume_type(&continuation)
            .map_err(Error::program)?;
        let received = self
            .program
            .encode_value(received_type, &received)
            .map_err(Error::program)?;

        // restore the continuation and deliver its received value
        let outcome = Activation::new(self, activation, stop_points, watch_points, profile, None)
            .resume(continuation, received.as_slice())?;

        self.decode_outcome(function, outcome)
    }

    /// Continue one canonical continuation at its captured program point.
    pub fn continue_execution(
        &mut self,
        activation: program::Activation<'_>,
        continuation: Continuation,
        stop_points: Option<&StopSet>,
        watch_points: Option<&WatchSet>,
        profile: Option<&mut Profile>,
        resume_skip: Option<ResumeSkip>,
    ) -> Result<Outcome<Continuation, Value>> {
        let function = self
            .program
            .continuation_function(&continuation)
            .map_err(Error::program)?;

        // restore the continuation at its captured instruction
        let outcome = Activation::new(
            self,
            activation,
            stop_points,
            watch_points,
            profile,
            resume_skip,
        )
        .continue_execution(continuation)?;

        self.decode_outcome(function, outcome)
    }

    /// Return the immutable Program.
    pub fn program(&self) -> &Program {
        &self.program
    }

    /// Return the machine resource limits.
    pub const fn limits(&self) -> &MachineLimits {
        &self.limits
    }

    /// Encode typed host arguments into function register words.
    fn encode_arguments(&self, function: FunctionId, arguments: &[Value]) -> Result<Vec<Word>> {
        let parameters = self
            .program
            .function_parameters(function)
            .ok_or_else(|| Error::undefined_function(function))?;
        if parameters.len() != arguments.len() {
            return Err(Error::invalid_instruction(function, CodeOffset(0)));
        }

        let mut words = Vec::with_capacity(arguments.len());

        // encode arguments in register order
        for (ty, value) in parameters.iter().copied().zip(arguments) {
            let word = self
                .program
                .encode_value(ty, value)
                .map_err(Error::program)?;
            words.extend(word);
        }

        Ok(words)
    }

    /// Decode raw register words through linked Program types.
    fn decode_outcome(
        &self,
        function: FunctionId,
        outcome: Outcome<Continuation, Vec<Word>>,
    ) -> Result<Outcome<Continuation, Value>> {
        match outcome {
            Outcome::Completed { value } => {
                let ty = self
                    .program
                    .function_result(function)
                    .ok_or_else(|| Error::undefined_function(function))?;
                let value = self
                    .program
                    .decode_value(ty, &value)
                    .map_err(Error::program)?;

                Ok(Outcome::Completed { value })
            }
            Outcome::Yielded {
                continuation,
                value,
            } => {
                let ty = self
                    .program
                    .continuation_yield_type(&continuation)
                    .map_err(Error::program)?;
                let value = self
                    .program
                    .decode_value(ty, &value)
                    .map_err(Error::program)?;

                Ok(Outcome::Yielded {
                    continuation,
                    value,
                })
            }
            Outcome::Stopped {
                continuation,
                reason,
            } => Ok(Outcome::Stopped {
                continuation,
                reason,
            }),
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
