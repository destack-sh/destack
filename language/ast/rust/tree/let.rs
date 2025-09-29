use crate::{Expression, Node, NodeId, NodeType, Pattern, ScopedMutability, Type, Visibility};

/// Let or var binding for constant or mutable variables.
/// Both let and var may destructure and pattern match.
///
/// Examples:
/// ```
/// let x = 1
/// let x: int32 = 1
/// let (x, y) = foo()
/// if let Some(x) = someFunction() {
///     ...
/// }
/// var x = 1
/// var x: int32 = 1
/// var x: int32 // implicitly uninitialized, must be set before use
/// if var Some(x) = someFunction() {
///     ...
/// }
///
/// let t? = foo() else { return }
/// let t = foo() ?? return;
#[derive(Debug, Clone, PartialEq)]
pub struct Let {
    /// Whether the binding is mutable.
    pub mutability: ScopedMutability,
    /// The visibility of the binding.
    pub visibility: Option<Visibility>,
    /// The pattern of the binding.
    pub pattern: NodeId<Pattern>,
    /// The type of the binding.
    pub r#type: Option<NodeId<Type>>,
    /// The value of the binding.
    pub value: Option<NodeId<Expression>>,
}

impl Node for Let {
    const KIND: NodeType = NodeType::Let;
}
