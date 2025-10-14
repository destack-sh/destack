use crate::{Expression, Node, NodeId, NodeType, StringId, Type, VariantField};

/// A Parameter is a parameter to some construct.
#[derive(Debug, Clone, PartialEq)]
pub enum Parameter {
    /// Named parameter (like `T`, `x: int32` or `Validate: boolean = true`).
    Scalar {
        name: StringId,
        ty: Option<NodeId<Type>>,
        default: Option<NodeId<Expression>>,
    },
    /// Variadic parameter (like `..T` or `...x: int32[]`).
    Variadic {
        name: StringId,
        ty: Option<NodeId<Type>>,
    },
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
