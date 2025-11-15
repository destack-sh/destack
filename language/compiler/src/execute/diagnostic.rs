use dyst_dir::{Expression, NodeId, NodeIdAny};

use crate::CompileError;

/// Error when evaluating something statically.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum ExecuteError {
    /// Dynamic dependency cannot be statically evaluated.
    NotExecutable { node: NodeIdAny } = 1,
}

impl ExecuteError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::NotExecutable { .. } => 1,
        }
    }
}

impl std::fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecuteError")
            .field("code", &format!("XE{:03}", self.sub_code()))
            .finish()
    }
}

pub type ExecuteResult<T> = Result<T, ExecuteError>;

impl From<ExecuteError> for CompileError {
    #[inline]
    fn from(error: ExecuteError) -> Self {
        CompileError::Execute(error)
    }
}

/// Warning when executing something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ExecuteWarning {
    /// Complex expression.
    ComplexExpression { node: NodeId<Expression> } = 1,
}

impl ExecuteWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::ComplexExpression { .. } => 1,
        }
    }

    /// Get the message of the warning.
    pub fn message(&self) -> &'static str {
        match self {
            Self::ComplexExpression { .. } => "complex expression",
        }
    }
}
