use dyst_dir::{Expression, NodeId, NodeIdAny};

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

/// Warning when optimizing something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum OptimizeWarning {
    /// Unknown type for an expression.
    MissingType { node: NodeId<Expression> } = 1,
}

impl OptimizeWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::MissingType { .. } => 1,
        }
    }

    /// Get the message of the warning.
    pub fn message(&self) -> &'static str {
        match self {
            Self::MissingType { .. } => "unknown type for an expression",
        }
    }
}

impl std::fmt::Display for OptimizeWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OptimizeWarning")
            .field("code", &format!("OW{:03}", self.sub_code()))
            .finish()
    }
}
