use crate::{Node, NodeId, NodeType};

/// A Trait is trait definition node defining behavior and constants.
#[derive(Debug, Clone, PartialEq)]
pub struct Trait {}

impl Node for Trait {
    const KIND: NodeType = NodeType::Trait;
}
