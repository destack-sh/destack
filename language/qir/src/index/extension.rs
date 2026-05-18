use destack_dir::{GlobalSymbolId, LocalExtensionId};
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

/// Extension declaration index.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionIndex {
    /// The extension entries ordered by target symbol.
    by_target: Vec<ExtensionEntry>,
}

impl ExtensionIndex {
    /// Create an extension index from entries.
    pub fn new(entries: Vec<ExtensionEntry>) -> Self {
        let mut index = Self { by_target: entries };
        index.finish();

        index
    }

    /// Sort and deduplicate this index.
    pub fn finish(&mut self) {
        self.by_target.sort_by_key(extension_entry_key);
        self.by_target.dedup();
    }

    /// Return extensions that target one symbol.
    pub fn for_target(&self, target_symbol: GlobalSymbolId) -> Vec<ExtensionEntry> {
        let range = self.target_range(target_symbol);

        self.by_target[range].to_vec()
    }

    /// Return all indexed extension entries.
    pub fn entries(&self) -> &[ExtensionEntry] {
        &self.by_target
    }

    /// Return the stored range for one target symbol.
    fn target_range(&self, target_symbol: GlobalSymbolId) -> std::ops::Range<usize> {
        let start = self
            .by_target
            .partition_point(|entry| entry.target_symbol < target_symbol);
        let end = self.by_target[start..]
            .partition_point(|entry| entry.target_symbol == target_symbol)
            + start;

        start..end
    }
}

/// Extension declaration entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ExtensionEntry {
    /// The module that declares the extension.
    pub module_id: ModuleId,
    /// The local extension id.
    pub extension_id: LocalExtensionId,
    /// The canonical target symbol.
    pub target_symbol: GlobalSymbolId,
}

/// Return the stable ordering key for one extension entry.
fn extension_entry_key(entry: &ExtensionEntry) -> (GlobalSymbolId, ModuleId, LocalExtensionId) {
    (entry.target_symbol, entry.module_id, entry.extension_id)
}
