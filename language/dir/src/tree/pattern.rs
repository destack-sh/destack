use destack_core::StringId;
use serde::{Deserialize, Serialize};

use crate::{Expression, LocalNodeId, Mutability, Name, Node, NodeType, RangeEnd, TypeExpression};

/// A Pattern is a pattern to match something and unwrap it.
/// Guards are handled only for match cases (see MatchCase).
///
/// Examples:
/// ```
/// _
/// ...
/// x
/// 1
/// &MyEnum.A
/// *Point { x, y }
/// 2 | 3
/// (x, 0, ...)
/// Success(_)
/// Vector2 { x: 0, y, z: zed }
/// geom.Mesh<2, float32> { vertices: [2, ...] }
/// { a: 2 }
/// ```
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
    /// Borrow pattern (like `&x`).
    BorrowOf {
        mutability: Option<Mutability>,
        right: LocalNodeId<Pattern>,
    },
    /// Move pattern (like `^x`).
    MoveOf {
        mutability: Option<Mutability>,
        right: LocalNodeId<Pattern>,
    },
    /// Dereference pattern (like `*x`).
    DereferenceOf { right: LocalNodeId<Pattern> },
    /// Binding pattern (basically a PatternField, like `x`, `x: 4`, or `x: int32`).
    Binding {
        name: StringId,
        pattern: Option<LocalNodeId<Pattern>>,
    },
    /// Literal value or value-space path pattern.
    Expression { value: LocalNodeId<Expression> },
    /// Ordered scalar interval pattern like `0..10` or `..=255`.
    Range {
        start: Option<LocalNodeId<Expression>>,
        end: Option<LocalNodeId<Expression>>,
        end_kind: RangeEnd,
    },
    /// Type-space literal or reference pattern.
    TypeExpression { value: LocalNodeId<TypeExpression> },
    /// Tuple pattern (like `(x, 0)`).
    Tuple {
        fields: Vec<LocalNodeId<PatternField>>,
    },
    /// Newtype wrapper pattern (like `T(1)`).
    Newtype {
        ty: LocalNodeId<TypeExpression>,
        fields: Vec<LocalNodeId<PatternField>>,
    },
    /// Sequence pattern like `[1, 2, x]` or `[1, y, ..]`.
    Sequence {
        fields: Vec<LocalNodeId<PatternField>>,
    },
    /// Object pattern (like `{ x, y }`).
    Object {
        fields: Vec<LocalNodeId<PatternField>>,
    },
    /// Nominal object pattern (like `T { x, y }`).
    NominalObject {
        ty: LocalNodeId<TypeExpression>,
        fields: Vec<LocalNodeId<PatternField>>,
    },
    /// Union pattern (like `1 | 2 | 3`).
    Union { patterns: Vec<LocalNodeId<Pattern>> },
}

impl Node for Pattern {
    const TYPE: NodeType = NodeType::Pattern;
}

/// A PatternField is a field of a variant pattern.
///
/// Examples:
/// ```
/// x // named
/// x: 4  // named
/// x: int32 // named
/// x: y  // named alias
/// 4     // positional
/// x = 4 // named with default
/// x: y = 4 // named with default and alias
/// ... // spread
/// ...rest // spread with name
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatternField {
    /// Named field, maybe shorthand and maybe with a nested pattern.
    Named {
        name: Name,
        pattern: Option<LocalNodeId<Pattern>>,
        is_shorthand: bool,
    },
    /// Computed field (like `[key]: value`).
    Computed {
        key: LocalNodeId<Expression>,
        pattern: LocalNodeId<Pattern>,
    },
    /// Positional field with a pattern (like `4` or `x = 1`).
    Positional { pattern: LocalNodeId<Pattern> },
    /// Spread field (like `...x` or `...[a, b]`).
    Spread {
        pattern: Option<LocalNodeId<Pattern>>,
    },
    /// Elision in a sequence pattern like `[,a]` or `[,,b]`.
    Elision,
}

impl Node for PatternField {
    const TYPE: NodeType = NodeType::PatternField;
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
    /// Sequence destructuring target like `[a, , ...rest]`.
    Sequence {
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
        pattern: Option<LocalNodeId<AssignPattern>>,
        is_shorthand: bool,
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
