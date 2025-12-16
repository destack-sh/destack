use destack_source::{BatchEdit, FileId, Span};

use crate::Session;

/// Result of a prepare rename query.
#[derive(Debug, Clone)]
pub struct PrepareRenameResult {
    /// The range of the symbol to rename.
    pub range: Span,
    /// The current name (placeholder for rename dialog).
    pub placeholder: String,
}

/// Result of a rename query.
#[derive(Debug, Clone)]
pub struct RenameResult {
    /// All edits to apply.
    pub edits: BatchEdit,
}

impl RenameResult {
    /// Create an empty rename result.
    pub fn empty() -> Self {
        Self {
            edits: BatchEdit::new(),
        }
    }

    /// Create a rename result from a batch edit.
    pub fn from_edits(edits: BatchEdit) -> Self {
        Self { edits }
    }

    /// Whether there are any edits.
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// Total number of edits.
    pub fn edit_count(&self) -> usize {
        self.edits.total_edits()
    }

    /// Number of files affected.
    pub fn file_count(&self) -> usize {
        self.edits.file_count()
    }
}

/// Check if the symbol at the given position can be renamed.
///
/// Returns the range and current name if renameable.
pub fn prepare_rename(
    _session: &Session,
    _file: FileId,
    _offset: u32,
) -> Option<PrepareRenameResult> {
    // 1. find the symbol at offset
    // 2. check if it's renameable (not a keyword, not from external module)
    // 3. return the range and current name
    todo!("#Incomplete: prepare_rename")
}

/// Rename the symbol at the given position.
///
/// Returns edits for all files that need to be modified.
pub fn rename(
    _session: &Session,
    _file: FileId,
    _offset: u32,
    _new_name: &str,
) -> Option<RenameResult> {
    // 1. find the symbol at offset
    // 2. validate new_name is a valid identifier
    // 3. find all references (including declaration)
    // 4. create edits for each reference
    // 5. group by file
    todo!("#Incomplete: rename")
}
