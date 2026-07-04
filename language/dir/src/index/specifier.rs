use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::{GlobalNodeIdAny, Postings};
use destack_serde::Reflect;
use destack_source::{FileId, ModuleId, Span};
use serde::{Deserialize, Serialize};

/// Module specifier rewrite index.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SpecifierIndex {
    /// The resolved specifiers ordered by target path.
    by_target_path: Vec<(PathBuf, SpecifierEntry)>,
    /// The specifiers without a resolved target path.
    unresolved: Vec<SpecifierEntry>,
}

/// Module specifier postings by resolved target path.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct SpecifierPostings {
    /// Resolved target path postings.
    pub paths: Postings<PathBuf>,
    /// Modules that contain unresolved specifiers.
    pub unresolved: Vec<u32>,
}

impl SpecifierIndex {
    /// Create a specifier index from entries.
    pub fn new(entries: Vec<SpecifierEntry>) -> Self {
        let mut index = Self::default();

        for entry in entries {
            index.push(entry);
        }

        index.finish();

        index
    }

    /// Sort and deduplicate this index.
    pub fn finish(&mut self) {
        self.by_target_path.sort_by(|left, right| {
            let left = (
                &left.0,
                left.1.source.module_id,
                left.1.file,
                left.1.source.local_id.id,
                left.1.text.as_str(),
            );
            let right = (
                &right.0,
                right.1.source.module_id,
                right.1.file,
                right.1.source.local_id.id,
                right.1.text.as_str(),
            );

            left.cmp(&right)
        });
        self.by_target_path.dedup();

        self.unresolved
            .sort_by(|left, right| left.compare_by_source(right));
        self.unresolved.dedup();
    }

    /// Return specifiers whose target path is absent or included in the rename set.
    pub fn renaming<'a>(
        &'a self,
        old_paths: &'a HashSet<PathBuf>,
    ) -> impl Iterator<Item = &'a SpecifierEntry> + 'a {
        let mut entries = Vec::new();

        for old_path in old_paths {
            entries.extend(self.path_entries(old_path));
        }

        entries.extend(self.unresolved.iter());
        entries.sort_by(|left, right| left.compare_by_source(right));
        entries.dedup();

        entries.into_iter()
    }

    /// Return all indexed resolved specifiers.
    pub fn resolved(&self) -> impl Iterator<Item = &SpecifierEntry> {
        self.by_target_path.iter().map(|(_, entry)| entry)
    }

    /// Return all indexed unresolved specifiers.
    pub fn unresolved(&self) -> &[SpecifierEntry] {
        &self.unresolved
    }

    /// Add one specifier entry to its lookup bucket.
    fn push(&mut self, entry: SpecifierEntry) {
        match entry.target_path.clone() {
            Some(path) => self.by_target_path.push((path, entry)),
            None => self.unresolved.push(entry),
        }
    }

    /// Return entries that target one resolved path.
    fn path_entries<'a>(
        &'a self,
        target_path: &'a Path,
    ) -> impl Iterator<Item = &'a SpecifierEntry> + 'a {
        let start = self
            .by_target_path
            .partition_point(|(path, _)| path.as_path() < target_path);
        let end = self.by_target_path[start..]
            .partition_point(|(path, _)| path.as_path() == target_path)
            + start;

        self.by_target_path[start..end]
            .iter()
            .map(|(_, entry)| entry)
    }
}

impl SpecifierPostings {
    /// Build specifier postings from module index sections.
    pub fn new(indexes: &[&SpecifierIndex]) -> Self {
        let paths = Postings::from_pairs((0..indexes.len()).flat_map(|ordinal| {
            let module = ordinal as u32;

            indexes[ordinal]
                .resolved()
                .filter_map(move |entry| entry.target_path.clone().map(|path| (path, module)))
        }));
        let mut unresolved = (0..indexes.len())
            .filter_map(|ordinal| {
                (!indexes[ordinal].unresolved().is_empty()).then_some(ordinal as u32)
            })
            .collect::<Vec<_>>();

        unresolved.sort();
        unresolved.dedup();

        Self { paths, unresolved }
    }
}

/// One indexed module specifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SpecifierEntry {
    /// The source file.
    pub file: FileId,
    /// The source node that owns the specifier.
    pub source: GlobalNodeIdAny,
    /// The source range.
    pub span: Span,
    /// The module specifier kind.
    pub kind: SpecifierKind,
    /// The specifier text.
    pub text: String,
    /// The semantic target module when resolved.
    pub target_module: Option<ModuleId>,
    /// The semantic target path when known.
    pub target_path: Option<PathBuf>,
}

impl SpecifierEntry {
    /// Return the stable index order for this specifier.
    fn compare_by_source(&self, other: &Self) -> std::cmp::Ordering {
        let left = (
            self.source.module_id,
            self.file,
            self.span.start,
            self.span.end,
            self.kind,
            self.text.as_str(),
        );
        let right = (
            other.source.module_id,
            other.file,
            other.span.start,
            other.span.end,
            other.kind,
            other.text.as_str(),
        );

        left.cmp(&right)
    }
}

/// Kind of module specifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
pub enum SpecifierKind {
    /// Import specifier.
    Import,
    /// Export specifier.
    Export,
}
