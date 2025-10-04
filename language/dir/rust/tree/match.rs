use crate::{Expression, Node, NodeId, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub struct MatchCase {
    pub body: NodeId<Expression>,
    pub guard: Option<NodeId<Expression>>,
}

impl Node for MatchCase {
    const KIND: NodeType = NodeType::MatchCase;
}
