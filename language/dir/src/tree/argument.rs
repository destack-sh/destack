use serde::{Deserialize, Serialize};

use crate::{
    BindingModifier, Expression, GlobalNodeId, GlobalNodeIdAny, GlobalSymbolId, LocalNodeId,
    LocalSymbolId, LocalTypeId, Node, NodeType, Pattern, StaticExpression, StringId,
};

/// A Parameter is a parameter to some construct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Variadic parameter with a named binding (like `..T` or `...x: int32[]`).
    VariadicNamed {
        modifiers: Option<BindingModifier>,
        name: StringId,
        symbol: LocalSymbolId,
    },
    /// Variadic parameter with a pattern binding (like `...[x, y]`).
    VariadicPattern {
        modifiers: Option<BindingModifier>,
        pattern: LocalNodeId<Pattern>,
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
            Parameter::VariadicNamed { modifiers, .. } => modifiers.as_ref(),
            Parameter::VariadicPattern { modifiers, .. } => modifiers.as_ref(),
        }
    }

    /// Report whether the parameter has a default value.
    pub fn has_default(&self) -> bool {
        match self {
            Parameter::Named { default, .. } | Parameter::Pattern { default, .. } => {
                default.is_some()
            }
            Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. } => false,
        }
    }

    /// Get the symbol of the parameter.
    pub fn symbol(&self) -> LocalSymbolId {
        match self {
            Parameter::Named { symbol, .. } => *symbol,
            Parameter::Pattern { symbol, .. } => *symbol,
            Parameter::VariadicNamed { symbol, .. } => *symbol,
            Parameter::VariadicPattern { symbol, .. } => *symbol,
        }
    }
}

/// An Argument is a named, positional, spread, or labeled argument.
/// Parameter mapping (which parameter an argument maps to) is resolved
/// as part of call resolution, not stored here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Argument {
    /// Named argument (like `foo: 42` in tree literals).
    Named {
        modifiers: Option<BindingModifier>,
        name: StringId,
        value: LocalNodeId<Expression>,
    },
    /// Labeled tuple element (like `start: number` in `[start: number, end: number]`).
    /// (Labels are purely for documentation/tooling and don't affect type checking directly.)
    Labeled {
        modifiers: Option<BindingModifier>,
        label: StringId,
        value: LocalNodeId<Expression>,
    },
    /// Positional argument (like `42` in `foo(42)`).
    Positional {
        modifiers: Option<BindingModifier>,
        value: LocalNodeId<Expression>,
    },
    /// Spread argument (like `...args` or `[...args: any[]]`).
    Spread {
        modifiers: Option<BindingModifier>,
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

    /// Return whether the argument carries an explicit label.
    pub fn is_named(&self) -> bool {
        matches!(self, Argument::Named { .. } | Argument::Labeled { .. })
    }
}

impl Node for Argument {
    const TYPE: NodeType = NodeType::Argument;
}

/// Static argument in some static context.
/// Static evaluation supports all constructs, this is for the resulting static value.
/// This is a plain value type, not a tree node so we can pass it around directly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StaticArgument {
    /// Unevaluated argument (needs compile-time evaluation).
    Unevaluated { node: GlobalNodeIdAny },

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

    /// Build an evaluated static argument from a static expression.
    pub fn value(value: StaticExpression) -> Self {
        Self::Evaluated { name: None, value }
    }
}

/// Describe how a static parameter is interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StaticParameterKind {
    /// Use the parameter as a type argument.
    Type,
    /// Use the parameter as a value argument.
    Value,
}

/// Metadata for resolving and validating a static parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticParameter {
    /// Whether this is a type or value parameter.
    pub kind: StaticParameterKind,
    /// Identify the static parameter symbol.
    pub symbol: GlobalSymbolId,
    /// Parameter name for mapping and diagnostics.
    pub name: Option<StringId>,
    /// Declared type for validation.
    pub declared_type_id: LocalTypeId,
    /// Default expression for missing arguments.
    pub default_expression: Option<GlobalNodeId<Expression>>,
}
