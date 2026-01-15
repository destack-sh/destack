use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{DependencyItem, GlobalSymbolId, LocalNodeId, LocalSymbolId, StaticKey, SymbolSpace};

/// The kind of an export entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportKind {
    /// A local symbol export.
    Local,
    /// A reexport via a dependency item.
    ReExport,
}

/// The resolution state of an export target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportTarget {
    /// A resolved export target.
    Resolved(GlobalSymbolId),
    /// An unresolved export target.
    Unresolved,
}

impl ExportTarget {
    /// Return the resolved target symbol, if any.
    pub fn resolved(self) -> Option<GlobalSymbolId> {
        match self {
            ExportTarget::Resolved(symbol) => Some(symbol),
            ExportTarget::Unresolved => None,
        }
    }
}

/// An Export is a resolved module export entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Export {
    /// The export key.
    pub key: StaticKey,
    /// The export space.
    pub space: SymbolSpace,
    /// The export kind.
    pub kind: ExportKind,
    /// The export target resolution state.
    pub target: ExportTarget,
    /// The local symbol export.
    pub symbol: Option<LocalSymbolId>,
    /// The reexport item.
    pub item: Option<LocalNodeId<DependencyItem>>,
}

impl Export {
    /// Create a local export entry.
    pub fn local(
        module_id: ModuleId,
        key: StaticKey,
        space: SymbolSpace,
        symbol: LocalSymbolId,
    ) -> Self {
        Self {
            key,
            space,
            kind: ExportKind::Local,
            target: ExportTarget::Resolved(symbol.into_global(module_id)),
            symbol: Some(symbol),
            item: None,
        }
    }

    /// Create a reexport entry.
    pub fn reexport(key: StaticKey, space: SymbolSpace, item: LocalNodeId<DependencyItem>) -> Self {
        Self {
            key,
            space,
            kind: ExportKind::ReExport,
            target: ExportTarget::Unresolved,
            symbol: None,
            item: Some(item),
        }
    }

    /// Resolve the export target to a concrete symbol.
    pub fn resolve_target(&mut self, target: GlobalSymbolId) {
        self.target = ExportTarget::Resolved(target);
    }
}

/// The symbol space lookup order for a dependency kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
