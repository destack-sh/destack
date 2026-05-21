use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{DependencyItem, ExportSelector, LocalNodeId, LocalSymbolId, StaticKey};

/// One local global declaration from a symbol declared in the current module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalGlobalEntry {
    /// The global name.
    pub key: StaticKey,
    /// The local symbol exposed as a global.
    pub source: LocalSymbolId,
}

/// One named global re-export from another module.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IndirectGlobalEntry {
    /// The global name.
    pub key: StaticKey,
    /// The dependency item that declared the global export.
    pub item: LocalNodeId<DependencyItem>,
    /// The target module selected by the export.
    pub target: Option<ModuleId>,
    /// The export selected from the target module.
    pub imported: ExportSelector,
}

/// One global table entry.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GlobalEntry {
    /// A local global declaration.
    Local(LocalGlobalEntry),
    /// A re-exported global declaration.
    Indirect(IndirectGlobalEntry),
}

impl GlobalEntry {
    /// Return the global key.
    #[inline]
    pub fn key(self) -> StaticKey {
        match self {
            Self::Local(entry) => entry.key,
            Self::Indirect(entry) => entry.key,
        }
    }
}
