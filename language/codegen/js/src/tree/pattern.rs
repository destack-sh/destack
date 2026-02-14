use crate::{Expression, LocalNodeId, Mutability, Node, NodeType, StringId};

/// A Pattern is a pattern to match something and unwrap it.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Binding pattern (like `x`).
    Binding {
        mutability: Option<Mutability>,
        name: StringId,
    },
    /// Array pattern (like `[1, 2, .., x, 3]`).
    Array { elements: Vec<LocalNodeId<Pattern>> },
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
        pattern: Option<LocalNodeId<Pattern>>,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Computed pattern field (like `[key]: value`).
    Computed {
        mutability: Option<Mutability>,
        key: LocalNodeId<Expression>,
        pattern: Option<LocalNodeId<Pattern>>,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Named field with an alias (like `x: y`).
    Alias {
        mutability: Option<Mutability>,
        name: StringId,
        alias: StringId,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Positional field with a pattern and optional default (like `4` or `x = 1`).
    Positional {
        pattern: LocalNodeId<Pattern>,
        default: Option<LocalNodeId<Expression>>,
    },
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
