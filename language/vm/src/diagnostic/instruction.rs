use std::{error, fmt};

use serde::{Deserialize, Serialize};
use tspp_program::FunctionId;
use tspp_serde::Reflect;

/// One bytecode instruction execution failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum InstructionError {
    /// The instruction stream or its operands are malformed.
    Invalid,
    /// The instruction is not implemented by this VM.
    UnsupportedOpcode {
        /// The unsupported opcode value.
        opcode: u16,
    },
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
            Self::Invalid => formatter.write_str("invalid instruction"),
            Self::UnsupportedOpcode { opcode } => {
                write!(formatter, "unsupported opcode {opcode}")
            }
            Self::InvalidDestructor { function } => {
                write!(formatter, "invalid destructor {function:?}")
            }
        }
    }
}

impl error::Error for InstructionError {}
