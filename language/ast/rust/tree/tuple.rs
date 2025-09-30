use crate::{Node, NodeId, NodeType, StringId, Type};

/// A Tuple is tuple definition node.
/// Tuples are declared anonymously and inline.
/// The ',' separator is optional if newline-delimited.
///
/// Examples:
/// ```
/// ()
/// (a: int32, b: boolean)
/// (
///   a: int32
///   b: boolean
/// )
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Tuple {
    /// The elements of the tuple.
    pub elements: Vec<NodeId<TupleField>>,
}

impl Node for Tuple {
    const KIND: NodeType = NodeType::Tuple;
}

/// A TupleField is a tuple field definition.
/// Tuple elements may be named or anonymous, but cannot have default values.
#[derive(Debug, Clone, PartialEq)]
pub enum TupleField {
    Named {
        name: StringId,
        r#type: NodeId<Type>,
    },
    Positional {
        r#type: NodeId<Type>,
    },
}

impl Node for TupleField {
    const KIND: NodeType = NodeType::TupleField;
}
