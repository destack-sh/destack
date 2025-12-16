use destack_source::{FileId, Span};

use crate::Session;

/// Kind of document highlight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HighlightKind {
    /// A textual occurrence.
    #[default]
    Text,
    /// Read access to a symbol.
    Read,
    /// Write access to a symbol.
    Write,
}

/// A highlighted range in a document.
#[derive(Debug, Clone)]
pub struct DocumentHighlight {
    /// The highlighted range.
    pub range: Span,
    /// The kind of highlight.
    pub kind: HighlightKind,
}

impl DocumentHighlight {
    /// Create a text highlight.
    pub fn text(range: Span) -> Self {
        Self {
            range,
            kind: HighlightKind::Text,
        }
    }

    /// Create a read highlight.
    pub fn read(range: Span) -> Self {
        Self {
            range,
            kind: HighlightKind::Read,
        }
    }

    /// Create a write highlight.
    pub fn write(range: Span) -> Self {
        Self {
            range,
            kind: HighlightKind::Write,
        }
    }
}

/// Highlight all occurrences of the symbol at the given position in the document.
///
/// Only highlights within the same file (for cross-file, use find_references).
pub fn document_highlight(
    _session: &Session,
    _file: FileId,
    _offset: u32,
) -> Vec<DocumentHighlight> {
    // 1. find the symbol at offset
    // 2. find all references to that symbol in the same file
    // 3. classify as read/write based on context
    todo!("#Incomplete: document_highlight")
}
