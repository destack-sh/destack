use crate::{GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, Postings};
use destack_serde::Reflect;
use destack_source::Span;
use serde::{Deserialize, Serialize};

/// Indexed nominal heritage edges.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct HeritageIndex {
    /// Heritage edges ordered by base symbol.
    by_base: Vec<HeritageEntry>,
    /// Heritage edges ordered by derived symbol.
    by_derived: Vec<HeritageEntry>,
}

/// Heritage postings by base symbol.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct HeritagePostings {
    /// Heritage base postings.
    pub bases: Postings<GlobalSymbolId>,
    /// Heritage derived postings.
    pub derived: Postings<GlobalSymbolId>,
}

impl HeritageIndex {
    /// Create a heritage index from edges.
    pub fn new(entries: Vec<HeritageEntry>) -> Self {
        let mut index = Self {
            by_base: entries.clone(),
            by_derived: entries,
        };
        index.finish();

        index
    }

    /// Sort and deduplicate this index.
    pub fn finish(&mut self) {
        self.by_base.sort_by(HeritageEntry::compare_by_base);
        self.by_base.dedup();

        self.by_derived.sort_by(HeritageEntry::compare_by_derived);
        self.by_derived.dedup();
    }

    /// Iterate heritage edges that target one base symbol.
    pub fn base_entries(&self, base: GlobalSymbolId) -> impl Iterator<Item = &HeritageEntry> {
        let range = self.base_range(base);

        self.by_base[range].iter()
    }

    /// Return all indexed heritage edges.
    pub fn entries(&self) -> &[HeritageEntry] {
        &self.by_base
    }

    /// Iterate heritage edges declared by one derived symbol.
    pub fn derived_entries(&self, derived: GlobalSymbolId) -> impl Iterator<Item = &HeritageEntry> {
        let range = self.derived_range(derived);

        self.by_derived[range].iter()
    }

    /// Return the stored range for one base symbol.
    fn base_range(&self, base: GlobalSymbolId) -> std::ops::Range<usize> {
        let start = self.by_base.partition_point(|entry| entry.base < base);
        let end = self.by_base[start..].partition_point(|entry| entry.base == base) + start;

        start..end
    }

    /// Return the stored range for one derived symbol.
    fn derived_range(&self, derived: GlobalSymbolId) -> std::ops::Range<usize> {
        let start = self
            .by_derived
            .partition_point(|entry| entry.derived < derived);
        let end =
            self.by_derived[start..].partition_point(|entry| entry.derived == derived) + start;

        start..end
    }
}

impl HeritagePostings {
    /// Build heritage postings from module index sections.
    pub fn build(indexes: &[&HeritageIndex]) -> Self {
        let bases = Postings::from_pairs((0..indexes.len()).flat_map(|ordinal| {
            let module = ordinal as u32;

            indexes[ordinal]
                .entries()
                .iter()
                .map(move |entry| (entry.base, module))
        }));
        let derived = Postings::from_pairs((0..indexes.len()).flat_map(|ordinal| {
            let module = ordinal as u32;

            indexes[ordinal]
                .entries()
                .iter()
                .map(move |entry| (entry.derived, module))
        }));

        Self { bases, derived }
    }

    /// Replace postings for one module heritage index.
    pub fn update(&mut self, module: u32, index: &HeritageIndex) {
        self.bases
            .replace(module, index.entries().iter().map(|entry| entry.base));
        self.derived
            .replace(module, index.entries().iter().map(|entry| entry.derived));
    }
}

/// One nominal heritage edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct HeritageEntry {
    /// The derived nominal symbol.
    pub derived: GlobalSymbolId,
    /// The declaration that authored the heritage edge.
    pub declaration: GlobalSymbolId,
    /// The inherited or implemented nominal symbol.
    pub base: GlobalSymbolId,
    /// The source heritage node.
    pub source: GlobalNodeIdAny,
    /// The applied heritage type.
    pub ty: GlobalTypeId,
    /// The authored heritage range.
    pub span: Span,
    /// The heritage kind.
    pub kind: HeritageKind,
}

impl HeritageEntry {
    /// Compare two heritage edges in base lookup order.
    fn compare_by_base(&self, other: &Self) -> std::cmp::Ordering {
        let left = (
            self.base,
            self.span.file,
            self.span.start,
            self.span.end,
            self.kind,
            self.derived,
            self.declaration,
        );
        let right = (
            other.base,
            other.span.file,
            other.span.start,
            other.span.end,
            other.kind,
            other.derived,
            other.declaration,
        );

        left.cmp(&right)
    }

    /// Compare two heritage edges in derived lookup order.
    fn compare_by_derived(&self, other: &Self) -> std::cmp::Ordering {
        let left = (
            self.derived,
            self.span.file,
            self.span.start,
            self.span.end,
            self.kind,
            self.base,
            self.declaration,
        );
        let right = (
            other.derived,
            other.span.file,
            other.span.start,
            other.span.end,
            other.kind,
            other.base,
            other.declaration,
        );

        left.cmp(&right)
    }
}

/// Nominal heritage kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
pub enum HeritageKind {
    /// Inheritance edge.
    Extends,
    /// Interface implementation edge.
    Implements,
}
