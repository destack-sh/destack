use std::collections::HashSet;
use std::path::{Path, PathBuf};

use destack_source::{FileId, ModuleId};
use serde::{Deserialize, Serialize};

/// Import specifier rewrite index.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecifierIndex {
    /// The resolved specifier entries ordered by target path.
    by_target_path: Vec<(PathBuf, SpecifierEntry)>,
    /// The specifier entries without a resolved target path.
    unresolved: Vec<SpecifierEntry>,
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
            resolved_specifier_key(left).cmp(&resolved_specifier_key(right))
        });
        self.by_target_path.dedup();

        self.unresolved
            .sort_by(|left, right| specifier_entry_key(left).cmp(&specifier_entry_key(right)));
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
        entries.sort_by(|left, right| specifier_entry_key(left).cmp(&specifier_entry_key(right)));
        entries.dedup();

        entries.into_iter()
    }

    /// Return all indexed resolved specifier entries.
    pub fn resolved(&self) -> impl Iterator<Item = &SpecifierEntry> {
        self.by_target_path.iter().map(|(_, entry)| entry)
    }

    /// Return all indexed unresolved specifier entries.
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

/// Import specifier rewrite entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecifierEntry {
    /// The module containing the specifier.
    pub module_id: ModuleId,
    /// The source file.
    pub file_id: FileId,
    /// The AST node that owns the specifier.
    pub ast_node_id: u32,
    /// The specifier text.
    pub specifier: String,
    /// The semantic target module when resolved.
    pub target_module_id: Option<ModuleId>,
    /// The semantic target path when known.
    pub target_path: Option<PathBuf>,
}

/// Return the stable ordering key for one specifier entry.
fn specifier_entry_key(entry: &SpecifierEntry) -> (ModuleId, FileId, u32, &str) {
    (
        entry.module_id,
        entry.file_id,
        entry.ast_node_id,
        entry.specifier.as_str(),
    )
}

/// Return the stable ordering key for one resolved specifier entry.
fn resolved_specifier_key(
    entry: &(PathBuf, SpecifierEntry),
) -> (&PathBuf, ModuleId, FileId, u32, &str) {
    (
        &entry.0,
        entry.1.module_id,
        entry.1.file_id,
        entry.1.ast_node_id,
        entry.1.specifier.as_str(),
    )
}
