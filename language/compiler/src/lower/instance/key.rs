use destack_dir as dir;

/// A concrete lowered instance key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum InstanceKey {
    /// A concrete instance keyed only by a symbol.
    Symbol(dir::GlobalSymbolId),
}

impl InstanceKey {
    /// Build an instance key for a symbol.
    pub(crate) const fn symbol(symbol: dir::GlobalSymbolId) -> Self {
        Self::Constructor(symbol)
    }
}
