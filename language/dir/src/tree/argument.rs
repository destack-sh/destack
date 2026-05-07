use serde::{Deserialize, Serialize};

use crate::{
    Expression, GlobalNodeIdAny, LocalNodeId, LocalSymbolId, Name, Node, NodeType, Pattern,
    StaticExpression, StringId, TypeExpression, VarianceModifier, Visibility,
};

/// A generic parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GenericParameter {
    /// Type parameter.
    Type {
        name: StringId,
        variance: Option<VarianceModifier>,
        constraint: Option<LocalNodeId<TypeExpression>>,
        default: Option<LocalNodeId<TypeExpression>>,
        symbol: LocalSymbolId,
        is_const: bool,
    },
    /// Value parameter.
    Value {
        name: StringId,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        default: Option<LocalNodeId<Expression>>,
        symbol: LocalSymbolId,
        is_comptime: bool,
    },
    /// Malformed generic parameter slot.
    Error { symbol: LocalSymbolId },
}

impl Node for GenericParameter {
    const TYPE: NodeType = NodeType::GenericParameter;
}

impl GenericParameter {
    /// Get the symbol of the generic parameter.
    pub fn symbol(&self) -> LocalSymbolId {
        match self {
            GenericParameter::Type { symbol, .. }
            | GenericParameter::Value { symbol, .. }
            | GenericParameter::Error { symbol } => *symbol,
        }
    }
}

/// A parameter to a callable construct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Parameter {
    /// Named scalar parameter.
    Named {
        name: StringId,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        default: Option<LocalNodeId<Expression>>,
        visibility: Option<Visibility>,
        symbol: LocalSymbolId,
        is_readonly: bool,
        is_optional: bool,
    },
    /// Pattern parameter.
    Pattern {
        pattern: LocalNodeId<Pattern>,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        default: Option<LocalNodeId<Expression>>,
        symbol: LocalSymbolId,
        is_optional: bool,
    },
    /// Variadic named parameter.
    VariadicNamed {
        name: StringId,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        visibility: Option<Visibility>,
        symbol: LocalSymbolId,
        is_readonly: bool,
    },
    /// Variadic pattern parameter.
    VariadicPattern {
        pattern: LocalNodeId<Pattern>,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        symbol: LocalSymbolId,
    },
    /// Malformed parameter slot.
    Error { symbol: LocalSymbolId },
}

impl Node for Parameter {
    const TYPE: NodeType = NodeType::Parameter;
}

impl Parameter {
    /// Get the symbol of the parameter.
    pub fn symbol(&self) -> LocalSymbolId {
        match self {
            Parameter::Named { symbol, .. }
            | Parameter::Pattern { symbol, .. }
            | Parameter::VariadicNamed { symbol, .. }
            | Parameter::VariadicPattern { symbol, .. }
            | Parameter::Error { symbol } => *symbol,
        }
    }
}

/// A generic argument.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GenericArgument {
    /// Type generic argument.
    Type { value: LocalNodeId<TypeExpression> },
    /// Value generic argument.
    Value { value: LocalNodeId<Expression> },
    /// Malformed generic argument slot.
    Error,
}

impl Node for GenericArgument {
    const TYPE: NodeType = NodeType::GenericArgument;
}

/// One tuple type element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TupleElement {
    /// One non-spread tuple element.
    Element {
        label: Option<StringId>,
        value: LocalNodeId<TypeExpression>,
        is_optional: bool,
        is_readonly: bool,
    },
    /// One spread tuple element.
    Spread {
        label: Option<StringId>,
        value: LocalNodeId<TypeExpression>,
    },
    /// Malformed tuple element slot.
    Error,
}

impl Node for TupleElement {
    const TYPE: NodeType = NodeType::TupleElement;
}

impl TupleElement {
    /// Get the value type expression when one exists.
    pub fn value(&self) -> Option<LocalNodeId<TypeExpression>> {
        match self {
            TupleElement::Element { value, .. } | TupleElement::Spread { value, .. } => {
                Some(*value)
            }
            TupleElement::Error => None,
        }
    }
}

/// An argument to a runtime call or tree construct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Argument {
    /// Named argument.
    Named {
        name: Name,
        value: LocalNodeId<Expression>,
    },
    /// Labeled argument.
    Labeled {
        label: StringId,
        value: LocalNodeId<Expression>,
    },
    /// Positional argument.
    Positional { value: LocalNodeId<Expression> },
    /// Spread argument.
    Spread {
        label: Option<StringId>,
        value: LocalNodeId<Expression>,
    },
    /// Malformed argument slot.
    Error { value: LocalNodeId<Expression> },
}

impl Argument {
    /// Get the value of the argument.
    pub fn value(&self) -> LocalNodeId<Expression> {
        match self {
            Argument::Named { value, .. }
            | Argument::Labeled { value, .. }
            | Argument::Positional { value }
            | Argument::Spread { value, .. }
            | Argument::Error { value } => *value,
        }
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
    /// Unevaluated argument.
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
