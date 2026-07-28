use std::cmp::Reverse;

use destack_dir as dir;
use destack_source::{EnclosingSpan, FileId, NodeSpanRegion, NodeSpanType};

use crate::{ModuleQueryContext, QueryResult};

impl ModuleQueryContext<'_> {
    /// Collect enclosing spans at an exact cursor offset.
    pub(crate) fn enclosing_spans_at_cursor(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> Vec<EnclosingSpan> {
        let mut enclosing = self.sorted_enclosing_spans(file_id, offset, offset);

        // source spans own their trailing cursor boundary
        if let Some(previous) = offset.checked_sub(1) {
            enclosing.extend(self.sorted_enclosing_spans(file_id, previous, previous));
        }

        // retain each authored owner once from innermost to outermost
        enclosing.sort_by_key(|span| (span.length, Reverse(span.source_id)));
        enclosing.dedup_by_key(|span| span.source_id);

        enclosing
    }

    /// Return whether one source region owns the cursor.
    pub(crate) fn region_owns_cursor(
        &self,
        file_id: FileId,
        offset: u32,
        region: NodeSpanRegion,
    ) -> bool {
        for enclosing in self.enclosing_spans_at_cursor(file_id, offset) {
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
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<dir::LocalNodeId<dir::Expression>>> {
        let view = self.view();

        // walk inward to outward until one expression hole claims the cursor
        for enclosing in self.enclosing_spans_at_cursor(file_id, offset) {
            let Some(node_id) = view.get_node_id_by_source_id(enclosing.source_id) else {
                continue;
            };
            if node_id.ty != dir::NodeType::Expression {
                continue;
            }
            let expression_id = dir::LocalNodeId::<dir::Expression>::new(node_id.id);

            // only explicit parser holes classify expression slots
            if matches!(view.get(expression_id), dir::Expression::Missing) {
                return Ok(Some(expression_id));
            }
        }

        Ok(None)
    }
}
