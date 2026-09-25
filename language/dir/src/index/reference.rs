use std::slice::Iter;

use crate::{GlobalNodeIdAny, GlobalSymbolId, Postings};
use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;
use tspp_source::Span;

/// Reference occurrence index.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReferenceIndex {
    /// The references ordered by target symbol.
    by_target: Vec<ReferenceEntry>,
    /// The references ordered by declaration symbol.
    by_declaration: Vec<ReferenceEntry>,
}

/// Reference postings by target and declaration symbols.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct ReferencePostings {
    /// Reference target postings.
    pub targets: Postings<GlobalSymbolId>,
    /// Reference declaration postings.
    pub declarations: Postings<GlobalSymbolId>,
}

impl ReferenceIndex {
    /// Create a reference index from entries.
    pub fn new(
        target_entries: Vec<ReferenceEntry>,
        declaration_entries: Vec<ReferenceEntry>,
    ) -> Self {
        let mut index = Self {
            by_target: target_entries,
            by_declaration: declaration_entries,
        };
        index.finish();

        index
    }

    /// Sort and deduplicate this index.
    pub fn finish(&mut self) {
        self.by_target.sort_by(ReferenceEntry::compare_by_symbol);
        self.by_target.dedup();
        self.by_declaration
            .sort_by(ReferenceEntry::compare_by_symbol);
        self.by_declaration.dedup();
    }

    /// Iterate references to one target symbol.
    pub fn target_entries(&self, target: GlobalSymbolId) -> Iter<'_, ReferenceEntry> {
        let range = Self::entry_range(&self.by_target, target);

        self.by_target[range].iter()
    }

    /// Iterate references to one declaration symbol.
    pub fn declaration_entries(&self, declaration: GlobalSymbolId) -> Iter<'_, ReferenceEntry> {
        let range = Self::entry_range(&self.by_declaration, declaration);

        self.by_declaration[range].iter()
    }

    /// Return all indexed target references.
    pub fn target_references(&self) -> &[ReferenceEntry] {
        &self.by_target
    }

    /// Return all indexed declaration references.
    pub fn declaration_references(&self) -> &[ReferenceEntry] {
        &self.by_declaration
    }

    /// Return the stored range for one symbol.
    fn entry_range(entries: &[ReferenceEntry], symbol: GlobalSymbolId) -> std::ops::Range<usize> {
        let start = entries.partition_point(|entry| entry.symbol < symbol);
        let end = entries[start..].partition_point(|entry| entry.symbol == symbol) + start;

        start..end
    }
}

impl ReferencePostings {
    /// Build reference postings from module index sections.
    pub fn build(indexes: &[&ReferenceIndex]) -> Self {
        let targets = Postings::from_pairs((0..indexes.len()).flat_map(|ordinal| {
            let module = ordinal as u32;

            indexes[ordinal]
                .target_references()
                .iter()
                .map(move |entry| (entry.symbol, module))
        }));
        let declarations = Postings::from_pairs((0..indexes.len()).flat_map(|ordinal| {
            let module = ordinal as u32;

            indexes[ordinal]
                .declaration_references()
                .iter()
                .map(move |entry| (entry.symbol, module))
        }));

        Self {
            targets,
            declarations,
        }
    }

    /// Replace postings for one module reference index.
    pub fn update(&mut self, module: u32, index: &ReferenceIndex) {
        self.targets.replace(
            module,
            index.target_references().iter().map(|entry| entry.symbol),
        );
        self.declarations.replace(
            module,
            index
                .declaration_references()
                .iter()
                .map(|entry| entry.symbol),
        );
    }
}

/// One indexed reference occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReferenceEntry {
    /// The symbol indexing this occurrence.
    pub symbol: GlobalSymbolId,
    /// The DIR node carrying this occurrence.
    pub source: GlobalNodeIdAny,
    /// The source range.
    pub span: Span,
    /// Whether the authored occurrence names an alias instead of the target.
    pub is_alias: bool,
}

impl ReferenceEntry {
    /// Compare two references in symbol lookup order.
    fn compare_by_symbol(&self, other: &Self) -> std::cmp::Ordering {
        let left = (
            self.symbol,
            self.span.file,
            self.span.start,
            self.span.end,
            self.source,
            self.is_alias,
        );
        let right = (
            other.symbol,
            other.span.file,
            other.span.start,
            other.span.end,
            other.source,
            other.is_alias,
        );

        left.cmp(&right)
    }
}
