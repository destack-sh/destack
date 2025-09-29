use crate::{Block, Expression, Node, NodeId, NodeType, StringId, Visibility};

/// A Use is a use declaration for dependency management.
/// Use can be used as statement for the containing scope or in block form.
/// `use` includes all or some items from a definition in the relevant scope.
///
/// Examples:
/// ```
/// use foo
/// use foo, bar
/// use foo.bar
/// use foo.{bar, baz}
/// use foo.{} // valid but linted
/// use foo as baz
///
/// use Heap {
///   ...
/// }
///
/// use Time, !Disk, !Network, !Allocation {
///   ...
/// }
///
/// use someLock() {
///
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Use {
    /// The visibility of the use declaration.
    pub visibility: Option<Visibility>,
    /// The clauses in this use declaration.
    pub clauses: Vec<NodeId<UseClause>>,
    /// The body of the use declaration.
    pub body: Option<NodeId<Block>>,
}

impl Node for Use {
    const KIND: NodeType = NodeType::Use;
}

/// A UseClause is a single clause in a use declaration.
///
/// Examples:
/// ```
/// foo
/// foo as bar
/// foo.bar as baz
/// foo.{baz, qux}
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UseClause {
    /// The target to use (like `foo.bar` in `use foo.bar.{baz, qux}`)
    pub target: NodeId<Expression>,
    /// The alias to use for the definition (like `bar` in `use foo as bar`)
    pub alias: Option<StringId>,
    /// The items to use from the target (like `{baz, qux}` in `use foo.bar.{baz, qux}`)
    pub items: Option<Vec<NodeId<UseItem>>>,
}

impl Node for UseClause {
    const KIND: NodeType = NodeType::UseClause;
}

/// A UseItem is an item to use in a use clause.
///
/// Examples:
/// ```
/// baz
/// qux as quux
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UseItem {
    /// The source name of the item (like `foo` in `foo as bar`)
    pub name: StringId,
    /// The alias to use for the item (like `bar` in `foo as bar`)
    pub alias: Option<StringId>,
}

impl Node for UseItem {
    const KIND: NodeType = NodeType::UseItem;
}
