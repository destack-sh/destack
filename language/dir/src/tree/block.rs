use crate::{Expression, Node, NodeId, NodeType, ScopeId, StringId};

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// The label of the block.
    pub label: Option<StringId>,
    /// The scope of the block.
    pub scope: ScopeId,
    /// The expressions in the block.
    pub expressions: Vec<NodeId<Expression>>,
}

impl Node for Block {
    const TYPE: NodeType = NodeType::Block;

    fn is_resolved(&self) -> bool {
        true
    }
}
