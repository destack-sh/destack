use destack_core::FxIndexMap as IndexMap;
use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};
use smallvec::smallvec;

use crate::{
    DependencyItem, ExportBinding, ExportSelector, GlobalEntry, LocalNodeId, LocalSymbolId,
    StaticKey,
};

/// Global names contributed by one module.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
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
            entries_by_key: IndexMap::default(),
        }
    }

    /// Return true when the module contributes no globals.
    pub fn is_empty(&self) -> bool {
        self.entries_by_key.is_empty()
    }

    /// Add one local global declaration.
    pub fn push_local(&mut self, key: StaticKey, symbol: LocalSymbolId) {
        self.push(GlobalEntry {
            key,
            item: None,
            alias: None,
            binding: ExportBinding::Local {
                symbols: smallvec![symbol],
            },
        });
    }

    /// Add one global re-export.
    pub fn push_indirect(
        &mut self,
        key: StaticKey,
        item: LocalNodeId<DependencyItem>,
        alias: Option<LocalSymbolId>,
        target: Option<ModuleId>,
        imported: ExportSelector,
    ) {
        self.push(GlobalEntry {
            key,
            item: Some(item),
            alias,
            binding: ExportBinding::Import {
                module: target,
                selector: imported,
            },
        });
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
        self.entries_by_key
            .values()
            .flat_map(|entries| entries.iter().filter_map(|entry| entry.target_module()))
    }
}
