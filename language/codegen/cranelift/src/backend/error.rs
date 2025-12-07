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
pub enum CodegenCraneliftError {
    /// Unsupported target triple.
    UnsupportedTarget {
        triple: String,
        message: Option<String>,
    },

    /// Type not supported by Cranelift.
    UnsupportedType {
        node: mir::LocalNodeIdAny,
        message: Option<String>,
    },

    /// Missing type for an instruction.
    MissingType {
        node: mir::LocalNodeIdAny,
        message: Option<String>,
    },

    /// Instruction not yet implemented.
    UnsupportedInstruction {
        node: mir::LocalNodeIdAny,
        message: Option<String>,
    },

    /// Function not found.
    FunctionNotFound {
        name: String,
        message: Option<String>,
    },

    /// Internal Cranelift error.
    Internal { message: String },
}

impl CodegenCraneliftError {
    /// Create an unsupported type error with a node location.
    pub fn unsupported_type(message: impl Into<String>, node_id: mir::LocalNodeIdAny) -> Self {
        Self::UnsupportedType {
            node: node_id,
            message: Some(message.into()),
        }
    }

    /// Create an unsupported instruction error with a node location.
    pub fn unsupported_instruction(
        message: impl Into<String>,
        node_id: mir::LocalNodeIdAny,
    ) -> Self {
        Self::UnsupportedInstruction {
            node: node_id,
            message: Some(message.into()),
        }
    }
}

impl fmt::Display for CodegenCraneliftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedTarget { triple, message: _ } => {
                write!(f, "unsupported target: {triple}")
            }
            Self::UnsupportedType { node: _, message } => {
                if let Some(message) = message {
                    write!(f, "unsupported type: {message}")
                } else {
                    write!(f, "unsupported type")
                }
            }
            Self::MissingType { node: _, message } => {
                if let Some(message) = message {
                    write!(f, "missing type: {message}")
                } else {
                    write!(f, "missing type")
                }
            }
            Self::UnsupportedInstruction { node: _, message } => {
                if let Some(message) = message {
                    write!(f, "unsupported instruction: {message}")
                } else {
                    write!(f, "unsupported instruction")
                }
            }
            Self::FunctionNotFound { name, message: _ } => {
                write!(f, "function not found: {name}")
            }
            Self::Internal { message } => {
                write!(f, "internal error: {message}")
            }
        }
    }
}

impl std::error::Error for CodegenCraneliftError {}

impl From<cranelift_codegen::CodegenError> for CodegenCraneliftError {
    fn from(error: cranelift_codegen::CodegenError) -> Self {
        Self::Internal {
            message: error.to_string(),
        }
    }
}

impl From<cranelift_module::ModuleError> for CodegenCraneliftError {
    fn from(error: cranelift_module::ModuleError) -> Self {
        Self::Internal {
            message: error.to_string(),
        }
    }
}

pub type CodegenCraneliftResult<T> = Result<T, CodegenCraneliftError>;