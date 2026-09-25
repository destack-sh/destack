use crate::{ExportTarget, Postings};
use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// Indexed resolved named exports.
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
    pub fn build(indexes: &[&ExportIndex]) -> Self {
        let names = Postings::from_pairs((0..indexes.len()).flat_map(|ordinal| {
            let module = ordinal as u32;

            indexes[ordinal]
                .entries()
                .iter()
                .map(move |entry| (entry.name.clone(), module))
        }));

        Self { names }
    }

    /// Replace postings for one module export index.
    pub fn update(&mut self, module: u32, index: &ExportIndex) {
        self.names.replace(
            module,
            index.entries().iter().map(|entry| entry.name.clone()),
        );
    }
}

/// One indexed resolved named export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ExportEntry {
    /// The exposed name.
    pub name: String,
    /// The exact declaration selected by the exposed name.
    pub declaration: ExportTarget,
    /// The final exported target.
    pub target: ExportTarget,
}

impl ExportEntry {
    /// Compare two exports in stable display order.
    fn compare_by_display(&self, other: &Self) -> std::cmp::Ordering {
        let left = (self.name.as_str(), &self.declaration, &self.target);
        let right = (other.name.as_str(), &other.declaration, &other.target);

        left.cmp(&right)
    }
}
