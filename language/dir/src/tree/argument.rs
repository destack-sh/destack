use serde::{Deserialize, Serialize};

use crate::{
    Expression, LocalNodeId, Name, Node, NodeType, Pattern, StaticKey, StringId, SymbolKind,
    SymbolSpace, TypeExpression, VarianceModifier,
};

/// A declared generic parameter in source.
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
    /// Variadic type parameter.
    VariadicType {
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
    /// Variadic value parameter.
    VariadicValue {
        name: StringId,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        default: Option<LocalNodeId<Expression>>,
        is_comptime: bool,
    },
    /// Malformed generic parameter.
    Error,
}

impl Node for GenericParameter {
    const TYPE: NodeType = NodeType::GenericParameter;
}

impl GenericParameter {
    /// Return the symbol key introduced by this generic parameter.
    pub fn symbol_key(&self) -> Option<StaticKey> {
        match self {
            Self::Type { name, .. }
            | Self::VariadicType { name, .. }
            | Self::Value { name, .. }
            | Self::VariadicValue { name, .. } => Some(StaticKey::Name(*name)),
            Self::Error => None,
        }
    }

    /// Return the symbol space introduced by this generic parameter.
    pub fn symbol_space(&self) -> Option<SymbolSpace> {
        match self {
            Self::Type { .. } | Self::VariadicType { .. } => Some(SymbolSpace::Type),
            Self::Value { .. } | Self::VariadicValue { .. } => Some(SymbolSpace::Value),
            Self::Error => None,
        }
    }

    /// Return the symbol kind introduced by this generic parameter.
    pub fn symbol_kind(&self) -> Option<SymbolKind> {
        match self {
            Self::Type { .. } | Self::VariadicType { .. } => Some(SymbolKind::GenericTypeParameter),
            Self::Value { .. } | Self::VariadicValue { .. } => {
                Some(SymbolKind::GenericValueParameter)
            }
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
        is_comptime: bool,
    },
    /// Variadic pattern parameter.
    VariadicPattern {
        pattern: LocalNodeId<Pattern>,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        is_comptime: bool,
    },
    /// Malformed parameter.
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

    /// Return the default value attached to this parameter when present.
    pub fn default_value(&self) -> Option<LocalNodeId<Expression>> {
        match self {
            Self::Named { default, .. } | Self::Pattern { default, .. } => *default,
            Self::VariadicNamed { .. } | Self::VariadicPattern { .. } | Self::Error => None,
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

    /// Return whether this parameter may be omitted at the call site.
    pub fn is_optional(&self) -> bool {
        match self {
            Self::Named { is_optional, .. } | Self::Pattern { is_optional, .. } => *is_optional,
            Self::VariadicNamed { .. } | Self::VariadicPattern { .. } | Self::Error => false,
        }
    }
}

/// A generic argument in static argument position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GenericArgument {
    /// Type generic argument.
    Type { value: LocalNodeId<TypeExpression> },
    /// Spread type generic argument.
    SpreadType { value: LocalNodeId<TypeExpression> },
    /// Value generic argument.
    Value { value: LocalNodeId<Expression> },
    /// Spread value generic argument.
    SpreadValue { value: LocalNodeId<Expression> },
    /// Associated type refinement.
    AssociatedType {
        name: StringId,
        value: LocalNodeId<TypeExpression>,
    },
    /// Associated compile-time constant refinement.
    AssociatedConst {
        name: StringId,
        value: LocalNodeId<Expression>,
    },
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
