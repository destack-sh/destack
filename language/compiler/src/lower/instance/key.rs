use destack_dir::GlobalSymbolId;

/// A concrete lowered instance key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum InstanceKey {
    /// A concrete instance keyed only by a symbol.
    Symbol(GlobalSymbolId),
}

impl InstanceKey {
    /// Build an instance key for a symbol.
    pub(crate) const fn symbol(symbol: GlobalSymbolId) -> Self {
        Self::Symbol(symbol)
    }
}
