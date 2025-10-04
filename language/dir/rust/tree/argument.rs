use crate::{Expression, Node, NodeId, NodeType, StringId, Type};

/// A Parameter is a parameter to some expression.
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: StringId,
    pub ty: Option<NodeId<Type>>,
    pub default: Option<NodeId<Expression>>,
}

impl Node for Parameter {
    const KIND: NodeType = NodeType::Parameter;
}

/// An Argument is a named or positional argument to a function or method call.
#[derive(Debug, Clone, PartialEq)]
pub enum Argument {
    /// A named argument.
    Named {
        name: StringId,
        value: NodeId<Expression>,
    },
    /// A positional argument.
    Positional(NodeId<Expression>),
}

impl Node for Argument {
    const KIND: NodeType = NodeType::Argument;
}
