use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{GlobalSymbolId, LocalSymbolId, StaticKey};

/// Resolved import targets for one module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportTable {
    /// The module id of the import table.
    pub module_id: ModuleId,
    /// Modules reached by resolved imports and active globals.
    pub dependencies: Vec<ModuleId>,
    /// Imported local symbols keyed to their resolved target.
    pub target_by_symbol: IndexMap<LocalSymbolId, ImportTarget>,
    /// Global symbols made visible by the active profile.
    pub global_symbol_by_key: IndexMap<StaticKey, Vec<GlobalSymbolId>>,
}

impl ImportTable {
    /// Create an empty import table.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            dependencies: Vec::new(),
            target_by_symbol: IndexMap::new(),
            global_symbol_by_key: IndexMap::new(),
        }
    }

    /// Add one resolved dependency.
    pub fn push_dependency(&mut self, module: ModuleId) {
        if !self.dependencies.contains(&module) {
            self.dependencies.push(module);
        }
    }

    /// Insert one resolved local import symbol target.
    pub fn insert_symbol(&mut self, symbol: LocalSymbolId, target: ImportTarget) {
        self.target_by_symbol.insert(symbol, target);
    }

    /// Add one imported global symbol.
    pub fn push_global_symbol(&mut self, key: StaticKey, symbol: GlobalSymbolId) {
        let symbols = self.global_symbol_by_key.entry(key).or_default();
        if !symbols.contains(&symbol) {
            symbols.push(symbol);
        }
    }

    /// Return one resolved local import symbol target.
    pub fn symbol_target(&self, symbol: LocalSymbolId) -> Option<ImportTarget> {
        self.target_by_symbol.get(&symbol).copied()
    }

    /// Return imported global symbols for one key.
    pub fn global_symbols(&self, key: StaticKey) -> Option<&[GlobalSymbolId]> {
        self.global_symbol_by_key.get(&key).map(Vec::as_slice)
    }
}

/// Target selected by one import binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ImportTarget {
    /// A symbol exported by a target module.
    Symbol(GlobalSymbolId),
    /// A namespace object for a target module.
    Namespace(ModuleId),
}
