use crate::{NodeIdAny, StringId};

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
    /// Built-in path.
    Intrinsic,
    /// External path.
    External,
    /// Not found path.
    NotFound,
}

/// A destination is a target for a control flow statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Destination {
    /// An unresolved label.
    Label(StringId),
    /// A resolved destination.
    Resolved(NodeIdAny),
    /// A not found destination.
    NotFound,
}
