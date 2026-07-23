use std::{error, fmt};

use destack_bytecode::CodeOffset;
use destack_program::FunctionId;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// One bytecode instruction execution failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum InstructionError {
    /// The instruction stream is malformed at one function byte offset.
    Invalid {
        /// The containing function.
        function: FunctionId,
        /// The malformed instruction byte offset.
        code_offset: CodeOffset,
    },
    /// The instruction is not implemented by this VM.
    UnsupportedOpcode {
        /// The unsupported opcode value.
        opcode: u16,
    },
    /// The direct CPU tensor engine cannot execute a sharded tensor layout.
    UnsupportedTensorSharding,
    /// A destructor used an instruction forbidden during destruction.
    InvalidDestructor {
        /// The executing destructor.
        function: FunctionId,
    },
}

impl fmt::Display for InstructionError {
    /// Format one bytecode instruction execution failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid {
                function,
                code_offset,
            } => write!(
                formatter,
                "invalid instruction at {function:?}+{}",
                code_offset.0
            ),
            Self::UnsupportedOpcode { opcode } => {
                write!(formatter, "unsupported opcode {opcode}")
            }
            Self::UnsupportedTensorSharding => formatter.write_str("unsupported tensor sharding"),
            Self::InvalidDestructor { function } => {
                write!(formatter, "invalid destructor {function:?}")
            }
        }
    }
}

impl error::Error for InstructionError {}
