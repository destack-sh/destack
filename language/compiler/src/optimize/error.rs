use dyst_dir::NodeIdAny;

use crate::CompileError;

/// Error when optimizeing something into the compiler.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum OptimizeError {
    /// Optimization is impossible for this node.
    OptimizationImpossible { node: NodeIdAny } = 1,
}

impl OptimizeError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::OptimizationImpossible { .. } => 1,
        }
    }

    /// Get the message of the error.
    pub fn message(&self) -> &'static str {
        match self {
            Self::OptimizationImpossible { .. } => "optimization is impossible",
        }
    }
}

impl std::fmt::Display for OptimizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OptimizeError")
            .field("code", &format!("OE{:03}", self.sub_code()))
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

