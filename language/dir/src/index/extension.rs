use crate::{ExtensionForm, GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, Postings};
use destack_serde::Reflect;
use destack_source::{FileId, Span};
use serde::{Deserialize, Serialize};

/// Indexed checked extensions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ExtensionIndex {
    /// Extensions ordered by root symbol.
    by_root: Vec<ExtensionEntry>,
}

/// Extension postings by root symbol.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct ExtensionPostings {
    /// Rooted extension postings.
    pub roots: Postings<GlobalSymbolId>,
    /// Modules that contain blanket extensions.
    pub blankets: Vec<u32>,
}

impl ExtensionIndex {
    /// Create an extension index from entries.
    pub fn new(entries: Vec<ExtensionEntry>) -> Self {
        let mut index = Self { by_root: entries };
        index.finish();

        index
    }

    /// Sort and deduplicate this index.
    pub fn finish(&mut self) {
        self.by_root.sort_by(ExtensionEntry::compare_by_root);
        self.by_root.dedup();
    }

    /// Iterate extensions rooted at one symbol.
    pub fn root_entries(&self, root: GlobalSymbolId) -> impl Iterator<Item = &ExtensionEntry> {
        let range = self.root_range(root);

        self.by_root[range].iter()
    }

    /// Iterate blanket extensions.
    pub fn blankets(&self) -> impl Iterator<Item = &ExtensionEntry> {
        self.by_root.iter().filter(|entry| entry.root.is_none())
    }

    /// Return all indexed extensions.
    pub fn entries(&self) -> &[ExtensionEntry] {
        &self.by_root
    }

    /// Return the stored range for one root symbol.
    fn root_range(&self, root: GlobalSymbolId) -> std::ops::Range<usize> {
        let start = self
            .by_root
            .partition_point(|entry| entry.root < Some(root));
        let end = self.by_root[start..].partition_point(|entry| entry.root == Some(root)) + start;

        start..end
    }
}

impl ExtensionPostings {
    /// Build extension postings from module index sections.
    pub fn build(indexes: &[&ExtensionIndex]) -> Self {
        let roots = Postings::from_pairs((0..indexes.len()).flat_map(|ordinal| {
            let module = ordinal as u32;

            indexes[ordinal]
                .entries()
                .iter()
                .filter_map(move |entry| entry.root.map(|root| (root, module)))
        }));
        let mut blankets = (0..indexes.len())
            .filter_map(|ordinal| {
                indexes[ordinal]
                    .entries()
                    .iter()
                    .any(|entry| entry.root.is_none())
                    .then_some(ordinal as u32)
            })
            .collect::<Vec<_>>();

        blankets.sort();
        blankets.dedup();

        Self { roots, blankets }
    }
}

/// One indexed extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ExtensionEntry {
    /// The extension declaration symbol.
    pub declaration: GlobalSymbolId,
    /// The source extension declaration node.
    pub source: GlobalNodeIdAny,
    /// The source file.
    pub file: FileId,
    /// The source range.
    pub span: Span,
    /// The canonical lookup root when this is a rooted extension.
    pub root: Option<GlobalSymbolId>,
    /// The checked receiver type.
    pub ty: GlobalTypeId,
    /// The extension form.
    pub form: ExtensionForm,
}

impl ExtensionEntry {
    /// Compare two extensions in root lookup order.
    fn compare_by_root(&self, other: &Self) -> std::cmp::Ordering {
        let left = (
            self.root,
            self.declaration,
            self.file,
            self.span.start,
            self.span.end,
            self.ty,
        );
        let right = (
            other.root,
            other.declaration,
            other.file,
            other.span.start,
            other.span.end,
            other.ty,
        );

        left.cmp(&right)
    }
}
