use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{DependencyItem, GlobalSymbolId, LocalNodeId, LocalSymbolId, StaticKey, SymbolSpace};

/// The source declaration of an export entry.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ExportSource {
    /// A symbol declared in this module.
    Local(LocalSymbolId),
    /// A dependency item that forwards an export from another module.
    ReExport(LocalNodeId<DependencyItem>),
}

/// A resolved module export entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Export {
    /// The export key.
    pub key: StaticKey,
    /// The export space.
    pub space: SymbolSpace,
    /// The source declaration for this export entry.
    pub source: ExportSource,
    /// The symbol exported under this key.
    pub target: GlobalSymbolId,
    /// Canonical export dependency symbols.
    pub dependencies: Vec<GlobalSymbolId>,
}

impl Export {
    /// Create a local export entry.
    pub fn local(
        module_id: ModuleId,
        key: StaticKey,
        space: SymbolSpace,
        symbol: LocalSymbolId,
    ) -> Self {
        let target = symbol.into_global(module_id);

        Self {
            key,
            space,
            source: ExportSource::Local(symbol),
            target,
            dependencies: Vec::new(),
        }
    }

    /// Create a reexport entry.
    pub fn reexport(
        key: StaticKey,
        space: SymbolSpace,
        item: LocalNodeId<DependencyItem>,
        target: GlobalSymbolId,
        dependencies: Vec<GlobalSymbolId>,
    ) -> Self {
        Self {
            key,
            space,
            source: ExportSource::ReExport(item),
            target,
            dependencies,
        }
    }
}
