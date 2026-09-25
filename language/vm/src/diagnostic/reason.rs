use std::{error, fmt};

use serde::{Deserialize, Serialize};
use tspp_bytecode as bytecode;
use tspp_heap::HeapError;
use tspp_program as program;

use super::{InstructionError, MachineError, Panic, ResourceError, Trap};

/// The exact reason one VM operation failed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ErrorReason {
    /// A bytecode stream could not be decoded.
    Bytecode(Box<bytecode::Error>),
    /// A Program operation failed.
    Program(Box<program::Error>),
    /// A heap operation failed.
    Heap(Box<HeapError>),
    /// A bytecode instruction could not execute.
    Instruction(InstructionError),
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

impl From<bytecode::Error> for ErrorReason {
    /// Preserve one bytecode decoding failure.
    fn from(error: bytecode::Error) -> Self {
        Self::Bytecode(Box::new(error))
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
            Self::Bytecode(error) => write!(formatter, "bytecode decoding failed: {error}"),
            Self::Program(error) => write!(formatter, "program operation failed: {error}"),
            Self::Heap(error) => write!(formatter, "heap operation failed: {error}"),
            Self::Instruction(error) => write!(formatter, "instruction failed: {error}"),
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
            Self::Bytecode(error) => Some(error.as_ref()),
            Self::Program(error) => Some(error.as_ref()),
            Self::Heap(error) => Some(error.as_ref()),
            Self::Instruction(error) => Some(error),
            Self::Machine(error) => Some(error),
            Self::Trap(error) => Some(error),
            Self::Panic(error) => Some(error),
            Self::Resource(error) => Some(error),
        }
    }
}
