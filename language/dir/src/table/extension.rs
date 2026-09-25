use serde::{Deserialize, Serialize};
use tspp_core::FxIndexMap as IndexMap;
use tspp_serde::Reflect;
use tspp_source::ModuleId;

use crate::{GlobalSymbolId, LocalSymbolId};

/// Exported extensions resolved to their target root declaration.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ExtensionTable {
    /// The module id of the extension table.
    pub module_id: ModuleId,
    /// Exported extension symbols keyed to their resolved target root.
    pub target_by_symbol: IndexMap<LocalSymbolId, GlobalSymbolId>,
    /// Extension symbols keyed to the interfaces they implement.
    pub implementation_by_symbol: IndexMap<LocalSymbolId, ExtensionImplementation>,
}

/// One extension's resolved implementation.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ExtensionImplementation {
    /// The target root declaration, absent for blankets.
    pub root: Option<GlobalSymbolId>,
    /// The implemented interfaces.
    pub interfaces: Vec<GlobalSymbolId>,
}

impl ExtensionTable {
    /// Create an empty extension table.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            target_by_symbol: IndexMap::default(),
            implementation_by_symbol: IndexMap::default(),
        }
    }

    /// Insert one resolved extension target.
    pub fn insert(&mut self, symbol: LocalSymbolId, target: GlobalSymbolId) {
        self.target_by_symbol.insert(symbol, target);
    }

    /// Insert one extension's resolved implementation.
    pub fn insert_implementation(
        &mut self,
        symbol: LocalSymbolId,
        implementation: ExtensionImplementation,
    ) {
        self.implementation_by_symbol.insert(symbol, implementation);
    }

    /// Return true when no extension targets or implementations resolved.
    pub fn is_empty(&self) -> bool {
        self.target_by_symbol.is_empty() && self.implementation_by_symbol.is_empty()
    }

    /// Iterate extensions with their root and each interface they implement.
    pub fn implementations(
        &self,
    ) -> impl Iterator<Item = (GlobalSymbolId, Option<GlobalSymbolId>, GlobalSymbolId)> + '_ {
        self.implementation_by_symbol
            .iter()
            .flat_map(|(symbol, implementation)| {
                let symbol = (*symbol).into_global(self.module_id);
                implementation
                    .interfaces
                    .iter()
                    .map(move |interface| (symbol, implementation.root, *interface))
            })
    }

    /// Iterate exported extensions with their resolved target root.
    pub fn targets(&self) -> impl Iterator<Item = (GlobalSymbolId, GlobalSymbolId)> + '_ {
        self.target_by_symbol
            .iter()
            .map(|(symbol, target)| ((*symbol).into_global(self.module_id), *target))
    }
}
