use dyst_dir::{NodeId, NodeIdAny, Type};

use crate::CompileError;

/// Error when validateing something into the compiler.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum ValidateError {
    /// Assignment type mismatch.
    AssignmentTypeMismatch {
        left: NodeIdAny,
        right: NodeIdAny,
        required_ty: NodeId<Type>,
        actual_ty: NodeId<Type>,
    } = 1,
}

impl ValidateError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::AssignmentTypeMismatch { .. } => 1,
        }
    }

    /// Get the message of the error.
    pub fn message(&self) -> &'static str {
        match self {
            Self::AssignmentTypeMismatch { .. } => "assignment type mismatch",
        }
    }
}

impl std::fmt::Display for ValidateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidateError")
            .field("code", &format!("VE{:03}", self.sub_code()))
            .finish()
    }
}

impl From<ValidateError> for CompileError {
    #[inline]
    fn from(error: ValidateError) -> Self {
        CompileError::Validate(error)
    }
}

pub type ValidateResult<T> = Result<T, ValidateError>;
