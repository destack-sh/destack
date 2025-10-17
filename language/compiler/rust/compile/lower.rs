use dyst_dir::{Expression, NodeId};

/// Request to lower something to DIR/MIR.
#[derive(Debug, Clone)]
pub enum LowerRequest {
    /// Lower an Expression to DIR/MIR.
    LowerExpression { expression: NodeId<Expression> },
}
