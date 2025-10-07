use dyst_source::StringId;

use crate::{Expression, Mutability, Node, NodeId, NodeType, PathId, ScalarLiteral};

/// A Pattern is a pattern to match something and unwrap it.
/// Guards are handled only for match cases (see MatchCase).
///
/// Examples:
/// ```
/// _
/// ..
/// x
/// 1
/// *MyEnum.A
/// 2 | 3
/// 4..6
/// (x, 0, ..)
/// Success(_)
/// Vector2 { x: 0, y, z: zed }
/// geom.Mesh<2, float32> { vertices: [2, ..] }
/// (var x, ..)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Wildcard scalar pattern (`_`).
    Wildcard,
    /// Wildcard rest pattern (`..`).
    Rest,
    /// Maybe pattern (like `T?`).
    Maybe(NodeId<Pattern>),
    /// Reference pattern (like `&x`).
    Reference {
        right: NodeId<Pattern>,
        mutability: Mutability,
    },
    /// Literal value pattern (like `1`).
    ScalarLiteral(ScalarLiteral),
    /// Binding pattern (like `x`).
    Binding { name: StringId },
    /// Path pattern (like `MyEnum.A`).
    Path(PathId),
    /// Range pattern (like `1..3`).
    Range {
        start: Option<NodeId<Pattern>>,
        end: Option<NodeId<Pattern>>,
        is_inclusive: bool,
    },
    /// Tuple pattern (like `(x, 0)` or `Result.Success(_)`).
    Tuple {
        path: Option<PathId>,
        fields: Vec<NodeId<PatternField>>,
    },
    /// Array or slice pattern (like `[1, 2, x]` or `[1, y, ..]`).
    Slice { fields: Vec<NodeId<PatternField>> },
    /// Struct pattern (like `Vector2 { x: 0, y, z: zed  }`).
    Struct {
        ty: NodeId<Expression>,
        fields: Vec<NodeId<PatternField>>,
    },
    /// Union pattern (like `1 | 2 | 3`).
    Union { fields: Vec<NodeId<Pattern>> },
}

impl Node for Pattern {
    const KIND: NodeType = NodeType::Pattern;
}

/// A PatternStructField is a field of a struct pattern.
/// A PatternField is a field in a pattern (tuple, struct, union, etc.).
///
/// Examples:
/// ```
/// x // named
/// x: 4  // named
/// x: y  // named alias  
/// 4     // positional
/// var y // named explicit mutable
/// const z // named explicit immutable
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum PatternField {
    /// Named field, maybe with a pattern (like `x` or `x: 4`).
    Named {
        name: StringId,
        pattern: Option<NodeId<Pattern>>,
        mutability: Option<Mutability>,
    },
    /// Named field with an alias (like `x: y`).
    NamedAlias {
        name: StringId,
        alias: StringId,
        mutability: Option<Mutability>,
    },
    /// Positional field with just a pattern (like `4`).
    Positional { pattern: NodeId<Pattern> },
}

impl Node for PatternField {
    const KIND: NodeType = NodeType::PatternField;
}
