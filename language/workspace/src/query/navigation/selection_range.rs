use destack_source::{FileId, Span};

use crate::Session;

/// A selection range with parent.
///
/// Represents a range that can be expanded to its parent syntactic element.
#[derive(Debug, Clone)]
pub struct SelectionRange {
    /// The range of this selection.
    pub range: Span,
    /// The parent selection range (for expand selection).
    pub parent: Option<Box<SelectionRange>>,
}

impl SelectionRange {
    /// Create a leaf selection range (no parent).
    pub fn leaf(range: Span) -> Self {
        Self {
            range,
            parent: None,
        }
    }

    /// Create a selection range with a parent.
    pub fn with_parent(range: Span, parent: SelectionRange) -> Self {
        Self {
            range,
            parent: Some(Box::new(parent)),
        }
    }
}

/// Get selection ranges for positions in a file.
///
/// For each position, returns a nested SelectionRange from most specific
/// to least specific (innermost syntax node to outermost).
///
/// Used for "Expand Selection" / "Shrink Selection" editor commands.
pub fn selection_ranges(
    _session: &Session,
    _file: FileId,
    _positions: &[u32],
) -> Vec<SelectionRange> {
    // 1. parse the file to get AST
    // 2. for each position, find the syntax node at that position
    // 3. walk up the tree, collecting ranges:
    //    - identifier -> expression -> statement -> block -> function -> module
    // 4. build nested SelectionRange from innermost to outermost
    todo!("#Incomplete: selection_ranges")
}
