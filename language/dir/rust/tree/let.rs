use crate::{
    Expression, Node, NodeId, NodeType, Pattern, ScopedMutability, Type, Visibility,
};

/// A Let is a let or var binding for constant or mutable variables.
#[derive(Debug, Clone, PartialEq)]
pub struct Let {
    /// The pattern to bind to.
    pattern: NodeId<Pattern>,
    /// The mutability of the binding.
    mutability: ScopedMutability,
    /// The visibility of the binding.
    visibility: Option<Visibility>,
    /// The type of the binding.
    ty: Option<NodeId<Type>>,
    /// The value of the binding.
    value: Option<NodeId<Expression>>,
}

impl Node for Let {
    const KIND: NodeType = NodeType::Let;
}
