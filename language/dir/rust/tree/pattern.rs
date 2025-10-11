use crate::{Expression, Mutability, Node, NodeId, NodeType, Path, StringId};

/// A Pattern is a pattern to match something and unwrap it.
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
    /// Binding pattern (like `x`).
    Binding { name: StringId, pattern: Option<NodeId<Pattern>> },
    /// Literal value, type or path pattern (like `4`, `int32`, `Vector2`, `MyEnum.A`).
    Expression { value: NodeId<Expression> },
    /// Range pattern (like `1..3`).
    Range {
        start: Option<NodeId<Pattern>>,
        end: Option<NodeId<Pattern>>,
        is_inclusive: bool,
    },
    /// Tuple pattern (like `(x, 0)` or `Result.Success(_)`).
    Tuple {
        path: Option<Path>,
        fields: Vec<NodeId<PatternField>>,
    },
    /// Array or slice pattern (like `[1, 2, x]` or `[1, y, ..]`).
    Slice { fields: Vec<NodeId<PatternField>> },
    /// Struct pattern (like `Vector2 { x: 0, y, z: zedso  }`).
    Struct {
        ty: Option<NodeId<Expression>>,
        fields: Vec<NodeId<PatternField>>,
    },
    /// Union pattern (like `1 | 2 | 3`).
    Union { patterns: Vec<NodeId<Pattern>> },
}

impl Node for Pattern {
    const KIND: NodeType = NodeType::Pattern;
}

/// A PatternField is a field in a pattern (tuple, struct, union, etc.).
#[derive(Debug, Clone, PartialEq)]
pub enum PatternField {
    /// Named field, maybe with a pattern (like `x` or `x: 4` or `x: int32`).
    Named {
        mutability: Option<Mutability>,
        name: StringId,
        pattern: Option<NodeId<Pattern>>,
    },
    /// Named field with an alias (like `x: y`).
    NamedAlias {
        mutability: Option<Mutability>,
        name: StringId,
        alias: StringId,
    },
    /// Positional field with just a pattern (like `4` or `int32`).
    Positional { pattern: NodeId<Pattern> },
}

impl Node for PatternField {
    const KIND: NodeType = NodeType::PatternField;
}
