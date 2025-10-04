use crate::{Node, NodeId, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub enum Annotation {
    Decorator {},
    Tag {},
    Blank {},
    Doc {},
    Comment {},
}

impl Node for Annotation {
    const KIND: NodeType = NodeType::Annotation;
}
