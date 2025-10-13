use crate::{Definition, Intrinsic, NodeId, StringId};

#[derive(Debug, Clone, PartialEq)]
pub enum Symbol {
    // NOTE #Incomplete: Symbol?
}

/// The id of a symbol.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct SymbolId(pub u32);

/// A resolved path.
#[derive(Debug, Clone, PartialEq)]
pub enum Path {
    /// Unevaluated absolute string path.
    Unevaluated { segments: Vec<StringId> },
    /// Resolved Path to an intrinsic.
    Intrinsic { intrinsic: Intrinsic },
    /// Resolved to a Definition.
    Definition { definition: NodeId<Definition> },
    /// Error path.
    Error,
}

/// A destination is a target for a control flow statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Destination {
    /// Unevaluated Destination with a string label.
    LabelString(StringId),
    /// Resolved Destination to a Definition.
    Definition { definition: NodeId<Definition> },

    /// Error destination.
    Error,
}
