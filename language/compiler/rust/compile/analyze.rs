use dyst_dir::{Annotation, Argument, Expression, NodeId, NodeIdAny, Type};

/// Request to statically resolve something in-place.
#[derive(Debug, Clone)]
pub enum ResolveRequest {
    /// Resolve an Expression fully (in-place).
    ResolveExpression { expression: NodeId<Expression> },
    /// Resolve a Type to its Type value (in-place).
    ResolveType { ty: NodeId<Type> },
    /// Resolve an Argument (in-place).
    ResolveArgument { argument: NodeId<Argument> },
    /// Resolve an Annotation fully (in-place).
    ResolveAnnotation { annotation: NodeId<Annotation> },
}

/// Error when evaluating something statically.
#[derive(Debug, Clone)]
pub enum ResolveError {
    NotReady {
        node_id: NodeIdAny,
        depends_on: Option<NodeIdAny>,
    },
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

pub type ResolveResult<T> = Result<T, ResolveError>;
