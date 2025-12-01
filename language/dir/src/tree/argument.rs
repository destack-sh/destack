use crate::{
    BindingModifier, Expression, GlobalSymbolId, LocalNodeId, LocalSymbolId, Node, NodeType,
    Pattern, StaticExpression, StringId,
};

/// A Parameter is a parameter to some construct.
#[derive(Debug, Clone, PartialEq)]
pub enum Parameter {
    /// Named scalar parameter (like `T`, `x: int32` or `Validate: boolean = true`).
    Named {
        modifiers: Option<BindingModifier>,
        name: StringId,
        default: Option<LocalNodeId<Expression>>,
        symbol: LocalSymbolId,
    },
    /// Pattern parameter (like `_` or `{ x }` or `{ x }: MyType = Foo`).
    Pattern {
        modifiers: Option<BindingModifier>,
        pattern: LocalNodeId<Pattern>,
        default: Option<LocalNodeId<Expression>>,
        symbol: LocalSymbolId,
    },
    /// Variadic parameter (like `..T` or `...x: int32[]`).
    Variadic {
        modifiers: Option<BindingModifier>,
        name: StringId,
        symbol: LocalSymbolId,
    },
}

impl Node for Parameter {
    const TYPE: NodeType = NodeType::Parameter;

    fn is_resolved(&self) -> bool {
        true
    }
}

/// An Argument is a named or positional argument.
/// Named arguments are only valid in tree literals (JSX-like attributes).
#[derive(Debug, Clone, PartialEq)]
pub enum Argument {
    /// Unresolved named argument (tree literals only).
    UnresolvedNamed {
        name: StringId,
        value: LocalNodeId<Expression>,
    },
    /// Unresolved positional argument.
    UnresolvedPositional { value: LocalNodeId<Expression> },
    /// Unresolved spread argument.
    UnresolvedSpread { value: LocalNodeId<Expression> },
    /// Unresolved dynamic argument.
    UnresolvedDynamic {
        key: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
    },

    /// Direct argument (named or positional).
    Direct {
        name: StringId,
        target_symbol: GlobalSymbolId,
        value: LocalNodeId<Expression>,
    },
    /// Spread argument.
    Spread {
        name: StringId,
        target_symbol: GlobalSymbolId,
        value: LocalNodeId<Expression>,
    },
    /// Dynamic argument.
    Dynamic {
        target_symbol: GlobalSymbolId,
        key: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
    },
}

impl Argument {
    /// Get the value of the Argument.
    pub fn value(&self) -> LocalNodeId<Expression> {
        match self {
            Argument::UnresolvedNamed { value, .. } => *value,
            Argument::UnresolvedPositional { value, .. } => *value,
            Argument::UnresolvedSpread { value, .. } => *value,
            Argument::UnresolvedDynamic { value, .. } => *value,
            Argument::Direct { value, .. } => *value,
            Argument::Spread { value, .. } => *value,
            Argument::Dynamic { value, .. } => *value,
        }
    }
}

impl Node for Argument {
    const TYPE: NodeType = NodeType::Argument;

    fn is_resolved(&self) -> bool {
        matches!(self, Argument::Direct { .. } | Argument::Spread { .. })
    }
}

/// Static argument in some static context.
/// Static evaluation supports all constructs, this is for the resulting static value.
#[derive(Debug, Clone, PartialEq)]
pub enum StaticArgument {
    /// Unresolved dynamic argument.
    Unresolved { node: LocalNodeId<Argument> },

    /// Named static argument.
    Direct {
        name: StringId,
        target_symbol: GlobalSymbolId,
        value: LocalNodeId<StaticExpression>,
    },
}

impl Node for StaticArgument {
    const TYPE: NodeType = NodeType::StaticArgument;

    fn is_resolved(&self) -> bool {
        !matches!(self, StaticArgument::Unresolved { .. })
    }
}
