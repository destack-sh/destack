use crate::{CompilerDiagnostic, CompileError, SourceNodeIdAny};

/// Error when evaluating something statically.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum ExecuteError {
    /// Dynamic dependency cannot be statically evaluated.
    DependencyIsUnevaluatable {
        node_id: SourceNodeIdAny,
        depends_on: Vec<SourceNodeIdAny>,
    } = 1,
}

impl ExecuteError {
    /// Get the numeric sub-code of the error.
    #[inline]
    fn sub_code(&self) -> u8 {
        match self {
            Self::DependencyIsUnevaluatable { .. } => 1,
        }
    }
}

impl std::fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecuteError")
            .field("code", &self.full_code())
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

impl CompilerDiagnostic for ExecuteError {
    #[inline]
    fn family_letter(&self) -> &'static str {
        "X"
    }

    #[inline]
    fn family_number(&self) -> u8 {
        4
    }

    #[inline]
    fn sub_code(&self) -> u8 {
        self.sub_code()
    }
}
