use dyst_dir::{Expression, NodeId};

/// Request to execute something at compile time.
#[derive(Debug, Clone)]
pub enum ExecuteRequest {
    /// Execute an Expression fully (in-place).
    ExecuteExpression { expression: NodeId<Expression> },
}
