use crate::{
    BindingModifier, Expression, LocalNodeId, LocalSymbolId, Node, NodeType, Pattern,
    StaticExpression, StringId,
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
}

/// An Argument is a named, positional, spread, or dynamic argument.
/// Parameter mapping (which parameter an argument maps to) is resolved
/// as part of call resolution, not stored here.
#[derive(Debug, Clone, PartialEq)]
pub enum Argument {
    /// Named argument (like `foo: 42` in tree literals or named function args).
    Named {
        name: StringId,
        value: LocalNodeId<Expression>,
    },
    /// Positional argument (like `42` in `foo(42)`).
    Positional { value: LocalNodeId<Expression> },
    /// Spread argument (like `...args`).
    Spread { value: LocalNodeId<Expression> },
    /// Dynamic/computed argument (like `[key]: value`).
    Dynamic {
        key: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
    },
}

impl Argument {
    /// Get the value of the Argument.
    pub fn value(&self) -> LocalNodeId<Expression> {
        match self {
            Argument::Named { value, .. } => *value,
            Argument::Positional { value, .. } => *value,
            Argument::Spread { value, .. } => *value,
            Argument::Dynamic { value, .. } => *value,
        }
    }
}

impl Node for Argument {
    const TYPE: NodeType = NodeType::Argument;
}

/// Static argument in some static context.
/// Static evaluation supports all constructs, this is for the resulting static value.
#[derive(Debug, Clone, PartialEq)]
pub enum StaticArgument {
    /// Unevaluated argument (needs compile-time evaluation).
    Unevaluated { node: LocalNodeId<Argument> },

    /// Evaluated static argument.
    Evaluated {
        name: Option<StringId>,
        value: LocalNodeId<StaticExpression>,
    },
}

impl Node for StaticArgument {
    const TYPE: NodeType = NodeType::StaticArgument;

    fn is_evaluated(&self) -> bool {
        !matches!(self, StaticArgument::Unevaluated { .. })
    }
}
