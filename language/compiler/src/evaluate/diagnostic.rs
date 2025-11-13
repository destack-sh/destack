use crate::{CompilerDiagnostic, CompileError, SourceNodeIdAny};
use dyst_dir::ModuleId;

/// Error when evaluating something statically.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EvaluateError {
    /// Dependent nodes are not ready to be evaluated.
    NotReady {
        module_id: ModuleId,
        node_id: SourceNodeIdAny,
        depends_on: Vec<SourceNodeIdAny>,
    } = 1,
    /// Circular dependency.
    CircularDependency {
        module_id: ModuleId,
        node_id: SourceNodeIdAny,
        depends_on: Vec<SourceNodeIdAny>,
    } = 2,
}

impl EvaluateError {
    /// Get the numeric sub-code of the error.
    #[inline]
    fn sub_code(&self) -> u8 {
        match self {
            Self::NotReady { .. } => 1,
            Self::CircularDependency { .. } => 2,
        }
    }
}

impl std::fmt::Display for EvaluateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvaluateError")
            .field("code", &self.full_code())
            .finish()
    }
}

pub type EvaluateResult<T> = Result<T, EvaluateError>;

impl From<EvaluateError> for CompileError {
    #[inline]
    fn from(error: EvaluateError) -> Self {
        CompileError::Evaluate(error)
    }
}

impl CompilerDiagnostic for EvaluateError {
    #[inline]
    fn family_letter(&self) -> &'static str {
        "E"
    }

    #[inline]
    fn family_number(&self) -> u8 {
        2
    }

    #[inline]
    fn sub_code(&self) -> u8 {
        self.sub_code()
    }
}
