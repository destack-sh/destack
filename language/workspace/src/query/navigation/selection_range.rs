use destack_source::{FileId, Span};

use crate::Session;
use crate::query::common::get_module_by_file_id;

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

    /// Get the depth of this selection range.
    pub fn depth(&self) -> usize {
        match &self.parent {
            None => 1,
            Some(parent) => 1 + parent.depth(),
        }
    }
}

/// Get selection ranges for positions in a file.
///
/// For each position, returns a nested SelectionRange from most specific
/// to least specific (innermost syntax node to outermost).
///
/// Used for "Expand Selection" / "Shrink Selection" editor commands.
pub fn selection_ranges(session: &Session, file: FileId, positions: &[u32]) -> Vec<SelectionRange> {
    // get the module for this file
    let Some(module) = get_module_by_file_id(session, file) else {
        return Vec::new();
    };

    let module = module.read();
    let Some(ctx) = session.query_context(&module) else {
        return Vec::new();
    };
    let mut results = Vec::with_capacity(positions.len());

    for &offset in positions {
        // find all enclosing AST nodes at this position
        let enclosing = ctx.ast.tree.source_map.get_enclosing_spans(offset, offset);

        if enclosing.is_empty() {
            // no enclosing spans, return a minimal selection at the position
            results.push(SelectionRange::leaf(Span::new(file, offset, offset)));
            continue;
        }

        // enclosing spans are sorted by distance (innermost first)
        // we need to deduplicate spans with the same range
        let mut unique_spans: Vec<Span> = Vec::new();
        for enc in &enclosing {
            if unique_spans.last() != Some(&enc.span) {
                unique_spans.push(enc.span);
            }
        }

        // build the nested SelectionRange from outermost to innermost
        // start with the outermost as the root (no parent)
        let mut selection = SelectionRange::leaf(
            unique_spans
                .pop()
                .unwrap_or(Span::new(file, offset, offset)),
        );

        // add each subsequent span as a child (with the previous as parent)
        while let Some(span) = unique_spans.pop() {
            selection = SelectionRange::with_parent(span, selection);
        }

        results.push(selection);
    }

    results
}
