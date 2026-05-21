use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{LocalSymbolId, StaticKey};

/// Global declarations contributed by one module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalTable {
    /// The module id of the global table.
    pub module_id: ModuleId,
    /// Global symbols keyed by visible global name.
    pub symbols_by_key: IndexMap<StaticKey, Vec<LocalSymbolId>>,
}

impl GlobalTable {
    /// Create an empty global table.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            symbols_by_key: IndexMap::new(),
        }
    }

    /// Return true when the module contributes no globals.
    pub fn is_empty(&self) -> bool {
        self.symbols_by_key.is_empty()
    }

    /// Add one global declaration.
    pub fn push_symbol(&mut self, key: StaticKey, symbol: LocalSymbolId) {
        let symbols = self.symbols_by_key.entry(key).or_default();
        if !symbols.contains(&symbol) {
            symbols.push(symbol);
        }
    }

    /// Iterate global declarations in declaration order.
    pub fn symbols(&self) -> impl Iterator<Item = (&StaticKey, &[LocalSymbolId])> {
        self.symbols_by_key
            .iter()
            .map(|(key, symbols)| (key, symbols.as_slice()))
    }
}
