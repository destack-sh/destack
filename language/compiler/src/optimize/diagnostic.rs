use dyst_dir::NodeIdAny;

use crate::{CompilerDiagnostic, CompileError};

/// Error when optimizeing something into the compiler.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum OptimizeError {
    /// Optimization is impossible for this node.
    OptimizationImpossible { node_id: NodeIdAny } = 1,
}

impl OptimizeError {
    /// Get the numeric sub-code of the error.
    #[inline]
    fn sub_code(&self) -> u8 {
        match self {
            Self::OptimizationImpossible { .. } => 1,
        }
    }
}

impl std::fmt::Display for OptimizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OptimizeError")
            .field("code", &self.full_code())
            .finish()
    }
}

pub type OptimizeResult<T> = Result<T, OptimizeError>;

impl From<OptimizeError> for CompileError {
    #[inline]
    fn from(error: OptimizeError) -> Self {
        CompileError::Optimize(error)
    }
}

impl CompilerDiagnostic for OptimizeError {
    #[inline]
    fn family_letter(&self) -> &'static str {
        "O"
    }

    #[inline]
    fn family_number(&self) -> u8 {
        5
    }

    #[inline]
    fn sub_code(&self) -> u8 {
        self.sub_code()
    }
}
