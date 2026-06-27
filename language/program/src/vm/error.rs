use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::FunctionId;

/// VM program record result.
pub type Result<T> = std::result::Result<T, Error>;

/// VM program record error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Error {
    /// Type mismatch while building or decoding a VM program.
    TypeMismatch { expected: String, actual: String },
    /// Invalid VM instruction encoding.
    InvalidInstruction,
    /// Invalid VM cast encoding.
    InvalidCast,
    /// Invalid VM field access encoding.
    InvalidFieldAccess { index: u32, field_count: usize },
    /// Invalid VM pointer type.
    InvalidPointerType { actual: String },
    /// Unsupported VM instruction form.
    UnsupportedInstruction { name: String },
    /// Unsupported zero initializer.
    UnsupportedZeroValue { ty: String },
    /// Invalid VM program record.
    InvalidProgram { context: String },
    /// Internal VM program invariant failure.
    Internal { context: String },
    /// Undefined lowered function.
    UndefinedFunction {
        /// The missing function id.
        function: FunctionId,
    },
}

impl Error {
    /// Return one type mismatch error.
    pub fn type_mismatch(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::TypeMismatch {
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Return one invalid instruction error.
    pub const fn invalid_instruction() -> Self {
        Self::InvalidInstruction
    }

    /// Return one invalid cast error.
    pub const fn invalid_cast() -> Self {
        Self::InvalidCast
    }

    /// Return one invalid field access error.
    pub const fn invalid_field_access(index: u32, field_count: usize) -> Self {
        Self::InvalidFieldAccess { index, field_count }
    }

    /// Return one invalid pointer type error.
    pub fn invalid_pointer_type(actual: impl Into<String>) -> Self {
        Self::InvalidPointerType {
            actual: actual.into(),
        }
    }

    /// Return one unsupported instruction error.
    pub fn unsupported_instruction(name: impl Into<String>) -> Self {
        Self::UnsupportedInstruction { name: name.into() }
    }

    /// Return one unsupported zero initializer error.
    pub fn unsupported_zero_value(ty: impl Into<String>) -> Self {
        Self::UnsupportedZeroValue { ty: ty.into() }
    }

    /// Return one invalid program error.
    pub fn invalid_program(context: impl Into<String>) -> Self {
        Self::InvalidProgram {
            context: context.into(),
        }
    }

    /// Return one internal error.
    pub fn internal(context: impl Into<String>) -> Self {
        Self::Internal {
            context: context.into(),
        }
    }

    /// Return one undefined function error.
    pub fn undefined_function(function: FunctionId) -> Self {
        Self::UndefinedFunction { function }
    }
}
