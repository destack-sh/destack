use crate::{Expression, LocalNodeId, Name, Node, NodeType, StringId};

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// A Pattern is a pattern to match something and unwrap it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Pattern {
    /// Binding pattern (like `x`).
    Binding { name: StringId },
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum PatternField {
    /// Named pattern field like `x: y`.
    Named {
        name: StringId,
        pattern: LocalNodeId<Pattern>,
    },
    /// Shorthand pattern field like `x` or `x = 4`.
    Shorthand {
        name: StringId,
        value: Option<LocalNodeId<Expression>>,
    },
    /// Computed pattern field (like `[key]: value`).
    Computed {
        key: LocalNodeId<Expression>,
        pattern: LocalNodeId<Pattern>,
    },
    /// Positional field with a pattern (like `4` or `x = 1`).
    Positional { pattern: LocalNodeId<Pattern> },
    /// Spread field like `...x` or `...[a, b]`.
    Spread { pattern: LocalNodeId<Pattern> },
    /// Elision (hole) in an array pattern (like `[,a]`).
    Elision,
}

impl Node for PatternField {
    const TYPE: NodeType = NodeType::PatternField;
}

/// An AssignPattern is one assignment left hand side.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum AssignPattern {
    /// Expression target like `x`, `obj.x`, or `obj[key]`.
    Expression { value: LocalNodeId<Expression> },
    /// Defaulted destructuring target like `x = 1`.
    Assign {
        pattern: LocalNodeId<AssignPattern>,
        value: LocalNodeId<Expression>,
    },
    /// Array destructuring target like `[a, , ...rest]`.
    Array {
        fields: Vec<LocalNodeId<AssignPatternField>>,
    },
    /// Object destructuring target like `{ x, y: z }`.
    Object {
        fields: Vec<LocalNodeId<AssignPatternField>>,
    },
}

impl Node for AssignPattern {
    const TYPE: NodeType = NodeType::AssignPattern;
}

/// An AssignPatternField is one field in a destructuring assignment target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum AssignPatternField {
    /// Named field like `{ x: y }`.
    Named {
        name: Name,
        pattern: LocalNodeId<AssignPattern>,
    },
    /// Shorthand field like `{ x }` or `{ x = 4 }`.
    Shorthand {
        name: Name,
        value: Option<LocalNodeId<Expression>>,
    },
    /// Computed field like `{ [key]: value }`.
    Computed {
        key: LocalNodeId<Expression>,
        pattern: LocalNodeId<AssignPattern>,
    },
    /// Positional field like `[value]`.
    Positional { pattern: LocalNodeId<AssignPattern> },
    /// Spread field like `{ ...rest }` or `[...rest]`.
    Spread { pattern: LocalNodeId<AssignPattern> },
    /// Elision like `[, value]`.
    Elision,
}

impl Node for AssignPatternField {
    const TYPE: NodeType = NodeType::AssignPatternField;
}
