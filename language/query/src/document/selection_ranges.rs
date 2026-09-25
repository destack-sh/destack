use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;
use tspp_source::{FileId, Span};

use crate::{Module, ModuleQueryContext, QueryError, QueryResult};

/// One source range that can be expanded to its parent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SelectionRange {
    /// The range of this selection.
    pub range: Span,
    /// The parent selection range (for expand selection).
    pub parent: Option<Box<SelectionRange>>,
}

impl SelectionRange {
    /// Create a selection range with a parent.
    fn with_parent(range: Span, parent: SelectionRange) -> Self {
        Self {
            range,
            parent: Some(Box::new(parent)),
        }
    }
}

impl From<Span> for SelectionRange {
    /// Convert one source span into a leaf selection range.
    fn from(range: Span) -> Self {
        Self {
            range,
            parent: None,
        }
    }
}

/// A selection ranges request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SelectionRangesRequest {
    /// The queried module profile.
    pub module: Module,
    /// The queried source file.
    pub file_id: FileId,
    /// The byte offsets in the document.
    pub offsets: Vec<u32>,
}

/// A selection ranges response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SelectionRangesResponse {
    /// Selection ranges.
    pub ranges: Vec<SelectionRange>,
}

impl ModuleQueryContext<'_> {
    /// Return nested selection ranges from the narrowest source owner outward.
    pub fn selection_ranges(
        &self,
        request: SelectionRangesRequest,
    ) -> QueryResult<SelectionRangesResponse> {
        let mut ranges = Vec::with_capacity(request.offsets.len());

        // build one selection chain per requested position
        for offset in request.offsets {
            let range = self.selection_range_at_offset(request.file_id, offset)?;
            ranges.push(range);
        }

        Ok(SelectionRangesResponse { ranges })
    }

    /// Return one selection range at an offset.
    fn selection_range_at_offset(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<SelectionRange> {
        // select a retained comment as one authored lexical unit
        if let Some(comment) = self
            .comments(file_id)?
            .iter()
            .find(|comment| comment.span.contains(offset))
        {
            return Ok(comment.span.into());
        }

        // collect exact authored ranges containing the cursor
        let mut spans = self.source_index()?.spans_at(file_id, offset);
        if spans.is_empty() {
            let range = Span::new(file_id, offset, offset).into();

            return Ok(range);
        }

        // reject invalid source index state before ordering the ranges
        for span in &spans {
            if span.end < span.start {
                return Err(QueryError::invalid(format!("selection range: {:?}", *span)));
            }
        }
        spans.sort_by_key(|span| (span.end - span.start, span.start, span.end));

        // build the strict containment chain from the narrowest range
        let leaf_span = spans[0];
        let mut chain = vec![leaf_span];
        let mut last = leaf_span;

        for span in spans.into_iter().skip(1) {
            let contains_leaf = span.contains_span(leaf_span);
            let contains_last = span.contains_span(last);
            if contains_leaf && contains_last && span != last {
                chain.push(span);
                last = span;
            }
        }

        // link each narrower range to its immediate parent
        let mut selection = last.into();
        for span in chain.iter().rev().skip(1) {
            selection = SelectionRange::with_parent(*span, selection);
        }

        Ok(selection)
    }
}
