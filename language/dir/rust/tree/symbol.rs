use crate::{Definition, Intrinsic, NodeId, StringId, Type};

#[derive(Debug, Clone, PartialEq)]
pub enum Symbol {
    // nocheckin
}

/// The id of a symbol.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct SymbolId(pub u32);

/// A resolved path.
#[derive(Debug, Clone, PartialEq)]
pub enum Path {
    /// Resolved Path to an intrinsic.
    Intrinsic { intrinsic: Intrinsic },
    /// Unresolved absolute string path.
    AbsoluteString { segments: Vec<StringId> },
    /// Unresolved relative string path.
    RelativeString {
        root: NodeId<Type>,
        segments: Vec<StringId>,
    },
    /// Resolved to a Definition.
    Definition { definition: NodeId<Definition> },

    /// Error path.
    Error,
}

/// A destination is a target for a control flow statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Destination {
    /// Unresolved Destination with a string label.
    LabelString(StringId),
    /// Resolved Destination to a Definition.
    Definition { definition: NodeId<Definition> },

    /// Error destination.
    Error,
}
