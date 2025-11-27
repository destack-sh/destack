use crate::{
    Expression, GlobalSymbolId, LocalNodeId, LocalSymbolId, Mutability, Node, NodeType, StringId,
};

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
        target_symbol: Option<GlobalSymbolId>,
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
        target_symbol: Option<GlobalSymbolId>,
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
#[derive(Debug, Clone, PartialEq)]
pub enum PatternField {
    /// Named field, maybe with a pattern (like `x` or `x: 4` or `x: int32`).
    UnresolvedNamed {
        mutability: Option<Mutability>,
        name: StringId,
        pattern: Option<LocalNodeId<Pattern>>,
        default: Option<LocalNodeId<Expression>>,
        symbol: LocalSymbolId,
    },
    /// Named field with an alias (like `x: y`).
    UnresolvedAlias {
        mutability: Option<Mutability>,
        name: StringId,
        alias: StringId,
        default: Option<LocalNodeId<Expression>>,
        symbol: LocalSymbolId,
    },
    /// Positional field with just a pattern (like `4` or `int32`).
    UnresolvedPositional { pattern: LocalNodeId<Pattern> },
    /// Named field, maybe with a pattern (like `x` or `x: 4` or `x: int32`).
    Named {
        mutability: Option<Mutability>,
        name: StringId,
        pattern: Option<LocalNodeId<Pattern>>,
        default: Option<LocalNodeId<Expression>>,
        symbol: LocalSymbolId,
        target_symbol: GlobalSymbolId,
    },
    /// Named field with an alias (like `x: y`).
    Alias {
        mutability: Option<Mutability>,
        name: StringId,
        alias: StringId,
        default: Option<LocalNodeId<Expression>>,
        symbol: LocalSymbolId,
        target_symbol: GlobalSymbolId,
    },
    /// Positional field with just a pattern (like `4` or `int32`).
    Positional { pattern: LocalNodeId<Pattern> },
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

impl PatternField {
    /// Get the symbol of the pattern field.
    pub fn symbol(&self) -> Option<LocalSymbolId> {
        match self {
            PatternField::UnresolvedNamed { symbol, .. } => Some(*symbol),
            PatternField::UnresolvedAlias { symbol, .. } => Some(*symbol),
            PatternField::UnresolvedPositional { .. } => None,
            PatternField::Named { symbol, .. } => Some(*symbol),
            PatternField::Alias { symbol, .. } => Some(*symbol),
            PatternField::Positional { .. } => None,
        }
    }
}
