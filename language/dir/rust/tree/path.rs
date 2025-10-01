use crate::SymbolId;

/// A Path is a path of symbols.
#[derive(Debug, Clone, PartialEq)]
pub struct PathId {
    pub segments: Vec<SymbolId>,
}
