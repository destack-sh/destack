use crate::{Expression, Identifier, LocalNodeId, Node, NodeType, PropertyName};

use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// One JavaScript binding pattern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Pattern {
    /// Binding pattern (like `x`).
    Binding { identifier: Identifier },
    /// Assignment pattern (like `x = 1`).
    Assign {
        pattern: LocalNodeId<Pattern>,
        value: LocalNodeId<Expression>,
    },
    /// Array pattern like `[a, , ...rest]`.
    Array {
        fields: Vec<LocalNodeId<PatternField>>,
    },
    /// Object pattern like `{ a, b: value, ...rest }`.
    Object {
        fields: Vec<LocalNodeId<PatternField>>,
    },
    /// Hole pattern (like the empty in `, ,`).
    Hole,
}

impl Node for Pattern {
    const TYPE: NodeType = NodeType::Pattern;
}

/// One field in a binding pattern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum PatternField {
    /// Named pattern field like `x: y`.
    Named {
        name: PropertyName,
        pattern: LocalNodeId<Pattern>,
    },
    /// Shorthand pattern field like `x` or `x = 4`.
    Shorthand {
        identifier: Identifier,
        value: Option<LocalNodeId<Expression>>,
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

/// One destructuring assignment target.
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

/// One field in a destructuring assignment target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum AssignPatternField {
    /// Named field like `{ x: y }`.
    Named {
        name: PropertyName,
        pattern: LocalNodeId<AssignPattern>,
    },
    /// Shorthand field like `{ x }` or `{ x = 4 }`.
    Shorthand {
        identifier: Identifier,
        value: Option<LocalNodeId<Expression>>,
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
