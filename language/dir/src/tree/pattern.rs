use crate::{Expression, LocalNodeId, LocalSymbolId, Mutability, Node, NodeType, StringId};

/// A Pattern is a pattern to match something and unwrap it.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Wildcard scalar pattern (`_`).
    Wildcard,
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
    /// Anonymous tuple pattern (like `(x, 0)`).
    Tuple {
        fields: Vec<LocalNodeId<PatternField>>,
    },
    /// Tagged tuple pattern (like `Result.Success(_)`).
    TaggedTuple {
        ty: LocalNodeId<Expression>,
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
        ty: LocalNodeId<Expression>,
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
    /// Spread field (like `...x`).
    Spread {
        mutability: Option<Mutability>,
        name: Option<StringId>,
        symbol: LocalSymbolId,
    },
}

impl Node for PatternField {
    const TYPE: NodeType = NodeType::PatternField;
}

impl PatternField {
    /// Get the symbol of the pattern field (the local binding it creates).
    pub fn symbol(&self) -> Option<LocalSymbolId> {
        match self {
            PatternField::Named { symbol, .. } => Some(*symbol),
            PatternField::Alias { symbol, .. } => Some(*symbol),
            PatternField::Positional { .. } => None,
            PatternField::Spread { symbol, .. } => Some(*symbol),
        }
    }
}
