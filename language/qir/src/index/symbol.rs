use destack_source::{FileId, ModuleId, Span};
use serde::{Deserialize, Serialize};

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
        self.entries
            .sort_by(|left, right| symbol_entry_key(left).cmp(&symbol_entry_key(right)));
        self.entries.dedup();
    }

    /// Return entries whose name contains the case-insensitive query text.
    pub fn search(&self, query: &str) -> Vec<SymbolEntry> {
        let query = query.to_lowercase();

        self.entries
            .iter()
            .filter(|entry| query.is_empty() || entry.name.to_lowercase().contains(&query))
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
    pub name: String,
    /// The symbol kind.
    pub kind: SymbolKind,
    /// The owning module.
    pub module_id: ModuleId,
    /// The source file.
    pub file_id: FileId,
    /// The source range.
    pub range: Span,
    /// The containing symbol display name.
    pub container_name: Option<String>,
}

/// Searchable workspace symbol kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SymbolKind {
    /// Namespace symbol.
    Namespace,
    /// Class symbol.
    Class,
    /// Method symbol.
    Method,
    /// Field symbol.
    Field,
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
    /// Enum member symbol.
    EnumMember,
    /// Struct symbol.
    Struct,
    /// Type parameter symbol.
    TypeParameter,
}

/// Return the stable ordering key for one symbol entry.
fn symbol_entry_key(entry: &SymbolEntry) -> (&str, ModuleId, FileId, u32, u32) {
    (
        entry.name.as_str(),
        entry.module_id,
        entry.file_id,
        entry.range.start,
        entry.range.end,
    )
}
