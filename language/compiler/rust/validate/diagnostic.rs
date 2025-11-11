use dyst_dir::{NodeId, NodeIdAny, Type};

use crate::{CompilerDiagnostic, CompilerError};

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
    fn sub_code(&self) -> u8 {
        match self {
            Self::AssignmentTypeMismatch { .. } => 1,
        }
    }
}

impl std::fmt::Display for ValidateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidateError")
            .field("code", &self.full_code())
            .finish()
    }
}

impl From<ValidateError> for CompilerError {
    #[inline]
    fn from(error: ValidateError) -> Self {
        CompilerError::Validate(error)
    }
}

impl CompilerDiagnostic for ValidateError {
    #[inline]
    fn family_letter(&self) -> &'static str {
        "O"
    }

    #[inline]
    fn family_number(&self) -> u8 {
        3
    }

    #[inline]
    fn sub_code(&self) -> u8 {
        self.sub_code()
    }
}

pub type ValidateResult<T> = Result<T, ValidateError>;
