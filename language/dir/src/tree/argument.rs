use crate::{
    BindingModifier, Expression, GlobalSymbolId, LocalNodeId, LocalSymbolId, Node, NodeType,
    Pattern, StringId,
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

/// An Argument is a named or positional argument to a function or method call.
#[derive(Debug, Clone, PartialEq)]
pub enum Argument {
    /// Unresolved named argument.
    UnresolvedNamed {
        modifiers: Option<BindingModifier>,
        name: StringId,
        value: LocalNodeId<Expression>,
    },
    /// Unresolved positional argument.
    UnresolvedPositional {
        modifiers: Option<BindingModifier>,
        value: LocalNodeId<Expression>,
    },
    /// Unresolved positional spread argument.
    UnresolvedSpread {
        modifiers: Option<BindingModifier>,
        name: Option<StringId>,
        value: LocalNodeId<Expression>,
    },
    /// Unresolved dynamic argument.
    UnresolvedDynamic {
        modifiers: Option<BindingModifier>,
        name: Option<StringId>,
        key: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
    },

    /// Direct argument (named or positional).
    Direct {
        modifiers: Option<BindingModifier>,
        name: StringId,
        target_symbol: GlobalSymbolId,
        value: LocalNodeId<Expression>,
    },
    /// Spread argument.
    Spread {
        modifiers: Option<BindingModifier>,
        name: StringId,
        target_symbol: GlobalSymbolId,
        value: LocalNodeId<Expression>,
    },
    /// Dynamic argument.
    Dynamic {
        modifiers: Option<BindingModifier>,
        name: Option<StringId>,
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
