use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{
    Expression, LocalNodeId, Node, NodeFold, NodeType, Pattern, StaticKey, StringId, SymbolKind,
    TypeExpression, VarianceModifier,
};

/// A declared generic parameter in source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
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
    /// Lifetime parameter.
    Lifetime { name: StringId },
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
            Self::Type { name, .. } | Self::VariadicType { name, .. } | Self::Lifetime { name } => {
                Some(StaticKey::Name(*name))
            }
            Self::Error => None,
        }
    }

    /// Return the symbol kind introduced by this generic parameter.
    pub fn symbol_kind(&self) -> Option<SymbolKind> {
        match self {
            Self::Type { is_const, .. } | Self::VariadicType { is_const, .. } => {
                Some(match is_const {
                    true => SymbolKind::GenericConstParameter,
                    false => SymbolKind::GenericTypeParameter,
                })
            }
            Self::Lifetime { .. } => Some(SymbolKind::GenericLifetimeParameter),
            Self::Error => None,
        }
    }
}

/// A parameter to a callable construct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub enum Parameter {
    /// Named scalar parameter.
    Named {
        name: StringId,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        default: Option<LocalNodeId<Expression>>,
        is_optional: bool,
    },
    /// Pattern parameter.
    Pattern {
        pattern: LocalNodeId<Pattern>,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        default: Option<LocalNodeId<Expression>>,
        is_optional: bool,
    },
    /// Variadic named parameter.
    VariadicNamed {
        name: StringId,
        declared_type: Option<LocalNodeId<TypeExpression>>,
    },
    /// Variadic pattern parameter.
    VariadicPattern {
        pattern: LocalNodeId<Pattern>,
        declared_type: Option<LocalNodeId<TypeExpression>>,
    },
    /// Malformed parameter.
    Error,
}

impl Node for Parameter {
    const TYPE: NodeType = NodeType::Parameter;
}

impl Parameter {
    /// Return the name this parameter declares.
    pub fn name(&self) -> Option<StringId> {
        match self {
            Self::Named { name, .. } | Self::VariadicNamed { name, .. } => Some(*name),
            Self::Pattern { .. } | Self::VariadicPattern { .. } | Self::Error => None,
        }
    }

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

    /// Return the destructuring pattern attached to this parameter when present.
    pub fn pattern(&self) -> Option<LocalNodeId<Pattern>> {
        match self {
            Self::Pattern { pattern, .. } | Self::VariadicPattern { pattern, .. } => Some(*pattern),
            Self::Named { .. } | Self::VariadicNamed { .. } | Self::Error => None,
        }
    }

    /// Return the default value attached to this parameter when present.
    pub fn default_value(&self) -> Option<LocalNodeId<Expression>> {
        match self {
            Self::Named { default, .. } | Self::Pattern { default, .. } => *default,
            Self::VariadicNamed { .. } | Self::VariadicPattern { .. } | Self::Error => None,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub enum GenericArgument {
    /// Type generic argument.
    Type { value: LocalNodeId<TypeExpression> },
    /// Spread type generic argument.
    SpreadType { value: LocalNodeId<TypeExpression> },
    /// Associated type refinement.
    AssociatedType {
        name: StringId,
        value: LocalNodeId<TypeExpression>,
    },
    /// Associated const refinement.
    AssociatedConst {
        name: StringId,
        value: LocalNodeId<TypeExpression>,
    },
    /// Malformed generic argument slot.
    Error,
}

impl Node for GenericArgument {
    const TYPE: NodeType = NodeType::GenericArgument;
}

/// One tuple type element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
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

/// An argument to a call, sequence literal, or template interpolation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub enum Argument {
    /// Positional argument.
    Positional { value: LocalNodeId<Expression> },
    /// Spread argument.
    Spread { value: LocalNodeId<Expression> },
    /// Elided array element.
    Elision,
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
            Argument::Positional { value } | Argument::Spread { value } => Some(*value),
            Argument::Elision | Argument::Error => None,
        }
    }
}
