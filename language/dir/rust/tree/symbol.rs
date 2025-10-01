/// The id of a symbol.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct SymbolId(pub u32);

/// A Path is a path of symbols.
#[derive(Debug, Clone, PartialEq)]
pub struct PathId {
    pub segments: Vec<SymbolId>,
}
