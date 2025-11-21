use dyst_source::StringId;

use crate::{Expression, Mutability, Name, Node, LocalNodeId, NodeType};

/// A Pattern is a pattern to match something and unwrap it.
/// Guards are handled only for match cases (see MatchCase).
///
/// Examples:
/// ```
/// _
/// ..
/// x
/// 1
/// &MyEnum.A
/// 2 | 3
/// 4..6
/// (x, 0, ..)
/// Success(_)
/// Vector2 { x: 0, y, z: zed }
/// geom.Mesh<2, float32> { vertices: [2, ..] }
/// (var x, ..)
/// { a: 2 }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Wildcard scalar pattern (`_`).
    Wildcard,
    /// Wildcard rest pattern (`..` or `..rest`).
    Rest { name: Option<StringId> },
    /// Maybe pattern (like `T?`).
    Maybe(LocalNodeId<Pattern>),
    /// Reference of pattern (like `&x`).
    ReferenceOf {
        mutability: Option<Mutability>,
        right: LocalNodeId<Pattern>,
    },
    /// Binding pattern (basically a PatternField, like `x`, `x: 4`, or `x: int32`).
    Binding {
        mutability: Option<Mutability>,
        name: StringId,
        pattern: Option<LocalNodeId<Pattern>>,
    },
    /// Literal value, type or path pattern (like `4`, `int32`, `Vector2`, `MyEnum.A`).
    Expression { value: LocalNodeId<Expression> },
    /// Range pattern (like `1..3`).
    Range {
        start: Option<LocalNodeId<Pattern>>,
        end: Option<LocalNodeId<Pattern>>,
        is_inclusive: bool,
    },
    /// Tuple pattern (like `(x, 0)` or `Result.Success(_)`).
    Tuple {
        ty: Option<LocalNodeId<Expression>>,
        fields: Vec<LocalNodeId<PatternField>>,
    },
    /// Array or slice pattern (like `[1, 2, x]` or `[1, y, ..]`).
    Slice { fields: Vec<LocalNodeId<PatternField>> },
    /// Struct pattern (like `Vector2 { x: 0, y, z: zed  }` or `{ a: 2 }`).
    Struct {
        ty: Option<LocalNodeId<Expression>>,
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
/// var y // named explicit mutable
/// const z // named explicit immutable
/// x = 4 // named with default
/// x: y = 4 // named with default and alias
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum PatternField {
    /// Named field, maybe with a pattern (like `x` or `x: 4`).
    Named {
        mutability: Option<Mutability>,
        name: Name,
        pattern: Option<LocalNodeId<Pattern>>,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Named field with an alias (like `x: y`).
    Alias {
        mutability: Option<Mutability>,
        name: Name,
        alias: StringId,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Positional field with just a pattern (like `4`).
    Positional { pattern: LocalNodeId<Pattern> },
}

impl Node for PatternField {
    const TYPE: NodeType = NodeType::PatternField;
}
