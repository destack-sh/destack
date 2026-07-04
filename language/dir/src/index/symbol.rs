use crate::{
    GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, Mutability, Postings, SymbolKind, SymbolRole,
};
use destack_serde::Reflect;
use destack_source::{FileId, Span};
use serde::{Deserialize, Serialize};

/// Indexed declared symbols.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SymbolIndex {
    /// The symbols in stable display order.
    entries: Vec<SymbolEntry>,
}

/// Symbol postings by name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct SymbolPostings {
    /// Symbol name postings.
    pub names: Postings<String>,
}

/// One indexed declared symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SymbolEntry {
    /// The display name.
    pub name: String,
    /// The checked symbol kind.
    pub kind: SymbolKind,
    /// The checked symbol role.
    pub role: SymbolRole,
    /// The symbol id.
    pub symbol: GlobalSymbolId,
    /// The source node that declares the symbol.
    pub source: GlobalNodeIdAny,
    /// The source file.
    pub file: FileId,
    /// The source range.
    pub span: Span,
    /// The checked type of the symbol when known.
    pub ty: Option<GlobalTypeId>,
    /// The containing declaration display name.
    pub container: Option<String>,
    /// The binding mutability when this is a value binding.
    pub mutability: Option<Mutability>,
    /// Whether this symbol is exported from its declaring module.
    pub is_exported: bool,
}

impl SymbolIndex {
    /// Create a symbol index from entries.
    pub fn new(entries: Vec<SymbolEntry>) -> Self {
        let mut index = Self { entries };
        index.finish();

        index
    }

    /// Sort and deduplicate this index.
    pub fn finish(&mut self) {
        self.entries.sort_by(SymbolEntry::compare_by_display);
        self.entries.dedup();
    }

    /// Return all indexed symbols.
    pub fn entries(&self) -> &[SymbolEntry] {
        &self.entries
    }
}

impl SymbolPostings {
    /// Build symbol postings from module index sections.
    pub fn build(indexes: &[&SymbolIndex]) -> Self {
        let names = Postings::from_pairs((0..indexes.len()).flat_map(|ordinal| {
            let module = ordinal as u32;

            indexes[ordinal]
                .entries()
                .iter()
                .map(move |entry| (entry.name.clone(), module))
        }));

        Self { names }
    }
}

impl SymbolEntry {
    /// Compare two symbols in stable display order.
    fn compare_by_display(&self, other: &Self) -> std::cmp::Ordering {
        let left = (
            self.name.as_str(),
            self.kind,
            self.source.module_id,
            self.file,
            self.span.start,
            self.span.end,
            self.symbol,
        );
        let right = (
            other.name.as_str(),
            other.kind,
            other.source.module_id,
            other.file,
            other.span.start,
            other.span.end,
            other.symbol,
        );

        left.cmp(&right)
    }
}
