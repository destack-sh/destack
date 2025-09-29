use crate::{Expression, Node, NodeId, NodeType, StringId, Type};

/// A Parameter is a parameter to some expression.
/// Can be used in static and dynamic contexts (e.g. in [..] or (..)).
///
/// Examples:
/// ```
/// T
/// x: int32
/// y: (int32, boolean, Vector2)
/// Validate: boolean = true
/// z: int32 = 4
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    /// The name of the parameter.
    pub name: StringId,
    /// The type of the parameter.
    pub r#type: Option<NodeId<Type>>,
    /// The default value of the parameter.
    pub default: Option<NodeId<Expression>>,
}

impl Node for Parameter {
    const KIND: NodeType = NodeType::Parameter;
}

/// An Argument is an argument to a function call in the AST.
/// It may be named or positional.
/// Can be used in static and dynamic contexts (e.g. in [..] or (..)).
///
/// Examples:
/// ```
/// x: 1
/// y: 2
/// y
/// false
/// z: foo() > 7
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Argument {
    /// A named argument.
    Named {
        name: StringId,
        value: NodeId<Expression>,
    },
    /// A named shorthand argument.
    NamedShorthand { name: StringId },
    /// A positional argument.
    Positional { value: NodeId<Expression> },
}

impl Node for Argument {
    const KIND: NodeType = NodeType::Argument;
}
