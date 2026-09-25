use tspp_dir as dir;

use crate::FragmentId;

/// The index of an operation in a pattern tree.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u32);

/// A range in the tree's flat child storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeList {
    /// The first child index.
    pub start: u32,
    /// The child count.
    pub length: u32,
}

/// An operation evaluated against a candidate DIR node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// Match a parsed DIR fragment.
    Fragment(FragmentId),
    /// Match any node with the given DIR type.
    NodeType(dir::NodeType),
    /// Require every referenced operation.
    All(NodeList),
    /// Require at least one referenced operation.
    Any(NodeList),
    /// Reject a referenced operation.
    Not(NodeId),
    /// Search ancestors for a referenced operation.
    Inside(Relation),
    /// Search descendants for a referenced operation.
    Has(Relation),
    /// Search following siblings for a referenced operation.
    Precedes(Relation),
    /// Search preceding siblings for a referenced operation.
    Follows(Relation),
    /// Match a sibling position.
    NthChild(NthChild),
}

/// A pattern searched along one DIR relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Relation {
    /// The operation applied to related nodes.
    pub pattern: NodeId,
    /// The search limit.
    pub stop: RelationStop,
}

/// The limit applied while traversing a DIR relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelationStop {
    /// Visit the nearest related node.
    Neighbor,
    /// Visit every related node.
    End,
    /// Stop at the first node matched by the referenced operation.
    Pattern(NodeId),
}

/// A CSS-style `an + b` sibling position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NthChild {
    /// The `a` coefficient.
    pub step: i32,
    /// The `b` offset.
    pub offset: i32,
    /// The operation filtering counted siblings.
    pub pattern: Option<NodeId>,
}
