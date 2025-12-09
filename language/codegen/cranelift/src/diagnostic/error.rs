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

/// Error during Cranelift code generation.
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
    /// Out of bounds access (tuple/array element index).
    OutOfBounds {
        node: mir::LocalNodeIdAny,
        index: u32,
        len: usize,
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

    /// Create an out of bounds error.
    pub fn out_of_bounds(node_id: mir::LocalNodeIdAny, index: u32, len: usize) -> Self {
        Self::OutOfBounds {
            node: node_id,
            index,
            len,
        }
    }
}

impl fmt::Display for CodegenCraneliftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedTarget { triple, message } => {
                write!(f, "unsupported target: {triple}")?;
                if let Some(msg) = message {
                    write!(f, " ({msg})")?;
                }
                Ok(())
            }
            Self::UnsupportedType { message, .. } => {
                if let Some(message) = message {
                    write!(f, "unsupported type: {message}")
                } else {
                    write!(f, "unsupported type")
                }
            }
            Self::MissingType { message, .. } => {
                if let Some(message) = message {
                    write!(f, "missing type: {message}")
                } else {
                    write!(f, "missing type")
                }
            }
            Self::UnsupportedInstruction { message, .. } => {
                if let Some(message) = message {
                    write!(f, "unsupported instruction: {message}")
                } else {
                    write!(f, "unsupported instruction")
                }
            }
            Self::FunctionNotFound { name, message } => {
                write!(f, "function not found: {name}")?;
                if let Some(msg) = message {
                    write!(f, " ({msg})")?;
                }
                Ok(())
            }
            Self::OutOfBounds { index, len, .. } => {
                write!(f, "index {index} out of bounds (len {len})")
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

/// Result type for Cranelift codegen operations.
pub type CodegenCraneliftResult<T> = Result<T, CodegenCraneliftError>;
