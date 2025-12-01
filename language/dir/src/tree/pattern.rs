use crate::{
    Expression, LocalNodeId, LocalSymbolId, Mutability, Node, NodeType, StringId,
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
    /// Tuple pattern (like `(x, 0)` or `Result.Success(_)`).
    /// If `ty` is present, it's a variant/newtype pattern; if None, it's anonymous.
    /// Type resolution is via the `ty` expression (which resolves to a Reference).
    Tuple {
        ty: Option<LocalNodeId<Expression>>,
        fields: Vec<LocalNodeId<PatternField>>,
    },
    /// Array or slice pattern (like `[1, 2, x]` or `[1, y, ..]`).
    Slice {
        fields: Vec<LocalNodeId<PatternField>>,
    },
    /// Struct pattern (like `{ x, y }` or `Vector2 { x: 0, y }`).
    /// If `ty` is present, it's a typed struct pattern; if None, it's anonymous.
    /// Type resolution is via the `ty` expression (which resolves to a Reference).
    Struct {
        ty: Option<LocalNodeId<Expression>>,
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
/// Field resolution (which struct field it maps to) is in ResolutionTable.
#[derive(Debug, Clone, PartialEq)]
pub enum PatternField {
    /// Named field, maybe with a pattern (like `x` or `x: 4` or `x: int32`).
    /// `symbol` is the LOCAL binding created by this field.
    Named {
        mutability: Option<Mutability>,
        name: StringId,
        pattern: Option<LocalNodeId<Pattern>>,
        default: Option<LocalNodeId<Expression>>,
        symbol: LocalSymbolId,
    },
    /// Named field with an alias (like `x: y` where `x` is the field name, `y` is the binding).
    /// `symbol` is the LOCAL binding created by the alias.
    Alias {
        mutability: Option<Mutability>,
        name: StringId,
        alias: StringId,
        default: Option<LocalNodeId<Expression>>,
        symbol: LocalSymbolId,
    },
    /// Positional field with just a pattern (like `4` or `int32`).
    Positional { pattern: LocalNodeId<Pattern> },
}

impl Node for PatternField {
    const TYPE: NodeType = NodeType::PatternField;

    fn is_resolved(&self) -> bool {
        true // field resolution is in ResolutionTable
    }
}

impl PatternField {
    /// Get the symbol of the pattern field (the local binding it creates).
    pub fn symbol(&self) -> Option<LocalSymbolId> {
        match self {
            PatternField::Named { symbol, .. } => Some(*symbol),
            PatternField::Alias { symbol, .. } => Some(*symbol),
            PatternField::Positional { .. } => None,
        }
    }
}
