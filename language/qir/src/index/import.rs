use destack_dir::{LocalSymbolId, SymbolKind, SymbolSpace};
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

/// Importable export index.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportIndex {
    /// The import entries in stable display order.
    entries: Vec<ImportEntry>,
}

impl ImportIndex {
    /// Create an import index from entries.
    pub fn new(entries: Vec<ImportEntry>) -> Self {
        let mut index = Self { entries };
        index.finish();

        index
    }

    /// Sort and deduplicate this index.
    pub fn finish(&mut self) {
        self.entries
            .sort_by(|left, right| import_entry_key(left).cmp(&import_entry_key(right)));
        self.entries.dedup();
    }

    /// Return entries whose export name contains the case-insensitive query text.
    pub fn search(&self, query: &str, exclude_module: Option<ModuleId>) -> Vec<ImportEntry> {
        let query = query.to_lowercase();

        self.entries
            .iter()
            .filter(|entry| Some(entry.module_id) != exclude_module)
            .filter(|entry| query.is_empty() || entry.name.to_lowercase().contains(&query))
            .cloned()
            .collect()
    }

    /// Return all indexed import entries.
    pub fn entries(&self) -> &[ImportEntry] {
        &self.entries
    }
}

/// Importable exported symbol entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportEntry {
    /// The exported symbol name.
    pub name: String,
    /// The exported symbol kind.
    pub kind: SymbolKind,
    /// The exported symbol space.
    pub space: SymbolSpace,
    /// The module that exports this symbol.
    pub module_id: ModuleId,
    /// The local exported symbol id.
    pub local_id: LocalSymbolId,
    /// The module path to use in imports.
    pub module_path: Option<String>,
}

/// Return the stable ordering key for one import entry.
fn import_entry_key(entry: &ImportEntry) -> (&str, ModuleId, LocalSymbolId) {
    (entry.name.as_str(), entry.module_id, entry.local_id)
}
