use crate::{Expression, Mutability, Node, NodeId, NodeType, StringId};

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Binding {
        mutability: Option<Mutability>,
        name: StringId,
    },
    Array {
        elements: Vec<NodeId<Pattern>>,
    },
    Object {
        fields: Vec<NodeId<PatternField>>,
    },
    Rest {
        name: Option<StringId>,
    },
    Hole,
}

impl Node for Pattern {
    const TYPE: NodeType = NodeType::Pattern;
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternField {
    Named {
        mutability: Option<Mutability>,
        name: StringId,
        pattern: NodeId<Pattern>,
        default: Option<NodeId<Expression>>,
    },
    Alias {
        mutability: Option<Mutability>,
        name: StringId,
        alias: StringId,
        default: Option<NodeId<Expression>>,
    },
    Positional {
        pattern: NodeId<Pattern>,
        default: Option<NodeId<Expression>>,
    },
}

impl Node for PatternField {
    const TYPE: NodeType = NodeType::PatternField;
}
