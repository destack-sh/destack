use serde::{Deserialize, Serialize};

use crate::{
    Expression, LocalNodeId, LocalSymbolId, Mutability, Name, Node, NodeType, StringId,
    TypeExpression,
};

/// A Pattern is a pattern to match something and unwrap it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Pattern {
    /// Wildcard scalar pattern (`_`).
    Wildcard,
    /// Must pattern (like `x!`).
    Must(LocalNodeId<Pattern>),
    /// Assignment pattern (like `x = 1` or `{ x } = {}`).
    Assign {
        pattern: LocalNodeId<Pattern>,
        value: LocalNodeId<Expression>,
    },
    /// Reference pattern (like `&x`).
    ReferenceOf {
        mutability: Option<Mutability>,
        right: LocalNodeId<Pattern>,
    },
    /// Value pattern (like `^x`).
    ValueOf {
        mutability: Option<Mutability>,
        right: LocalNodeId<Pattern>,
    },
    /// Binding pattern (like `x`).
    Binding {
        mutability: Option<Mutability>,
        name: StringId,
        pattern: Option<LocalNodeId<Pattern>>,
        symbol: LocalSymbolId,
    },
    /// Literal value, type or path pattern (like `4`, `int32`, `Vector2`, `MyEnum.A`).
    Expression { value: LocalNodeId<Expression> },
    /// Type-space literal or reference pattern.
    TypeExpression { value: LocalNodeId<TypeExpression> },
    /// Anonymous tuple pattern (like `(x, 0)`).
    Tuple {
        fields: Vec<LocalNodeId<PatternField>>,
    },
    /// Tagged tuple pattern (like `Result.Success(_)`).
    TaggedTuple {
        ty: LocalNodeId<TypeExpression>,
        fields: Vec<LocalNodeId<PatternField>>,
    },
    /// Array pattern (like `[1, 2, x]` or `[1, y, ..]`).
    Array {
        fields: Vec<LocalNodeId<PatternField>>,
    },
    /// Anonymous object pattern (like `{ x, y }`).
    Object {
        fields: Vec<LocalNodeId<PatternField>>,
    },
    /// Tagged object pattern (like `Vector2 { x: 0, y }`).
    TaggedObject {
        ty: LocalNodeId<TypeExpression>,
        fields: Vec<LocalNodeId<PatternField>>,
    },
    /// Union pattern (like `1 | 2 | 3`).
    Union { patterns: Vec<LocalNodeId<Pattern>> },
}

impl Node for Pattern {
    const TYPE: NodeType = NodeType::Pattern;
}

impl Pattern {
    /// Get the symbol of the pattern.
    pub fn symbol(&self) -> Option<LocalSymbolId> {
        match self {
            Pattern::Binding { symbol, .. } => Some(*symbol),
            _ => None,
        }
    }
}

/// A PatternField is a field in a pattern (tuple, struct, union, etc.).
/// Field resolution (which struct field it maps to) is in ResolutionTable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatternField {
    /// Named field, maybe shorthand and maybe with a nested pattern.
    Named {
        mutability: Option<Mutability>,
        name: StringId,
        symbol: Option<LocalSymbolId>,
        is_shorthand: bool,
        pattern: Option<LocalNodeId<Pattern>>,
    },
    /// Computed field (like `[key]: value`).
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
    /// Elision (hole) in an array pattern (like `[,a]` or `[,,b]`).
    Elision,
}

impl Node for PatternField {
    const TYPE: NodeType = NodeType::PatternField;
}

impl PatternField {
    /// Get the symbol of the pattern field (the local binding it creates).
    pub fn symbol(&self) -> Option<LocalSymbolId> {
        match self {
            PatternField::Named { symbol, .. } => *symbol,
            PatternField::Computed { .. } => None,
            PatternField::Positional { .. } => None,
            PatternField::Spread { .. } => None,
            PatternField::Elision => None,
        }
    }
}

/// An AssignPattern is one assignment left hand side.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AssignPatternField {
    /// Named field like `{ x }` or `{ x: y }`.
    Named {
        name: Name,
        is_shorthand: bool,
        pattern: Option<LocalNodeId<AssignPattern>>,
    },
    /// Computed field like `{ [key]: value }`.
    Computed {
        key: LocalNodeId<Expression>,
        pattern: LocalNodeId<AssignPattern>,
    },
    /// Positional field like `[value]`.
    Positional { pattern: LocalNodeId<AssignPattern> },
    /// Spread field like `{ ...rest }` or `[...rest]`.
    Spread {
        pattern: Option<LocalNodeId<AssignPattern>>,
    },
    /// Elision like `[, value]`.
    Elision,
}

impl Node for AssignPatternField {
    const TYPE: NodeType = NodeType::AssignPatternField;
}
