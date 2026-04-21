use crate::{Expression, LocalNodeId, Mutability, Node, NodeType, StringId};

/// A Pattern is a pattern to match something and unwrap it.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Binding pattern (like `x`).
    Binding {
        mutability: Option<Mutability>,
        name: StringId,
    },
    /// Assignment pattern (like `x = 1`).
    Assign {
        pattern: LocalNodeId<Pattern>,
        value: LocalNodeId<Expression>,
    },
    /// Array pattern (like `[1, 2, .., x, 3]`).
    Array {
        fields: Vec<LocalNodeId<PatternField>>,
    },
    /// Object pattern (like `{ a: 1, b: 2, ..., x: 3 }`).
    Object {
        fields: Vec<LocalNodeId<PatternField>>,
    },
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
        is_shorthand: bool,
        pattern: Option<LocalNodeId<Pattern>>,
    },
    /// Computed pattern field (like `[key]: value`).
    Computed {
        mutability: Option<Mutability>,
        key: LocalNodeId<Expression>,
        pattern: LocalNodeId<Pattern>,
    },
    /// Positional field with a pattern (like `4` or `x = 1`).
    Positional { pattern: LocalNodeId<Pattern> },
    /// Spread field (like `...x` or `...[a, b]`).
    Spread {
        mutability: Option<Mutability>,
        pattern: Option<LocalNodeId<Pattern>>,
    },
    /// Elision (hole) in an array pattern (like `[,a]`).
    Elision,
}

impl Node for PatternField {
    const TYPE: NodeType = NodeType::PatternField;
}
