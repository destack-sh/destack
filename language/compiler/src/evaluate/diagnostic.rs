use crate::CompileError;
use dyst_dir::{Expression, ModuleId, NodeId, NodeIdAny};

/// Error when evaluating something statically.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum EvaluateError {
    /// Dependent nodes are not ready to be evaluated.
    NotReady {
        module: ModuleId,
        node: NodeIdAny,
        depends_on: Vec<NodeIdAny>,
    } = 1,
    /// Circular dependency.
    CircularDependency {
        module: ModuleId,
        node: NodeIdAny,
        depends_on: Vec<NodeIdAny>,
    } = 2,
    /// Unevaluatable expression.
    UnevaluatableExpression {
        module: ModuleId,
        node: NodeId<Expression>,
    } = 3,
}

impl EvaluateError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::NotReady { .. } => 1,
            Self::CircularDependency { .. } => 2,
            Self::UnevaluatableExpression { .. } => 3,
        }
    }

    /// Get the message of the error.
    pub fn message(&self) -> &'static str {
        match self {
            Self::NotReady { .. } => "dependent nodes are not ready to be evaluated",
            Self::CircularDependency { .. } => "circular dependency",
            Self::UnevaluatableExpression { .. } => "unevaluatable expression",
        }
    }
}

impl std::fmt::Display for EvaluateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvaluateError")
            .field("code", &format!("XE{:03}", self.sub_code()))
            .finish()
    }
}

impl From<EvaluateError> for CompileError {
    #[inline]
    fn from(error: EvaluateError) -> Self {
        CompileError::Evaluate(error)
    }
}

pub type EvaluateResult<T> = Result<T, EvaluateError>;

/// Warning when evaluating something statically.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum EvaluateWarning {
    /// Unknown import.
    UnknownImport {
        module: ModuleId,
        node: NodeId<Expression>,
    } = 1,
}

impl EvaluateWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::UnknownImport { .. } => 1,
        }
    }

    /// Get the message of the warning.
    pub fn message(&self) -> &'static str {
        match self {
            Self::UnknownImport { .. } => "unknown import",
        }
    }
}

impl std::fmt::Display for EvaluateWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvaluateWarning")
            .field("code", &format!("XE{:03}", self.sub_code()))
            .finish()
    }
}
