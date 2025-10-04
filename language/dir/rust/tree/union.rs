use crate::{Node, NodeId, NodeType, Variant};

#[derive(Debug, Clone, PartialEq)]
pub struct Union {
    variants: Vec<NodeId<Variant>>,
}

impl Node for Union {
    const KIND: NodeType = NodeType::Union;
}
 