use crate::{Node, NodeId, NodeType, StringId};

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Identifier { name: StringId },
    Array { elements: Vec<NodeId<Pattern>> },
    Object { fields: Vec<NodeId<PatternField>> },
    Rest,
}

impl Node for Pattern {
    const TYPE: NodeType = NodeType::Pattern;
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternField {}

impl Node for PatternField {
    const TYPE: NodeType = NodeType::PatternField;
}
