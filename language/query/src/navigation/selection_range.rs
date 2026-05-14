use destack_source::{FileId, Span, Uri};
use destack_workspace::{Repository, Revision};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::core::with_source_query_for_file;
use crate::source::span_contains_span;

/// A selection range with parent.
///
/// Represents a range that can be expanded to its parent syntactic element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// Request selection ranges for positions in a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionRangesRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offsets in the document.
    pub offsets: Vec<u32>,
}

/// Response payload for selection ranges queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionRangesResponse {
    /// Selection ranges.
    pub ranges: Vec<SelectionRange>,
}

/// Get selection ranges for positions in a file.
///
/// For each position, returns a nested SelectionRange from most specific
/// to least specific (innermost source node to outermost).
///
/// Used for "Expand Selection" / "Shrink Selection" editor commands.
pub fn selection_ranges(
    repository: &Repository,
    revision: Revision,
    file: FileId,
    positions: &[u32],
) -> Vec<SelectionRange> {
    with_source_query_for_file(repository, revision, file, |parsed| {
        // allocate the results vector
        let mut results = Vec::with_capacity(positions.len());

        for &offset in positions {
            // find all enclosing source nodes at this position
            let enclosing = parsed.source_map().get_enclosing_spans(offset, offset);

            // fall back to a minimal selection when no spans are available
            if enclosing.is_empty() {
                results.push(SelectionRange::leaf(Span::new(file, offset, offset)));
                continue;
            }

            // collect unique spans by range
            let mut seen = HashSet::new();
            let mut spans: Vec<Span> = enclosing
                .iter()
                .map(|enc| enc.span)
                .filter(|span| seen.insert((span.start, span.end)))
                .collect();

            // sort by size and position so parents come after children
            spans.sort_by_key(|span| {
                let len = span.end.saturating_sub(span.start);
                (len, span.start, span.end)
            });

            // find the most specific span at the cursor
            let leaf_span = spans
                .first()
                .copied()
                .unwrap_or(Span::new(file, offset, offset));

            // build a containment chain starting at the leaf span
            let mut chain = vec![leaf_span];
            for span in spans.into_iter().skip(1) {
                let last = *chain.last().unwrap_or(&leaf_span);
                let contains_leaf = span_contains_span(span, leaf_span);
                let contains_last = span_contains_span(span, last);
                if contains_leaf && contains_last && span != last {
                    chain.push(span);
                }
            }

            // build the nested selection range from outermost to innermost
            let mut selection = SelectionRange::leaf(*chain.last().unwrap_or(&leaf_span));
            for span in chain.iter().rev().skip(1) {
                selection = SelectionRange::with_parent(*span, selection);
            }

            // store the computed selection chain
            results.push(selection);
        }

        results
    })
    .unwrap_or_default()
}
