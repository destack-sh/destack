/// The id of a symbol.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct SymbolId(pub u32);

/// A resolved path.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedPath {
	/// Built-in path.
    Intrinsic,
    /// External path.
    External,
    /// Not found path.
    NotFound,
}
