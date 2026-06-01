use destack_dir as dir;
use destack_source::{EnclosingSpan, NodeSpanRegion, NodeSpanType, Span};

use super::DirQueryContext;

/// Check whether one span owns the cursor.
pub(crate) fn span_owns_cursor(span: Span, offset: u32) -> bool {
    // the span directly owns the cursor
    if span.contains(offset) {
        return true;
    }

    // boundary ownership also includes the previous byte
    let previous_offset = offset.saturating_sub(1);
    if span.contains(previous_offset) {
        return true;
    }

    span.start == offset && span.end == offset
}

impl DirQueryContext<'_> {
    /// Collect enclosing spans around a cursor.
    ///
    /// This supplements the direct cursor probes with adjacent significant token probes
    /// so recovered source ownership survives when the cursor sits in whitespace gaps.
    pub(crate) fn enclosing_spans_at_cursor(self, offset: u32) -> Vec<EnclosingSpan> {
        let offsets = self.cursor_probe_offsets(offset);

        self.enclosing_spans_at_offsets(offsets)
    }

    /// Check whether an offset falls inside one source type side span.
    pub(crate) fn offset_is_in_source_type_side_span(self, offset: u32) -> bool {
        let previous_offset = offset.saturating_sub(1);

        // scan enclosing source spans
        for enclosing in self.enclosing_spans_with_previous(offset) {
            let type_span = self.source_index().get_side(
                enclosing.source_id,
                NodeSpanType::Region(NodeSpanRegion::Type),
            );

            // type side spans own both the cursor and the previous byte boundary
            if let Some(span) = type_span
                && (span.contains(offset) || span.contains(previous_offset))
            {
                return true;
            }
        }

        false
    }

    /// Resolve the innermost missing expression at the cursor.
    pub(crate) fn enclosing_missing_expression(
        self,
        offset: u32,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        // walk inward to outward until one missing expression claims the cursor
        for enclosing in self.enclosing_spans_at_cursor(offset) {
            if self.tree().get_node_type(enclosing.source_id) != dir::NodeType::Expression {
                continue;
            }

            let expr_id = dir::LocalNodeId::<dir::Expression>::new(enclosing.source_id);

            // only recovered missing expressions classify expression slots
            if matches!(self.tree().get(expr_id), dir::Expression::Missing) {
                return Some(expr_id);
            }
        }

        None
    }

    /// Collect probe offsets that should share one cursor owner.
    fn cursor_probe_offsets(self, offset: u32) -> Vec<u32> {
        let mut offsets = vec![offset];

        // include the previous byte for ordinary boundary cases
        if offset > 0 {
            offsets.push(offset - 1);
        }

        // include the previous significant token interior when the cursor trails it
        if let Some(token) = self.previous_significant_token(offset)
            && token.span.end <= offset
            && token.span.start < token.span.end
        {
            offsets.push(token.span.end - 1);
        }

        // include the next significant token start when the cursor precedes it
        if let Some(token) = self.next_significant_token(offset)
            && token.span.start >= offset
        {
            offsets.push(token.span.start);
        }

        offsets
    }
}
