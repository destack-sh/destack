use crate::{Node, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {}

impl Node for Pattern {
    const TYPE: NodeType = NodeType::Pattern;
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternField {}

impl Node for PatternField {
    const TYPE: NodeType = NodeType::PatternField;
}
