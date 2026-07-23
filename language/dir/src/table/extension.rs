use destack_serde::Reflect;
use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{GlobalSymbolId, LocalSymbolId};

/// Exported extensions resolved to their target root declaration.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ExtensionTable {
    /// The module id of the extension table.
    pub module_id: ModuleId,
    /// Exported extension symbols keyed to their resolved target root.
    pub target_by_symbol: IndexMap<LocalSymbolId, GlobalSymbolId>,
}

impl ExtensionTable {
    /// Create an empty extension table.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            target_by_symbol: IndexMap::new(),
        }
    }

    /// Insert one resolved extension target.
    pub fn insert(&mut self, symbol: LocalSymbolId, target: GlobalSymbolId) {
        self.target_by_symbol.insert(symbol, target);
    }

    /// Return true when no extension targets resolved.
    pub fn is_empty(&self) -> bool {
        self.target_by_symbol.is_empty()
    }

    /// Iterate exported extensions with their resolved target root.
    pub fn targets(&self) -> impl Iterator<Item = (GlobalSymbolId, GlobalSymbolId)> + '_ {
        self.target_by_symbol
            .iter()
            .map(|(symbol, target)| ((*symbol).into_global(self.module_id), *target))
    }
}
