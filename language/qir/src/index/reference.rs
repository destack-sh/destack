use destack_dir::GlobalSymbolId;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

/// Reference target membership index.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceIndex {
    /// The reference entries ordered by target symbol.
    by_target: Vec<ReferenceEntry>,
}

impl ReferenceIndex {
    /// Create a reference index from entries.
    pub fn new(entries: Vec<ReferenceEntry>) -> Self {
        let mut index = Self { by_target: entries };
        index.finish();

        index
    }

    /// Sort and deduplicate this index.
    pub fn finish(&mut self) {
        self.by_target.sort();
        self.by_target.dedup();
    }

    /// Return modules that may reference one target symbol.
    pub fn modules(&self, target_symbol: GlobalSymbolId) -> Vec<ModuleId> {
        let range = self.target_range(target_symbol);

        self.by_target[range]
            .iter()
            .map(|entry| entry.module_id)
            .collect()
    }

    /// Return all indexed reference entries.
    pub fn entries(&self) -> &[ReferenceEntry] {
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

/// One module's membership in one reference target set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ReferenceEntry {
    /// The referenced symbol.
    pub target_symbol: GlobalSymbolId,
    /// The module that may reference the symbol.
    pub module_id: ModuleId,
}
