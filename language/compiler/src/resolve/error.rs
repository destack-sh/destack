use crate::CompileError;
use dyst_dir::{Expression, ModuleId, NodeId, NodeIdAny};

/// Error when evaluating something statically.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ResolveError {
    /// Dependent nodes are not ready to be resolved.
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

impl ResolveError {
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
            Self::NotReady { .. } => "dependent nodes are not ready to be resolved",
            Self::CircularDependency { .. } => "circular dependency",
            Self::UnevaluatableExpression { .. } => "unevaluatable expression",
        }
    }
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolveError")
            .field("code", &format!("XE{:03}", self.sub_code()))
            .finish()
    }
}

impl From<ResolveError> for CompileError {
    #[inline]
    fn from(error: ResolveError) -> Self {
        CompileError::Resolve(error)
    }
}

pub type ResolveResult<T> = Result<T, ResolveError>;

