use std::fmt;

use destack_mir as mir;

/// User trap codes for Cranelift (start at 1, since 0 is invalid for NonZeroU8).
pub mod trap {
    use cranelift_codegen::ir::TrapCode;

    /// Trap for unreachable code.
    pub const UNREACHABLE: TrapCode = TrapCode::unwrap_user(1);

    /// Trap for null pointer dereference.
    pub const NULL_POINTER: TrapCode = TrapCode::unwrap_user(2);

    /// Trap for out of bounds access.
    pub const OUT_OF_BOUNDS: TrapCode = TrapCode::unwrap_user(3);

    /// Trap for integer overflow.
    pub const INTEGER_OVERFLOW: TrapCode = TrapCode::unwrap_user(4);

    /// Trap for division by zero.
    pub const DIVISION_BY_ZERO: TrapCode = TrapCode::unwrap_user(5);
}

/// Errors from Cranelift code generation.
#[derive(Debug, Clone)]
pub enum CraneliftError {
    /// Unsupported target triple.
    UnsupportedTarget { triple: String },

    /// Type not supported by Cranelift.
    UnsupportedType {
        description: String,
        /// The MIR node that caused the error (if available).
        node: Option<mir::LocalNodeIdAny>,
    },

    /// Instruction not yet implemented.
    UnsupportedInstruction {
        description: String,
        /// The MIR instruction that caused the error (if available).
        instruction: Option<mir::LocalNodeId<mir::Instruction>>,
    },

    /// Function not found.
    FunctionNotFound { name: String },

    /// Internal Cranelift error.
    Internal { message: String },
}

impl CraneliftError {
    /// Create an unsupported type error.
    pub fn unsupported_type(description: impl Into<String>) -> Self {
        Self::UnsupportedType {
            description: description.into(),
            node: None,
        }
    }

    /// Create an unsupported type error with a node location.
    pub fn unsupported_type_at(description: impl Into<String>, node: mir::LocalNodeIdAny) -> Self {
        Self::UnsupportedType {
            description: description.into(),
            node: Some(node),
        }
    }

    /// Create an unsupported instruction error.
    pub fn unsupported_instruction(description: impl Into<String>) -> Self {
        Self::UnsupportedInstruction {
            description: description.into(),
            instruction: None,
        }
    }

    /// Create an unsupported instruction error with a node location.
    pub fn unsupported_instruction_at(
        description: impl Into<String>,
        instruction: mir::LocalNodeId<mir::Instruction>,
    ) -> Self {
        Self::UnsupportedInstruction {
            description: description.into(),
            instruction: Some(instruction),
        }
    }
}

impl fmt::Display for CraneliftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedTarget { triple } => {
                write!(f, "unsupported target: {triple}")
            }
            Self::UnsupportedType { description, node } => {
                if let Some(node) = node {
                    write!(f, "unsupported type at {node:?}: {description}")
                } else {
                    write!(f, "unsupported type: {description}")
                }
            }
            Self::UnsupportedInstruction {
                description,
                instruction,
            } => {
                if let Some(inst) = instruction {
                    write!(f, "unsupported instruction at {inst:?}: {description}")
                } else {
                    write!(f, "unsupported instruction: {description}")
                }
            }
            Self::FunctionNotFound { name } => {
                write!(f, "function not found: {name}")
            }
            Self::Internal { message } => {
                write!(f, "internal error: {message}")
            }
        }
    }
}

impl std::error::Error for CraneliftError {}

impl From<cranelift_codegen::CodegenError> for CraneliftError {
    fn from(error: cranelift_codegen::CodegenError) -> Self {
        Self::Internal {
            message: error.to_string(),
        }
    }
}

impl From<cranelift_module::ModuleError> for CraneliftError {
    fn from(error: cranelift_module::ModuleError) -> Self {
        Self::Internal {
            message: error.to_string(),
        }
    }
}
