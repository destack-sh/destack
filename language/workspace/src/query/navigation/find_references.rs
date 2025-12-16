use destack_source::{FileId, Span};

use crate::Session;

/// Result of a find references query.
#[derive(Debug, Clone, Default)]
pub struct ReferencesResult {
    /// All reference locations.
    pub references: Vec<Span>,
    /// Whether the definition is included in the results.
    pub include_declaration: bool,
}

impl ReferencesResult {
    /// Create an empty result.
    pub fn empty() -> Self {
        Self {
            references: Vec::new(),
            include_declaration: false,
        }
    }

    /// Whether any references were found.
    pub fn is_empty(&self) -> bool {
        self.references.is_empty()
    }

    /// Number of references found.
    pub fn len(&self) -> usize {
        self.references.len()
    }
}

/// Find all references to the symbol at the given position.
///
/// Optionally includes the declaration in the results.
pub fn find_references(
    _session: &Session,
    _file: FileId,
    _offset: u32,
    _include_declaration: bool,
) -> Option<ReferencesResult> {
    // 1. find the symbol at offset
    // 2. get canonical symbol (resolve imports)
    // 3. search all modules for references to that symbol
    // 4. optionally include the declaration itself
    todo!("#Incomplete: find_references")
}
