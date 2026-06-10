use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{GlobalEntry, IndirectGlobalEntry, LocalGlobalEntry, LocalSymbolId, StaticKey};

/// Global names contributed by one module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalTable {
    /// The module id of the global table.
    pub module_id: ModuleId,
    /// Global entries keyed by visible global name.
    pub entries_by_key: IndexMap<StaticKey, Vec<GlobalEntry>>,
}

impl GlobalTable {
    /// Create an empty global table.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            entries_by_key: IndexMap::new(),
        }
    }

    /// Return true when the module contributes no globals.
    pub fn is_empty(&self) -> bool {
        self.entries_by_key.is_empty()
    }

    /// Add one local global declaration.
    pub fn push_local(&mut self, key: StaticKey, symbol: LocalSymbolId) {
        self.push(GlobalEntry::Local(LocalGlobalEntry {
            key,
            source: symbol,
        }));
    }

    /// Add one global re-export.
    pub fn push_indirect(&mut self, entry: IndirectGlobalEntry) {
        self.push(GlobalEntry::Indirect(entry));
    }

    /// Add one global entry.
    pub fn push(&mut self, entry: GlobalEntry) {
        let entries = self.entries_by_key.entry(entry.key()).or_default();
        if !entries.contains(&entry) {
            entries.push(entry);
        }
    }

    /// Iterate global entries in declaration order.
    pub fn entries(&self) -> impl Iterator<Item = (&StaticKey, &[GlobalEntry])> {
        self.entries_by_key
            .iter()
            .map(|(key, entries)| (key, entries.as_slice()))
    }

    /// Return modules targeted by global re-export edges.
    pub fn reexport_modules(&self) -> impl Iterator<Item = ModuleId> + '_ {
        self.entries_by_key.values().flat_map(|entries| {
            entries.iter().filter_map(|entry| match entry {
                GlobalEntry::Local(_) => None,
                GlobalEntry::Indirect(entry) => entry.target,
            })
        })
    }
}
