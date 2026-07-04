use crate::{GlobalNodeIdAny, GlobalSymbolId, Postings};
use destack_serde::Reflect;
use destack_source::{ModuleId, Span};
use serde::{Deserialize, Serialize};

/// Reference occurrence index.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReferenceIndex {
    /// The references ordered by target symbol.
    by_target: Vec<ReferenceEntry>,
}

/// Reference postings by target symbol.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct ReferencePostings {
    /// Reference target postings.
    pub targets: Postings<GlobalSymbolId>,
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
        self.by_target.sort_by(ReferenceEntry::compare_by_target);
        self.by_target.dedup();
    }

    /// Iterate modules that may reference one target symbol.
    pub fn modules(&self, target: GlobalSymbolId) -> impl Iterator<Item = ModuleId> + '_ {
        let range = self.target_range(target);

        self.by_target[range]
            .iter()
            .map(|entry| entry.source.module_id)
    }

    /// Iterate references to one target symbol.
    pub fn target_entries(&self, target: GlobalSymbolId) -> impl Iterator<Item = &ReferenceEntry> {
        let range = self.target_range(target);

        self.by_target[range].iter()
    }

    /// Return all indexed references.
    pub fn entries(&self) -> &[ReferenceEntry] {
        &self.by_target
    }

    /// Return the stored range for one target symbol.
    fn target_range(&self, target: GlobalSymbolId) -> std::ops::Range<usize> {
        let start = self
            .by_target
            .partition_point(|entry| entry.target < target);
        let end = self.by_target[start..].partition_point(|entry| entry.target == target) + start;

        start..end
    }
}

impl ReferencePostings {
    /// Build reference postings from module index sections.
    pub fn build(indexes: &[&ReferenceIndex]) -> Self {
        let targets = Postings::from_pairs((0..indexes.len()).flat_map(|ordinal| {
            let module = ordinal as u32;

            indexes[ordinal]
                .entries()
                .iter()
                .map(move |entry| (entry.target, module))
        }));

        Self { targets }
    }
}

/// One indexed reference occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReferenceEntry {
    /// The referenced symbol.
    pub target: GlobalSymbolId,
    /// The source node that references the symbol.
    pub source: GlobalNodeIdAny,
    /// The source range.
    pub span: Span,
    /// The reference kind.
    pub kind: ReferenceKind,
}

/// Kind of indexed reference occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
pub enum ReferenceKind {
    /// Name or path reference.
    Name,
    /// Member reference.
    Member,
    /// Call target reference.
    Call,
    /// Construct target reference.
    Construct,
    /// Import or export dependency reference.
    Dependency,
    /// Type reference.
    Type,
    /// Storage selection reference.
    Storage,
}

impl ReferenceEntry {
    /// Compare two references in target lookup order.
    fn compare_by_target(&self, other: &Self) -> std::cmp::Ordering {
        let left = (
            self.target,
            self.source.module_id,
            self.source.local_id.id,
            self.kind,
            self.span.file,
            self.span.start,
            self.span.end,
        );
        let right = (
            other.target,
            other.source.module_id,
            other.source.local_id.id,
            other.kind,
            other.span.file,
            other.span.start,
            other.span.end,
        );

        left.cmp(&right)
    }
}
