use destack_dir as dir;
use destack_source::{EnclosingSpan, NodeSpanRegion, NodeSpanType};

use crate::ModuleQueryContext;

impl ModuleQueryContext<'_> {
    /// Collect enclosing spans around a cursor.
    ///
    /// This supplements the direct cursor probe with adjacent significant token probes.
    pub(crate) fn enclosing_spans_at_cursor(&self, offset: u32) -> Vec<EnclosingSpan> {
        let offsets = self.cursor_probe_offsets(offset);

        self.enclosing_spans_at_offsets(offsets)
    }

    /// Return whether one source region owns the cursor.
    pub(crate) fn region_owns_cursor(&self, offset: u32, region: NodeSpanRegion) -> bool {
        for enclosing in self.enclosing_spans_with_previous(offset) {
            let span = self
                .source_index()
                .get_side(enclosing.source_id, NodeSpanType::Region(region));

            // accept the first matching region owner
            if span.is_some_and(|span| span.owns_cursor(offset)) {
                return true;
            }
        }

        false
    }

    /// Resolve the innermost expression hole at the cursor.
    pub(crate) fn expression_hole_at_offset(
        &self,
        offset: u32,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        // walk inward to outward until one expression hole claims the cursor
        for enclosing in self.enclosing_spans_at_cursor(offset) {
            if self.tree().get_node_type(enclosing.source_id) != dir::NodeType::Expression {
                continue;
            }

            let expr_id = dir::LocalNodeId::<dir::Expression>::new(enclosing.source_id);

            // only explicit parser holes classify expression slots
            if matches!(self.tree().get(expr_id), dir::Expression::Missing) {
                return Some(expr_id);
            }
        }

        None
    }

    /// Collect probe offsets that should share one cursor owner.
    fn cursor_probe_offsets(&self, offset: u32) -> Vec<u32> {
        let mut offsets = vec![offset];

        // include the previous byte for ordinary boundary cases
        if offset > 0 {
            offsets.push(offset - 1);
        }

        // include the previous significant token interior when the cursor trails it
        if let Some(token) = self.previous_significant_token(offset) {
            if token.span.end <= offset && token.span.start < token.span.end {
                offsets.push(token.span.end - 1);
            }
        }

        // include the next significant token start when the cursor precedes it
        if let Some(token) = self.next_significant_token(offset) {
            if token.span.start >= offset {
                offsets.push(token.span.start);
            }
        }

        offsets
    }
}
