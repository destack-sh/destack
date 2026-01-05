use crate::{DependencyItem, LocalNodeId, LocalSymbolId, StaticKey, SymbolSpace};

/// The kind of an export entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportKind {
    /// A local symbol export.
    Local,
    /// A re-export via a dependency item.
    ReExport,
}

/// An Export is a resolved module export entry.
#[derive(Debug, Clone, PartialEq)]
pub struct Export {
    /// The export key.
    pub key: StaticKey,
    /// The export space.
    pub space: SymbolSpace,
    /// The export kind.
    pub kind: ExportKind,
    /// The local symbol export.
    pub symbol: Option<LocalSymbolId>,
    /// The re-export item.
    pub item: Option<LocalNodeId<DependencyItem>>,
}

impl Export {
    /// Create a local export entry.
    pub fn local(key: StaticKey, space: SymbolSpace, symbol: LocalSymbolId) -> Self {
        Self {
            key,
            space,
            kind: ExportKind::Local,
            symbol: Some(symbol),
            item: None,
        }
    }

    /// Create a re-export entry.
    pub fn reexport(key: StaticKey, space: SymbolSpace, item: LocalNodeId<DependencyItem>) -> Self {
        Self {
            key,
            space,
            kind: ExportKind::ReExport,
            symbol: None,
            item: Some(item),
        }
    }
}

/// The symbol space lookup order for a dependency kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolSpaceOrder {
    /// Do not consider any spaces.
    None,
    /// Consider only type exports.
    TypeOnly,
    /// Consider only value exports.
    ValueOnly,
    /// Consider type exports first, then values.
    TypeThenValue,
    /// Consider value exports first, then types.
    ValueThenType,
}

impl SymbolSpaceOrder {
    /// Return the spaces to check for this order (in order).
    pub fn spaces(self) -> &'static [SymbolSpace] {
        const NONE: [SymbolSpace; 0] = [];
        const TYPE_ONLY: [SymbolSpace; 1] = [SymbolSpace::Type];
        const VALUE_ONLY: [SymbolSpace; 1] = [SymbolSpace::Value];
        const TYPE_THEN_VALUE: [SymbolSpace; 2] = [SymbolSpace::Type, SymbolSpace::Value];
        const VALUE_THEN_TYPE: [SymbolSpace; 2] = [SymbolSpace::Value, SymbolSpace::Type];

        match self {
            SymbolSpaceOrder::None => &NONE,
            SymbolSpaceOrder::TypeOnly => &TYPE_ONLY,
            SymbolSpaceOrder::ValueOnly => &VALUE_ONLY,
            SymbolSpaceOrder::TypeThenValue => &TYPE_THEN_VALUE,
            SymbolSpaceOrder::ValueThenType => &VALUE_THEN_TYPE,
        }
    }
}
