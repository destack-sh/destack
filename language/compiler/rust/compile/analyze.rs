use dyst_dir::{Annotation, Argument, Expression, NodeId, NodeIdAny, Type};

/// Request to statically evaluate something in-place.
#[derive(Debug, Clone)]
pub enum EvaluateRequest {
    /// Evaluate an Expression fully (in-place).
    EvaluateExpression { expression: NodeId<Expression> },
    /// Evaluate a Type to its Type value (in-place).
    EvaluateType { ty: NodeId<Type> },
    /// Evaluate an Argument (in-place).
    EvaluateArgument { argument: NodeId<Argument> },
    /// Evaluate an Annotation fully (in-place).
    EvaluateAnnotation { annotation: NodeId<Annotation> },
}

/// Error when evaluating something statically.
#[derive(Debug, Clone)]
pub enum EvaluateError {
    NotReady {
        node_id: NodeIdAny,
        depends_on: Option<NodeIdAny>,
    },
}

impl std::fmt::Display for EvaluateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

pub type EvaluateResult<T> = Result<T, EvaluateError>;
