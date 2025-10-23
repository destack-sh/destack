use crate::{Block, Expression, Node, NodeId, NodeType, ScopedMutability, StringId, Type};

/// A Pattern is a pattern to match something and unwrap it.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Wildcard scalar pattern (`_`).
    Wildcard,
    /// Wildcard rest pattern (`..` or `..rest`).
    Rest {
        name: Option<StringId>,
    },
    /// Maybe pattern (like `T?`).
    Maybe(NodeId<Pattern>),
    /// Reference pattern (like `&x`).
    Reference {
        mutability: Option<ScopedMutability>,
        right: NodeId<Pattern>,
    },
    /// Binding pattern (like `x`).
    Binding {
        mutability: Option<ScopedMutability>,
        name: StringId,
        pattern: Option<NodeId<Pattern>>,
    },
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
        ty: Option<NodeId<Type>>,
        fields: Vec<NodeId<PatternField>>,
    },
    /// Array or slice pattern (like `[1, 2, x]` or `[1, y, ..]`).
    Slice { fields: Vec<NodeId<PatternField>> },
    /// Struct pattern (like `Vector2 { x: 0, y, z: zedso  }`).
    Struct {
        ty: Option<NodeId<Type>>,
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
        mutability: Option<ScopedMutability>,
        name: StringId,
        pattern: Option<NodeId<Pattern>>,
        default: Option<NodeId<Expression>>,
    },
    /// Named field with an alias (like `x: y`).
    NamedAlias {
        mutability: Option<ScopedMutability>,
        name: StringId,
        alias: StringId,
        default: Option<NodeId<Expression>>,
    },
    /// Positional field with just a pattern (like `4` or `int32`).
    Positional { pattern: NodeId<Pattern> },
}

impl Node for PatternField {
    const KIND: NodeType = NodeType::PatternField;
}

/// A MatchSource is where the match was lowered from.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MatchSource {
    /// Match expression (regular match with cases).
    Match,
    /// Explicit try expression or block (`try { ... }` with optional catch).
    Try,
    /// Maybe unary expression (postfix `?`).
    Maybe,
    /// Must unary expression (postfix `!`).
    Must,
}

/// A MatchCase is a match case inside a Match expression.
/// MatchCases can be any Pattern and can have an optional `if` guard.
#[derive(Debug, Clone, PartialEq)]
pub enum MatchCase {
    /// A match case with an expression body.
    Expression {
        pattern: NodeId<Pattern>,
        body: NodeId<Expression>,
        guard: Option<NodeId<Expression>>,
    },
    /// A match case with a block body.
    Block {
        pattern: NodeId<Pattern>,
        body: NodeId<Block>,
        guard: Option<NodeId<Expression>>,
    },
}

impl Node for MatchCase {
    const KIND: NodeType = NodeType::MatchCase;
}
