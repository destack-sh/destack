use destack_source::AdaptImage;
use serde::{Deserialize, Serialize};

use crate::{
    Expression, LocalNodeId, LocalSymbolId, Mutability, Node, NodeType, StringId, TypeExpression,
};

/// A Pattern is a pattern to match something and unwrap it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum Pattern {
    /// Wildcard scalar pattern (`_`).
    Wildcard,
    /// Must pattern (like `x!`).
    Must(LocalNodeId<Pattern>),
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum PatternField {
    /// Named field, maybe with a nested pattern (like `x` or `x: 4` or `x: int32`).
    Named {
        mutability: Option<Mutability>,
        name: StringId,
        pattern: Option<LocalNodeId<Pattern>>,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Computed field (like `[key]: value`).
    Computed {
        mutability: Option<Mutability>,
        key: LocalNodeId<Expression>,
        pattern: Option<LocalNodeId<Pattern>>,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Named field with an alias (like `x: y` where `x` is the field name, `y` is the binding).
    Alias {
        mutability: Option<Mutability>,
        name: StringId,
        alias: StringId,
        default: Option<LocalNodeId<Expression>>,
        symbol: LocalSymbolId,
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
            PatternField::Named { .. } => None,
            PatternField::Computed { .. } => None,
            PatternField::Alias { symbol, .. } => Some(*symbol),
            PatternField::Positional { .. } => None,
            PatternField::Spread { .. } => None,
            PatternField::Elision => None,
        }
    }
}
