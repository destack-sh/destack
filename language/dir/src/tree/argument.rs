use serde::{Deserialize, Serialize};

use crate::{
    Expression, LocalNodeId, Name, Node, NodeType, Pattern, StaticKey, StringId, SymbolForm,
    SymbolSpace, TypeExpression, VarianceModifier, Visibility,
};

/// A generic parameter in static parameter position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GenericParameter {
    /// Type parameter.
    Type {
        name: StringId,
        variance: Option<VarianceModifier>,
        constraint: Option<LocalNodeId<TypeExpression>>,
        default: Option<LocalNodeId<TypeExpression>>,
        is_const: bool,
    },
    /// Value parameter.
    Value {
        name: StringId,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        default: Option<LocalNodeId<Expression>>,
        is_comptime: bool,
    },
    /// Malformed generic parameter slot.
    Error,
}

impl Node for GenericParameter {
    const TYPE: NodeType = NodeType::GenericParameter;
}

impl GenericParameter {
    /// Return the symbol key introduced by this generic parameter.
    pub fn symbol_key(&self) -> Option<StaticKey> {
        match self {
            Self::Type { name, .. } | Self::Value { name, .. } => Some(StaticKey::Name(*name)),
            Self::Error => None,
        }
    }

    /// Return the symbol space introduced by this generic parameter.
    pub fn symbol_space(&self) -> Option<SymbolSpace> {
        match self {
            Self::Type { .. } => Some(SymbolSpace::Type),
            Self::Value { .. } => Some(SymbolSpace::Value),
            Self::Error => None,
        }
    }

    /// Return the symbol form introduced by this generic parameter.
    pub fn symbol_form(&self) -> Option<SymbolForm> {
        match self {
            Self::Type { .. } => Some(SymbolForm::TypeAlias),
            Self::Value { .. } => Some(SymbolForm::Variable),
            Self::Error => None,
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
        is_readonly: bool,
        is_optional: bool,
        is_comptime: bool,
    },
    /// Pattern parameter.
    Pattern {
        pattern: LocalNodeId<Pattern>,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        default: Option<LocalNodeId<Expression>>,
        is_optional: bool,
        is_comptime: bool,
    },
    /// Variadic named parameter.
    VariadicNamed {
        name: StringId,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        visibility: Option<Visibility>,
        is_readonly: bool,
        is_comptime: bool,
    },
    /// Variadic pattern parameter.
    VariadicPattern {
        pattern: LocalNodeId<Pattern>,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        is_comptime: bool,
    },
    /// Malformed parameter slot.
    Error,
}

impl Node for Parameter {
    const TYPE: NodeType = NodeType::Parameter;
}

impl Parameter {
    /// Return the symbol key introduced by this parameter.
    pub fn symbol_key(&self) -> Option<StaticKey> {
        match self {
            Self::Named { name, .. } | Self::VariadicNamed { name, .. } => {
                Some(StaticKey::Name(*name))
            }
            Self::Pattern { .. } | Self::VariadicPattern { .. } | Self::Error => None,
        }
    }

    /// Return the declared type attached to this parameter when present.
    pub fn declared_type(&self) -> Option<LocalNodeId<TypeExpression>> {
        match self {
            Self::Named { declared_type, .. }
            | Self::Pattern { declared_type, .. }
            | Self::VariadicNamed { declared_type, .. }
            | Self::VariadicPattern { declared_type, .. } => *declared_type,
            Self::Error => None,
        }
    }

    /// Return whether this parameter must be a static call-site argument.
    pub fn is_comptime(&self) -> bool {
        match self {
            Self::Named { is_comptime, .. }
            | Self::Pattern { is_comptime, .. }
            | Self::VariadicNamed { is_comptime, .. }
            | Self::VariadicPattern { is_comptime, .. } => *is_comptime,
            Self::Error => false,
        }
    }
}

/// A generic argument in static argument position.
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
    Error,
}

impl Node for Argument {
    const TYPE: NodeType = NodeType::Argument;
}

impl Argument {
    /// Return the argument value expression.
    #[inline]
    pub fn value(&self) -> Option<LocalNodeId<Expression>> {
        match self {
            Argument::Named { value, .. }
            | Argument::Labeled { value, .. }
            | Argument::Positional { value }
            | Argument::Spread { value, .. } => Some(*value),
            Argument::Error => None,
        }
    }
}
