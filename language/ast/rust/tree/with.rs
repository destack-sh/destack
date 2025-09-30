use crate::{Expression, Node, NodeId, NodeType};

/// A With is a with declaration for context management.
/// With can declare the use of an item in a scope and refine type bounds.
///
/// Examples:
/// ```
/// with T: int32
/// with Foo
/// with Foo as Bar
/// with Foo, Bar
/// with Foo.Bar
/// with !Bar
/// with (
///    !Bar,
///    Time<F> // optional comma
///    F: Numeric
/// )
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct With {
    /// The clauses in this use declaration.
    pub clauses: Vec<NodeId<WithClause>>,
}

impl Node for With {
    const KIND: NodeType = NodeType::With;
}

/// A WithClause is a single clause in a with declaration.
/// It can be a type assertion (`T: Y`) or a use declaration (`Foo` or `Foo.Bar as Zeb`).
/// Only positive declarations should have aliases (checked later).
///
/// Examples:
/// ```
/// // declaration
/// Foo
/// Foo as Bar
/// Foo.Bar as Baz
/// // assertion
/// T: int32
/// Self: geom.Mesh<T>
/// T.Item: Copy
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum WithClause {
    Declaration {
        /// The item to use (like `Foo.Bar` in `with Foo.Bar`)
        target: NodeId<Expression>,
    },
    Assertion {
        /// The target to assert (like `T` in `with T: int32`)
        target: NodeId<Expression>,
        /// The assertion type (like `int32` in `with T: int32`)
        assertion: NodeId<Expression>,
    },
}

impl Node for WithClause {
    const KIND: NodeType = NodeType::WithClause;
}
