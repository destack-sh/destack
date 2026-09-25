use std::{error, fmt};

use serde::{Deserialize, Serialize};
use tspp_bytecode as bytecode;
use tspp_heap::HeapError;
use tspp_program as program;
use tspp_program::FunctionId;

use super::{
    DiagnosticAnchor, ErrorReason, InstructionError, MachineError, Panic, ResourceError,
    StackTraceFrame, Trap,
};

/// One VM execution error with its executable location.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Error(Box<Diagnostic>);

/// One complete VM execution diagnostic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Diagnostic {
    /// The exact execution failure.
    reason: ErrorReason,
    /// The call stack at the time of failure.
    stack: Vec<StackTraceFrame>,
    /// The operation that failed.
    anchor: DiagnosticAnchor,
}

/// Result of one VM operation.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Create one unlocated VM execution error.
    pub fn new(reason: ErrorReason) -> Self {
        Self(Box::new(Diagnostic {
            reason,
            stack: Vec::new(),
            anchor: DiagnosticAnchor::None,
        }))
    }

    /// Return the exact execution failure.
    pub fn reason(&self) -> &ErrorReason {
        &self.0.reason
    }

    /// Return the captured call stack.
    pub fn stack(&self) -> &[StackTraceFrame] {
        &self.0.stack
    }

    /// Return the operation that failed.
    pub const fn anchor(&self) -> &DiagnosticAnchor {
        &self.0.anchor
    }

    /// Create one bytecode decoding error.
    pub fn bytecode(error: bytecode::Error) -> Self {
        Self::new(ErrorReason::from(error))
    }

    /// Create one Program operation error.
    pub fn program(error: program::Error) -> Self {
        Self::new(ErrorReason::from(error))
    }

    /// Create one heap operation error.
    pub fn heap(error: HeapError) -> Self {
        Self::new(ErrorReason::from(error))
    }

    /// Create one undefined function error.
    pub fn undefined_function(function: FunctionId) -> Self {
        Self::program(program::Error::undefined_function(function))
    }

    /// Create one invalid instruction error.
    pub fn invalid_instruction() -> Self {
        Self::new(ErrorReason::Instruction(InstructionError::Invalid))
    }

    /// Create one unsupported opcode error.
    pub fn unsupported_opcode(opcode: u16) -> Self {
        Self::new(ErrorReason::Instruction(
            InstructionError::UnsupportedOpcode { opcode },
        ))
    }

    /// Create one invalid machine image error.
    pub fn invalid_image() -> Self {
        Self::new(ErrorReason::Machine(MachineError::InvalidImage))
    }

    /// Create one invalid detach boundary split error.
    pub fn invalid_split() -> Self {
        Self::new(ErrorReason::Machine(MachineError::InvalidSplit))
    }

    /// Create one unavailable bytecode error.
    pub fn bytecode_unavailable() -> Self {
        Self::new(ErrorReason::Machine(MachineError::BytecodeUnavailable))
    }

    /// Create one active execution error.
    pub fn execution_active() -> Self {
        Self::new(ErrorReason::Machine(MachineError::ExecutionActive))
    }

    /// Create one missing stopped execution error.
    pub fn execution_not_stopped() -> Self {
        Self::new(ErrorReason::Machine(MachineError::ExecutionNotStopped))
    }

    /// Create one invalid destructor error.
    pub fn invalid_destructor(function: FunctionId) -> Self {
        Self::new(ErrorReason::Instruction(
            InstructionError::InvalidDestructor { function },
        ))
    }

    /// Create one incompatible pointer width error.
    pub fn incompatible_pointer_width(program: u8, host: u8) -> Self {
        Self::new(ErrorReason::Machine(
            MachineError::IncompatiblePointerWidth { program, host },
        ))
    }

    /// Create one stack overflow error.
    pub fn stack_overflow() -> Self {
        Self::new(ErrorReason::Resource(ResourceError::StackOverflow))
    }

    /// Create one frame depth error.
    pub fn frame_limit_exceeded() -> Self {
        Self::new(ErrorReason::Resource(ResourceError::FrameLimitExceeded))
    }

    /// Create one instruction limit error.
    pub fn instruction_limit_exceeded() -> Self {
        Self::new(ErrorReason::Resource(
            ResourceError::InstructionLimitExceeded,
        ))
    }

    /// Create one world memory exhaustion error.
    pub fn memory_exhausted() -> Self {
        Self::new(ErrorReason::Resource(ResourceError::MemoryExhausted))
    }

    /// Create one language trap error.
    pub fn trap(trap: Trap) -> Self {
        Self::new(ErrorReason::Trap(trap))
    }

    /// Create one language panic error.
    pub fn panic(panic: Panic) -> Self {
        Self::new(ErrorReason::Panic(panic))
    }

    /// Attach one captured call stack.
    pub fn with_stack(mut self, stack: Vec<StackTraceFrame>) -> Self {
        self.0.stack = stack;

        self
    }

    /// Attach one executable location.
    pub fn with_anchor(mut self, anchor: DiagnosticAnchor) -> Self {
        self.0.anchor = anchor;

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

impl From<bytecode::Error> for Error {
    /// Preserve one bytecode decoding failure.
    fn from(error: bytecode::Error) -> Self {
        Self::bytecode(error)
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
        self.0.reason.fmt(formatter)?;
        if self.0.stack.is_empty() {
            return Ok(());
        }

        // append captured frames from the failure outward
        formatter.write_str("\nStack trace:\n")?;
        for (index, frame) in self.0.stack.iter().rev().enumerate() {
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
        Some(&self.0.reason)
    }
}
