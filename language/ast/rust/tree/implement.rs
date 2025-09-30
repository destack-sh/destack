use crate::{Argument, Expression, Node, NodeId, NodeType, Type};

/// An Impl defines the implementation of a concrete type node.
/// There may be multiple Impls for the same type, and even impls for different modules.
/// (To add a module's implementation to your own just use the corresponding module.)
///
/// Examples:
/// ```
/// implement Foo {
///     ...
/// }
///
/// implement Foo<int32> {
///     ...
/// }
///
/// implement Bar<int32> for Baz {
///     ...
/// }
///
/// implement<T> Bar<T> for Baz {
///     ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Implement {
    /// The static arguments to the implement (leftmost static arguments).
    pub static_arguments: Option<Vec<NodeId<Argument>>>,
    /// The trait type to implement.
    pub receiver: NodeId<Type>,
    /// The type to implement the trait for.
    pub for_trait: Option<NodeId<Type>>,
    /// The statements of the implement.
    pub expressions: Vec<NodeId<Expression>>,
}

impl Node for Implement {
    const KIND: NodeType = NodeType::Implement;
}
