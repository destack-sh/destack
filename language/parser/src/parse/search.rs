use crate::Parser;
use tspp_source::{EnclosingSpan, NodeSearchMode, Span};

impl Parser {
    /// Return the selected node starting at one source span.
    pub fn find_node_starting_at(
        &self,
        span: &Span,
        search: NodeSearchMode,
    ) -> Option<EnclosingSpan> {
        self.select_enclosing_span(span, search, |candidate| candidate.span.start == span.start)
    }

    /// Return the selected node ending at one source span.
    pub fn find_node_ending_at(
        &self,
        span: &Span,
        search: NodeSearchMode,
    ) -> Option<EnclosingSpan> {
        self.select_enclosing_span(span, search, |candidate| candidate.span.end == span.end)
    }

    /// Return the selected node enclosing one source span.
    pub fn find_node_enclosing_at(
        &self,
        span: &Span,
        search: NodeSearchMode,
        filter: impl Fn(&EnclosingSpan) -> bool,
    ) -> Option<EnclosingSpan> {
        self.select_enclosing_span(span, search, filter)
    }

    /// Select one enclosing node matching a source predicate.
    fn select_enclosing_span(
        &self,
        span: &Span,
        search: NodeSearchMode,
        filter: impl Fn(&EnclosingSpan) -> bool,
    ) -> Option<EnclosingSpan> {
        let mut selected = None;
        self.tree.source_index.visit_enclosing_spans(
            self.file_id,
            span.start,
            span.end.saturating_sub(1),
            |candidate| {
                // retain the highest ranked matching node
                if filter(&candidate)
                    && selected.as_ref().is_none_or(|current| {
                        Self::is_preferred_enclosing_span(search, &candidate, current)
                    })
                {
                    selected = Some(candidate);
                }
            },
        );

        selected
    }

    /// Return whether one candidate outranks the selected node for a search mode.
    fn is_preferred_enclosing_span(
        search: NodeSearchMode,
        candidate: &EnclosingSpan,
        current: &EnclosingSpan,
    ) -> bool {
        match search {
            NodeSearchMode::BiggestOutermost => {
                candidate.length > current.length
                    || candidate.length == current.length && candidate.source_id > current.source_id
            }
            NodeSearchMode::SmallestOutermost => {
                candidate.length < current.length
                    || candidate.length == current.length && candidate.source_id > current.source_id
            }
            NodeSearchMode::SmallestInnermost => {
                candidate.length < current.length
                    || candidate.length == current.length && candidate.source_id < current.source_id
            }
        }
    }

    /// Return whether two source spans occupy the same line.
    #[inline]
    pub fn is_same_line(&self, left: Span, right: Span) -> bool {
        self.file.is_same_line(left.start, right.end)
    }
}
