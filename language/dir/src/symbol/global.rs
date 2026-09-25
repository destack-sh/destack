use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;
use tspp_source::ModuleId;

use crate::{DependencyItem, ExportBinding, LocalNodeId, LocalSymbolId, StaticKey};

/// One global table entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GlobalEntry {
    /// The visible global name.
    pub key: StaticKey,
    /// The dependency item that declared this global name.
    pub item: Option<LocalNodeId<DependencyItem>>,
    /// The local declaration selected by the global name.
    pub declaration: Option<LocalSymbolId>,
    /// How the global name is bound.
    pub binding: ExportBinding,
}

impl GlobalEntry {
    /// Return the global key.
    #[inline]
    pub fn key(&self) -> StaticKey {
        self.key
    }

    /// Return the target module selected by an indirect global.
    #[inline]
    pub fn target_module(&self) -> Option<ModuleId> {
        self.binding.target_module()
    }
}
