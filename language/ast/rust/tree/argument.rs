use crate::{Expression, Node, NodeId, NodeType, StringId};

/// A Parameter is a parameter to some construct.
///
/// Examples:
/// ```
/// x
/// T
/// x: int32
/// y: (int32, boolean, Vector2)
/// Validate: boolean = true
/// z: int32 = 4
/// ..T
/// ...x: int32[]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Parameter {
    /// Named parameter (like `T`, `x: int32` or `Validate: boolean = true`).
    Scalar {
        name: StringId,
        ty: Option<NodeId<Expression>>,
        default: Option<NodeId<Expression>>,
    },
    /// Variadic parameter (like `..T` or `...x: int32[]`).
    Variadic {
        name: StringId,
        ty: Option<NodeId<Expression>>,
    },
}

impl Node for Parameter {
    const KIND: NodeType = NodeType::Parameter;
}

/// An Argument is an argument to a function call.
/// It may be named or positional. Named shorthands are only supported in struct-like literals.
/// Can be used in static and dynamic contexts (e.g. in [..] or (..)).
///
/// Examples:
/// ```
/// x: 1
/// y: foo()
/// y
/// false
/// ...args
/// foo()
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Argument {
    /// Named argument.
    Named {
        name: StringId,
        value: NodeId<Expression>,
    },
    /// Named shorthand argument (only in certain contexts like struct literals).
    NamedShorthand { name: StringId },
    /// Named shorthand function argument (only in certain contexts like struct literals).
    ImplicitFunction {
        name: StringId,
        value: NodeId<Expression>,
    },
    /// Positional argument.
    Positional { value: NodeId<Expression> },
    /// Positional spread argument.
    Spread { value: NodeId<Expression> },
}

impl Node for Argument {
    const KIND: NodeType = NodeType::Argument;
}
