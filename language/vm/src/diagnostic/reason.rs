use std::{error, fmt};

use destack_heap::HeapError;
use destack_program as program;
use serde::{Deserialize, Serialize};

use super::{BindingError, InstructionError, MachineError, Panic, ResourceError, Trap};

/// The exact reason one VM operation failed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ErrorReason {
    /// A Program operation failed.
    Program(Box<program::Error>),
    /// A heap operation failed.
    Heap(Box<HeapError>),
    /// A bytecode instruction could not execute.
    Instruction(InstructionError),
    /// A runtime binding could not execute.
    Binding(BindingError),
    /// VM state or host compatibility prevented execution.
    Machine(MachineError),
    /// Execution reached a language trap.
    Trap(Trap),
    /// Execution terminated with a language panic.
    Panic(Panic),
    /// A configured machine resource was exhausted.
    Resource(ResourceError),
}

impl From<program::Error> for ErrorReason {
    /// Preserve one Program operation failure.
    fn from(error: program::Error) -> Self {
        Self::Program(Box::new(error))
    }
}

impl From<HeapError> for ErrorReason {
    /// Preserve one heap operation failure.
    fn from(error: HeapError) -> Self {
        Self::Heap(Box::new(error))
    }
}

impl fmt::Display for ErrorReason {
    /// Format one VM execution failure reason.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Program(error) => write!(formatter, "program operation failed: {error}"),
            Self::Heap(error) => write!(formatter, "heap operation failed: {error}"),
            Self::Instruction(error) => write!(formatter, "instruction failed: {error}"),
            Self::Binding(error) => write!(formatter, "runtime binding failed: {error}"),
            Self::Machine(error) => write!(formatter, "machine operation failed: {error}"),
            Self::Trap(trap) => write!(formatter, "execution trapped: {trap}"),
            Self::Panic(panic) => write!(formatter, "execution panicked: {panic}"),
            Self::Resource(error) => write!(formatter, "vm resource exhausted: {error}"),
        }
    }
}

impl error::Error for ErrorReason {
    /// Return the specific VM failure.
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Program(error) => Some(error.as_ref()),
            Self::Heap(error) => Some(error.as_ref()),
            Self::Instruction(error) => Some(error),
            Self::Binding(error) => Some(error),
            Self::Machine(error) => Some(error),
            Self::Trap(error) => Some(error),
            Self::Panic(error) => Some(error),
            Self::Resource(error) => Some(error),
        }
    }
}
