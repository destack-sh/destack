use crate::{GlobalNodeIdAny, GlobalSymbolId, Postings, SymbolKind};
use destack_serde::Reflect;
use destack_source::{FileId, Span};
use serde::{Deserialize, Serialize};

/// Indexed exported symbols.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ExportIndex {
    /// The exports in stable display order.
    entries: Vec<ExportEntry>,
}

/// Export postings by name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct ExportPostings {
    /// Export name postings.
    pub names: Postings<String>,
}

impl ExportIndex {
    /// Create an export index from entries.
    pub fn new(entries: Vec<ExportEntry>) -> Self {
        let mut index = Self { entries };
        index.finish();

        index
    }

    /// Sort and deduplicate this index.
    pub fn finish(&mut self) {
        self.entries.sort_by(ExportEntry::compare_by_display);
        self.entries.dedup();
    }

    /// Return all indexed exports.
    pub fn entries(&self) -> &[ExportEntry] {
        &self.entries
    }
}

impl ExportPostings {
    /// Build export postings from module index sections.
    pub fn new(indexes: &[&ExportIndex]) -> Self {
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

/// One indexed exported symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ExportEntry {
    /// The exported symbol name.
    pub name: String,
    /// The exported symbol kind.
    pub kind: SymbolKind,
    /// The resolved exported symbol.
    pub symbol: GlobalSymbolId,
    /// The source node that exposes this export.
    pub source: GlobalNodeIdAny,
    /// The source file.
    pub file: FileId,
    /// The source range.
    pub span: Span,
    /// The module path to use in imports.
    pub module_path: Option<String>,
}

impl ExportEntry {
    /// Compare two exports in stable display order.
    fn compare_by_display(&self, other: &Self) -> std::cmp::Ordering {
        let left = (
            self.name.as_str(),
            self.source.module_id,
            self.file,
            self.span.start,
            self.span.end,
            self.symbol,
        );
        let right = (
            other.name.as_str(),
            other.source.module_id,
            other.file,
            other.span.start,
            other.span.end,
            other.symbol,
        );

        left.cmp(&right)
    }
}
