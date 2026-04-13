use serde::{Deserialize, Serialize};

use crate::{
    Expression, LocalNodeId, Node, NodeType, Pattern, StringId, TypeExpression, VarianceModifier,
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

/// A parameter to a callable construct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Parameter {
    /// Named scalar parameter.
    Named {
        name: StringId,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Pattern parameter.
    Pattern {
        pattern: LocalNodeId<Pattern>,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        default: Option<LocalNodeId<Expression>>,
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
    /// Malformed parameter slot.
    Error,
}

impl Node for Parameter {
    const TYPE: NodeType = NodeType::Parameter;
}

/// A generic argument in static argument position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GenericArgument {
    /// Type argument.
    Type {
        label: Option<StringId>,
        value: LocalNodeId<TypeExpression>,
    },
    /// Value argument.
    Value {
        label: Option<StringId>,
        value: LocalNodeId<Expression>,
        is_comptime: bool,
    },
    /// Spread type argument.
    SpreadType {
        label: Option<StringId>,
        value: LocalNodeId<TypeExpression>,
    },
    /// Spread value argument.
    SpreadValue {
        label: Option<StringId>,
        value: LocalNodeId<Expression>,
        is_comptime: bool,
    },
    /// Malformed generic argument slot.
    Error,
}

impl Node for GenericArgument {
    const TYPE: NodeType = NodeType::GenericArgument;
}

/// An argument to a runtime call or tree construct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Argument {
    /// Named argument.
    Named {
        name: StringId,
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
