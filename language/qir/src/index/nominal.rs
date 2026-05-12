use destack_dir::GlobalSymbolId;
use serde::{Deserialize, Serialize};

/// Nominal relation index.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NominalIndex {
    /// The nominal relation entries ordered by target symbol.
    by_target: Vec<NominalEntry>,
}

impl NominalIndex {
    /// Create a nominal index from entries.
    pub fn new(entries: Vec<NominalEntry>) -> Self {
        let mut index = Self { by_target: entries };
        index.finish();

        index
    }

    /// Sort and deduplicate this index.
    pub fn finish(&mut self) {
        self.by_target.sort_by_key(nominal_entry_key);
        self.by_target.dedup();
    }

    /// Return nominal relations that target one symbol.
    pub fn to(&self, target_symbol: GlobalSymbolId) -> Vec<NominalEntry> {
        let range = self.target_range(target_symbol);

        self.by_target[range].to_vec()
    }

    /// Return all indexed nominal relation entries.
    pub fn entries(&self) -> &[NominalEntry] {
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

/// Nominal relation entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NominalEntry {
    /// The source nominal symbol.
    pub source_symbol: GlobalSymbolId,
    /// The target nominal symbol.
    pub target_symbol: GlobalSymbolId,
    /// The relation kind.
    pub relation: NominalRelation,
}

/// Nominal relation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NominalRelation {
    /// Inheritance edge.
    Extends,
    /// Interface implementation edge.
    Implements,
}

/// Return the stable ordering key for one nominal entry.
fn nominal_entry_key(entry: &NominalEntry) -> (GlobalSymbolId, NominalRelation, GlobalSymbolId) {
    (entry.target_symbol, entry.relation, entry.source_symbol)
}
