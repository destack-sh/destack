use crate::{Expression, Node, NodeId, NodeType, StringId, Type, VariantField};

/// A Parameter is a parameter to some expression.
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    /// The name of the parameter.
    pub name: StringId,
    /// The type of the parameter.
    pub ty: Option<NodeId<Type>>,
    /// The default value of the parameter.
    pub default: Option<NodeId<Expression>>,
}

impl Node for Parameter {
    const KIND: NodeType = NodeType::Parameter;
}

/// A slot is the "position" target of an argument.
#[derive(Debug, Clone, PartialEq)]
pub enum ArgumentSlot {
    /// Parameter slot.
    Parameter { parameter: NodeId<Parameter> },
    /// Field slot.
    Field { field: NodeId<VariantField> },
}

/// An Argument is a named or positional argument to a function or method call.
#[derive(Debug, Clone, PartialEq)]
pub enum Argument {
    /// Unevaluated named argument.
    UnevaluatedNamed {
        name: StringId,
        value: NodeId<Expression>,
    },
    /// Unevaluated positional argument.
    UnevaluatedPositional { value: NodeId<Expression> },
    /// Unevaluated positional spread argument.
    UnevaluatedSpread { value: NodeId<Expression> },

    /// Direct argument (named or positional).
    Direct {
        name: StringId,
        slot: ArgumentSlot,
        value: NodeId<Expression>,
    },
    /// Spread argument.
    Spread {
        slot: ArgumentSlot,
        value: NodeId<Expression>,
    },
}

impl Node for Argument {
    const KIND: NodeType = NodeType::Argument;
}
