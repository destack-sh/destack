use crate::{Expression, Mutability, Node, NodeId, NodeType, StringId};

/// A Pattern is a pattern to match something and unwrap it.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Binding pattern (like `x`).
    Binding {
        mutability: Option<Mutability>,
        name: StringId,
    },
    /// Array pattern (like `[1, 2, .., x, 3]`).
    Array { elements: Vec<NodeId<Pattern>> },
    /// Object pattern (like `{ a: 1, b: 2, ..., x: 3 }`).
    Object { fields: Vec<NodeId<PatternField>> },
    /// Rest pattern (like `...x` or `...rest`).
    Rest { name: Option<StringId> },
    /// Hole pattern (like the empty in `, ,`).
    Hole,
}

impl Node for Pattern {
    const TYPE: NodeType = NodeType::Pattern;
}

/// A PatternField is a field in a pattern (object, array, etc.).
#[derive(Debug, Clone, PartialEq)]
pub enum PatternField {
    /// Named pattern field (like `x` or `x: y` or `x = 4`).
    Named {
        mutability: Option<Mutability>,
        name: StringId,
        alias: Option<StringId>,
        default: Option<NodeId<Expression>>,
    },
    /// Pattern pattern field (like `x: [y, ...]`).
    Pattern {
        mutability: Option<Mutability>,
        pattern: NodeId<Pattern>,
        default: Option<NodeId<Expression>>,
    },
    /// Positional pattern field (like `4`).
    Positional {
        pattern: NodeId<Pattern>,
    },
}

impl Node for PatternField {
    const TYPE: NodeType = NodeType::PatternField;
}
