use crate::{Argument, Expression, Node, NodeId, NodeType, Runtime};

/// Index into a receiver expression.
///
/// Examples:
/// ```
/// T[] // special declarative
/// foo[1]
/// foo[1..3]
/// foo["bar"]
/// foo().result[0][variable+1]
/// foo.1 // for member access tuple
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Index {
    Declarative {
        receiver: NodeId<Expression>,
    },
    Explicit {
        receiver: NodeId<Expression>,
        index: NodeId<Expression>,
    },
    Member {
        receiver: NodeId<Expression>,
        index: i64,
    },
}

impl Node for Index {
    const KIND: NodeType = NodeType::Index;
}

/// A Call is call to a function OR an instantiation of a tuple type.
/// The static arguments are expressed in the receiver, not the call.
///
/// The function may or may not be declared as comptime (with a `@ prefix),
///  but the call must be prefixed with a `@` to qualify as a static call.
///
/// Examples:
/// ```
/// foo()
/// @foo(1, 2, 3)
/// @foo(Vector2 {x: 1, y: 2}, (true, 3))
/// Bar(1, 2, 3)
/// MyUnion.Baz(2, 3)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    /// The runtime of the call (static or dynamic).
    pub runtime: Option<Runtime>,
    /// The receiver of the call (including function name / tuple type name).
    pub receiver: NodeId<Expression>,
    /// The dynamic arguments to the call `(arg1, arg2, ...)`.
    pub dynamic_arguments: Vec<NodeId<Argument>>,
}

impl Node for Call {
    const KIND: NodeType = NodeType::Call;
}
