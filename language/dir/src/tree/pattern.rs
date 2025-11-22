use crate::{Expression, GlobalSymbolId, LocalNodeId, Mutability, Node, NodeType, StringId};

/// A Pattern is a pattern to match something and unwrap it.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Wildcard scalar pattern (`_`).
    Wildcard,
    /// Wildcard rest pattern (`..` or `..rest`).
    Rest { name: Option<StringId> },
    /// Maybe pattern (like `T?`).
    Maybe(LocalNodeId<Pattern>),
    /// Reference pattern (like `&x`).
    Reference {
        mutability: Option<Mutability>,
        right: LocalNodeId<Pattern>,
    },
    /// Binding pattern (like `x`).
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
    /// Unresolved tuple pattern (like `Result.Success(_)`).
    UnresolvedTuple {
        ty: LocalNodeId<Expression>,
        fields: Vec<LocalNodeId<PatternField>>,
    },
    /// Tuple pattern (like `(x, 0)` or `Result.Success(_)`).
    Tuple {
        remote_symbol: GlobalSymbolId,
        fields: Vec<LocalNodeId<PatternField>>,
    },
    /// Array or slice pattern (like `[1, 2, x]` or `[1, y, ..]`).
    Slice {
        fields: Vec<LocalNodeId<PatternField>>,
    },
    /// Unresolved struct pattern (like `Vector2 { x: 0, y, z: zedso  }`).
    UnresolvedStruct {
        ty: LocalNodeId<Expression>,
        fields: Vec<LocalNodeId<PatternField>>,
    },
    /// Struct pattern (like `Vector2 { x: 0, y, z: zedso  }`).
    Struct {
        remote_symbol: GlobalSymbolId,
        fields: Vec<LocalNodeId<PatternField>>,
    },
    /// Union pattern (like `1 | 2 | 3`).
    Union { patterns: Vec<LocalNodeId<Pattern>> },
}

impl Node for Pattern {
    const TYPE: NodeType = NodeType::Pattern;

    fn is_resolved(&self) -> bool {
        true
    }
}

/// A PatternField is a field in a pattern (tuple, struct, union, etc.).
#[derive(Debug, Clone, PartialEq)]
pub enum PatternField {
    /// Named field, maybe with a pattern (like `x` or `x: 4` or `x: int32`).
    UnresolvedNamed {
        mutability: Option<Mutability>,
        name: StringId,
        pattern: Option<LocalNodeId<Pattern>>,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Named field with an alias (like `x: y`).
    UnresolvedAlias {
        mutability: Option<Mutability>,
        name: StringId,
        alias: StringId,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Positional field with just a pattern (like `4` or `int32`).
    UnresolvedPositional { pattern: LocalNodeId<Pattern> },
    /// Named field, maybe with a pattern (like `x` or `x: 4` or `x: int32`).
    Named {
        mutability: Option<Mutability>,
        name: StringId,
        pattern: Option<LocalNodeId<Pattern>>,
        default: Option<LocalNodeId<Expression>>,
        symbol: GlobalSymbolId,
    },
    /// Named field with an alias (like `x: y`).
    Alias {
        mutability: Option<Mutability>,
        name: StringId,
        alias: StringId,
        default: Option<LocalNodeId<Expression>>,
        symbol: GlobalSymbolId,
    },
    /// Positional field with just a pattern (like `4` or `int32`).
    Positional {
        pattern: LocalNodeId<Pattern>,
        symbol: GlobalSymbolId,
    },
}

impl Node for PatternField {
    const TYPE: NodeType = NodeType::PatternField;

    fn is_resolved(&self) -> bool {
        matches!(
            self,
            PatternField::Named { .. }
                | PatternField::Alias { .. }
                | PatternField::Positional { .. }
        )
    }
}
