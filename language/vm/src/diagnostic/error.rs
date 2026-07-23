use std::{error, fmt};

use destack_bytecode::CodeOffset;
use destack_heap::HeapError;
use destack_program as program;
use destack_program::{BindingId, FunctionId};
use serde::{Deserialize, Serialize};

use super::{
    BindingError, DiagnosticAnchor, ErrorReason, InstructionError, MachineError, Panic,
    ResourceError, StackTraceFrame, Trap,
};

/// One VM execution error with its executable location.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Error {
    /// The exact execution failure.
    pub reason: ErrorReason,
    /// The call stack at the time of failure.
    pub stack: Vec<StackTraceFrame>,
    /// The operation that failed.
    pub anchor: DiagnosticAnchor,
}

/// Result of one VM operation.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Create one unlocated VM execution error.
    pub const fn new(reason: ErrorReason) -> Self {
        Self {
            reason,
            stack: Vec::new(),
            anchor: DiagnosticAnchor::None,
        }
    }

    /// Create one Program operation error.
    pub fn program(error: program::Error) -> Self {
        Self::new(ErrorReason::from(error))
    }

    /// Create one heap operation error.
    pub fn heap(error: HeapError) -> Self {
        Self::new(ErrorReason::from(error))
    }

    /// Create one unavailable runtime binding error.
    pub const fn binding_unavailable(function: FunctionId, binding: BindingId) -> Self {
        Self::new(ErrorReason::Binding(BindingError::Unavailable {
            function,
            binding,
        }))
    }

    /// Create one undefined function error.
    pub fn undefined_function(function: FunctionId) -> Self {
        Self::program(program::Error::undefined_function(function))
    }

    /// Create one invalid instruction error.
    pub const fn invalid_instruction(function: FunctionId, code_offset: CodeOffset) -> Self {
        Self::new(ErrorReason::Instruction(InstructionError::Invalid {
            function,
            code_offset,
        }))
    }

    /// Create one unsupported opcode error.
    pub const fn unsupported_opcode(opcode: u16) -> Self {
        Self::new(ErrorReason::Instruction(
            InstructionError::UnsupportedOpcode { opcode },
        ))
    }

    /// Create one unsupported tensor sharding error.
    pub const fn unsupported_tensor_sharding() -> Self {
        Self::new(ErrorReason::Instruction(
            InstructionError::UnsupportedTensorSharding,
        ))
    }

    /// Create one invalid continuation error.
    pub const fn invalid_continuation() -> Self {
        Self::new(ErrorReason::Machine(MachineError::InvalidContinuation))
    }

    /// Create one invalid destructor error.
    pub const fn invalid_destructor(function: FunctionId) -> Self {
        Self::new(ErrorReason::Instruction(
            InstructionError::InvalidDestructor { function },
        ))
    }

    /// Create one incompatible pointer width error.
    pub const fn incompatible_pointer_width(program: u8, host: u8) -> Self {
        Self::new(ErrorReason::Machine(
            MachineError::IncompatiblePointerWidth { program, host },
        ))
    }

    /// Create one stack overflow error.
    pub const fn stack_overflow() -> Self {
        Self::new(ErrorReason::Resource(ResourceError::StackOverflow))
    }

    /// Create one frame depth error.
    pub const fn frame_limit_exceeded() -> Self {
        Self::new(ErrorReason::Resource(ResourceError::FrameLimitExceeded))
    }

    /// Create one instruction limit error.
    pub const fn instruction_limit_exceeded() -> Self {
        Self::new(ErrorReason::Resource(
            ResourceError::InstructionLimitExceeded,
        ))
    }

    /// Create one world memory exhaustion error.
    pub const fn memory_exhausted() -> Self {
        Self::new(ErrorReason::Resource(ResourceError::MemoryExhausted))
    }

    /// Create one language trap error.
    pub const fn trap(trap: Trap) -> Self {
        Self::new(ErrorReason::Trap(trap))
    }

    /// Create one language panic error.
    pub const fn panic(panic: Panic) -> Self {
        Self::new(ErrorReason::Panic(panic))
    }

    /// Attach one captured call stack.
    pub fn with_stack(mut self, stack: Vec<StackTraceFrame>) -> Self {
        self.stack = stack;

        self
    }

    /// Attach one executable location.
    pub fn with_anchor(mut self, anchor: DiagnosticAnchor) -> Self {
        self.anchor = anchor;

        self
    }
}

impl From<program::Error> for Error {
    /// Preserve one Program operation failure.
    fn from(error: program::Error) -> Self {
        Self::program(error)
    }
}

impl From<HeapError> for Error {
    /// Preserve one heap operation failure.
    fn from(error: HeapError) -> Self {
        Self::heap(error)
    }
}

impl From<ErrorReason> for Error {
    /// Create one unlocated VM execution error.
    fn from(reason: ErrorReason) -> Self {
        Self::new(reason)
    }
}

impl fmt::Display for Error {
    /// Format one VM execution error and its captured stack.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.reason.fmt(formatter)?;
        if self.stack.is_empty() {
            return Ok(());
        }

        // append captured frames from the failure outward
        formatter.write_str("\nStack trace:\n")?;
        for (index, frame) in self.stack.iter().rev().enumerate() {
            let name = frame.function_name.as_deref().unwrap_or("<anonymous>");
            writeln!(
                formatter,
                "  {index}: {name} (byte {})",
                frame.code_offset.0
            )?;
        }

        Ok(())
    }
}

impl error::Error for Error {
    /// Return the exact execution failure.
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(&self.reason)
    }
}
