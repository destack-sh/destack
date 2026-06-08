use destack_dir::GlobalSymbolId;
use serde::{Deserialize, Serialize};

/// Query index entries read from checked definitions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinitionIndex {
    /// Nominal relation entries ordered by target symbol.
    relations_by_target: Vec<NominalEntry>,
    /// Extension entries ordered by target symbol.
    extensions_by_target: Vec<ExtensionEntry>,
}

impl DefinitionIndex {
    /// Create a definition index from entries.
    pub fn new(relations: Vec<NominalEntry>, extensions: Vec<ExtensionEntry>) -> Self {
        let mut index = Self {
            relations_by_target: relations,
            extensions_by_target: extensions,
        };
        index.finish();

        index
    }

    /// Sort and deduplicate this index.
    pub fn finish(&mut self) {
        // order nominal relations by lookup target
        self.relations_by_target.sort_by_key(nominal_entry_key);
        self.relations_by_target.dedup();

        // order extension declarations by lookup target
        self.extensions_by_target.sort_by_key(extension_entry_key);
        self.extensions_by_target.dedup();
    }

    /// Return nominal relations that target one symbol.
    pub fn relations_for_target(&self, target_symbol: GlobalSymbolId) -> Vec<NominalEntry> {
        let range = relation_target_range(&self.relations_by_target, target_symbol);

        self.relations_by_target[range].to_vec()
    }

    /// Return extensions that target one symbol.
    pub fn extensions_for_target(&self, target_symbol: GlobalSymbolId) -> Vec<ExtensionEntry> {
        let range = extension_target_range(&self.extensions_by_target, target_symbol);

        self.extensions_by_target[range].to_vec()
    }

    /// Return all indexed nominal relation entries.
    pub fn relations(&self) -> &[NominalEntry] {
        &self.relations_by_target
    }

    /// Return all indexed extension entries.
    pub fn extensions(&self) -> &[ExtensionEntry] {
        &self.extensions_by_target
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

/// Extension declaration entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ExtensionEntry {
    /// The extension declaration symbol.
    pub extension_symbol: GlobalSymbolId,
    /// The canonical target symbol.
    pub target_symbol: GlobalSymbolId,
}

/// Return the stored relation range for one target symbol.
fn relation_target_range(
    entries: &[NominalEntry],
    target_symbol: GlobalSymbolId,
) -> std::ops::Range<usize> {
    // find the first matching relation
    let start = entries.partition_point(|entry| entry.target_symbol < target_symbol);

    // find the end of the matching run
    let end =
        entries[start..].partition_point(|entry| entry.target_symbol == target_symbol) + start;

    start..end
}

/// Return the stored extension range for one target symbol.
fn extension_target_range(
    entries: &[ExtensionEntry],
    target_symbol: GlobalSymbolId,
) -> std::ops::Range<usize> {
    // find the first matching extension
    let start = entries.partition_point(|entry| entry.target_symbol < target_symbol);

    // find the end of the matching run
    let end =
        entries[start..].partition_point(|entry| entry.target_symbol == target_symbol) + start;

    start..end
}

/// Return the stable ordering key for one nominal entry.
fn nominal_entry_key(entry: &NominalEntry) -> (GlobalSymbolId, NominalRelation, GlobalSymbolId) {
    (entry.target_symbol, entry.relation, entry.source_symbol)
}

/// Return the stable ordering key for one extension entry.
fn extension_entry_key(entry: &ExtensionEntry) -> (GlobalSymbolId, GlobalSymbolId) {
    (entry.target_symbol, entry.extension_symbol)
}
