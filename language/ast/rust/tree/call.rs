use crate::{Argument, Expression, Node, NodeId, NodeType, Runtime};

/// Index reference.
///
/// Examples:
/// ```
/// foo[1]
/// foo[1..3]
/// foo["bar"]
/// foo().result[0][variable+1]
/// foo.1 // for member access tuple
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Index {
    Explicit {
        receiver: NodeId<Expression>,
        index: NodeId<Expression>,
    },
    Implicit {
        receiver: NodeId<Expression>,
        index: i64,
    },
}

impl Node for Index {
    const KIND: NodeType = NodeType::Index;
}

/// A Call is call to a function at runtime or compile time ("dynamic" or "static").
///
/// The function may or may not be declared as comptime (with a `@ prefix),
///  but the call must be prefixed with a `@` to qualify as a static call.
///
/// Examples:
/// ```
/// foo()
/// @foo(1, 2, 3)
/// foo<int32>(1, 2, 3)
/// foo<Validate: false>(1, 2, 3)
/// @foo(Vector2 {x: 1, y: 2}, (true, 3))
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    /// The runtime of the call (static or dynamic).
    pub runtime: Option<Runtime>,
    /// The receiver of the call (including function name).
    pub receiver: NodeId<Expression>,
    /// The static arguments to the call `<Arg1, Arg2, ...>`.
    pub static_arguments: Option<Vec<NodeId<Argument>>>,
    /// The dynamic arguments to the call `(arg1, arg2, ...)`.
    pub dynamic_arguments: Vec<NodeId<Argument>>,
}

impl Node for Call {
    const KIND: NodeType = NodeType::Call;
}
