use destack_dir as dir;
use destack_source::{FileId, ModuleId, Span};
use serde::{Deserialize, Serialize};

use super::Name;

/// Searchable symbol declaration index.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolIndex {
    /// The symbol entries in stable display order.
    entries: Vec<SymbolEntry>,
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
        self.entries.sort_by_key(|entry| entry.order());
        self.entries.dedup();
    }

    /// Return entries whose name contains the case-insensitive query text.
    pub fn search(&self, query: &str) -> Vec<SymbolEntry> {
        let query = query.to_lowercase();

        self.entries
            .iter()
            .filter(|entry| entry.name.matches_lowercase_query(&query))
            .cloned()
            .collect()
    }

    /// Return all indexed symbol entries.
    pub fn entries(&self) -> &[SymbolEntry] {
        &self.entries
    }
}

/// Searchable workspace symbol entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolEntry {
    /// The display name.
    pub name: Name,
    /// The symbol kind.
    pub kind: SymbolKind,
    /// The owning module.
    pub module_id: ModuleId,
    /// The source file.
    pub file_id: FileId,
    /// The source range.
    pub range: Span,
    /// The indexed symbol when known.
    pub symbol_id: Option<dir::GlobalSymbolId>,
    /// The containing symbol display name.
    pub container_name: Option<String>,
}

impl SymbolEntry {
    /// Return the stable index order for this symbol.
    fn order(&self) -> (Name, ModuleId, FileId, u32, u32) {
        (
            self.name.clone(),
            self.module_id,
            self.file_id,
            self.range.start,
            self.range.end,
        )
    }
}

/// Searchable workspace symbol kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SymbolKind {
    /// Namespace symbol.
    Namespace,
    /// Class symbol.
    Class,
    /// Enum symbol.
    Enum,
    /// Interface symbol.
    Interface,
    /// Function symbol.
    Function,
    /// Variable symbol.
    Variable,
    /// Constant symbol.
    Constant,
    /// Struct symbol.
    Struct,
    /// Type parameter symbol.
    TypeParameter,
}
