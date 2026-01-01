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

impl Parameter {
    /// Get the modifiers of the parameter.
    pub fn modifiers(&self) -> Option<&BindingModifier> {
        match self {
            Parameter::Named { modifiers, .. } => modifiers.as_ref(),
            Parameter::Pattern { modifiers, .. } => modifiers.as_ref(),
            Parameter::Variadic { modifiers, .. } => modifiers.as_ref(),
        }
    }

    /// Get the symbol of the parameter.
    pub fn symbol(&self) -> LocalSymbolId {
        match self {
            Parameter::Named { symbol, .. } => *symbol,
            Parameter::Pattern { symbol, .. } => *symbol,
            Parameter::Variadic { symbol, .. } => *symbol,
        }
    }
}

/// An Argument is a named, positional, spread, or labeled argument.
/// Parameter mapping (which parameter an argument maps to) is resolved
/// as part of call resolution, not stored here.
#[derive(Debug, Clone, PartialEq)]
pub enum Argument {
    /// Named argument (like `foo: 42` in tree literals).
    Named {
        name: StringId,
        value: LocalNodeId<Expression>,
    },
    /// Labeled tuple element (like `start: number` in `[start: number, end: number]`).
    /// (Labels are purely for documentation/tooling and don't affect type checking directly.)
    Labeled {
        label: StringId,
        value: LocalNodeId<Expression>,
    },
    /// Positional argument (like `42` in `foo(42)`).
    Positional { value: LocalNodeId<Expression> },
    /// Spread argument (like `...args` or `[...args: any[]]`).
    Spread {
        label: Option<StringId>,
        value: LocalNodeId<Expression>,
    },
}

impl Argument {
    /// Get the value of the Argument.
    pub fn value(&self) -> LocalNodeId<Expression> {
        match self {
            Argument::Named { value, .. } => *value,
            Argument::Labeled { value, .. } => *value,
            Argument::Positional { value, .. } => *value,
            Argument::Spread { value, .. } => *value,
        }
    }
}

impl Node for Argument {
    const TYPE: NodeType = NodeType::Argument;
}

/// Static argument in some static context.
/// Static evaluation supports all constructs, this is for the resulting static value.
/// This is a plain value type, not a tree node so we can pass it around directly.
#[derive(Debug, Clone, PartialEq)]
pub enum StaticArgument {
    /// Unevaluated argument (needs compile-time evaluation).
    Unevaluated { node: LocalNodeId<Argument> },

    /// Evaluated static argument.
    Evaluated {
        name: Option<StringId>,
        value: StaticExpression,
    },
}

impl StaticArgument {
    /// Check if the static argument and its value have been evaluated.
    pub fn is_evaluated(&self) -> bool {
        match self {
            StaticArgument::Unevaluated { .. } => false,
            StaticArgument::Evaluated { value, .. } => value.is_evaluated(),
        }
    }
}
