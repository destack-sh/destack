use std::fmt;

/// Errors from Cranelift code generation.
#[derive(Debug, Clone)]
pub enum CraneliftError {
    /// Unsupported target triple.
    UnsupportedTarget { triple: String },
    /// Type not supported by Cranelift.
    UnsupportedType { description: String },
    /// Instruction not yet implemented.
    UnsupportedInstruction { description: String },
    /// Function not found.
    FunctionNotFound { name: String },
    /// Internal Cranelift error.
    Internal { message: String },
}

impl fmt::Display for CraneliftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedTarget { triple } => {
                write!(f, "unsupported target: {triple}")
            }
            Self::UnsupportedType { description } => {
                write!(f, "unsupported type: {description}")
            }
            Self::UnsupportedInstruction { description } => {
                write!(f, "unsupported instruction: {description}")
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
