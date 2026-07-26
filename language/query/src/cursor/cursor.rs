use destack_dir as dir;
use destack_source::{EnclosingSpan, FileId, NodeSpanRegion, NodeSpanType};

use crate::{ModuleQueryContext, QueryResult};

impl ModuleQueryContext<'_> {
    /// Collect enclosing spans around a cursor.
    ///
    /// This supplements the direct cursor probe with adjacent significant token probes.
    pub(crate) fn enclosing_spans_at_cursor(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Vec<EnclosingSpan>> {
        let offsets = self.cursor_probe_offsets(file_id, offset)?;

        Ok(self.enclosing_spans_at_offsets(file_id, offsets))
    }

    /// Return whether one source region owns the cursor.
    pub(crate) fn region_owns_cursor(
        &self,
        file_id: FileId,
        offset: u32,
        region: NodeSpanRegion,
    ) -> bool {
        for enclosing in self.enclosing_spans_with_previous(file_id, offset) {
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
        for enclosing in self.enclosing_spans_at_cursor(file_id, offset)? {
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

    /// Collect probe offsets that should share one cursor owner.
    fn cursor_probe_offsets(&self, file_id: FileId, offset: u32) -> QueryResult<Vec<u32>> {
        let mut offsets = vec![offset];

        // include the previous byte for ordinary boundary cases
        if offset > 0 {
            offsets.push(offset - 1);
        }

        // include the previous significant token interior when the cursor trails it
        if let Some(token) = self.previous_significant_token(file_id, offset)?
            && token.span.end <= offset
            && token.span.start < token.span.end
        {
            offsets.push(token.span.end - 1);
        }

        // include the next significant token start when the cursor precedes it
        if let Some(token) = self.next_significant_token(file_id, offset)?
            && token.span.start >= offset
        {
            offsets.push(token.span.start);
        }

        Ok(offsets)
    }
}
