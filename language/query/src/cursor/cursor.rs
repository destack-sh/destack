use std::cmp::Reverse;

use tspp_dir as dir;
use tspp_source::{EnclosingSpan, FileId, NodeSpanRegion, NodeSpanType};

use crate::{ModuleQueryContext, QueryResult};

/// One source insertion point and its enclosing spans.
pub(crate) struct Cursor<'module, 'repository> {
    /// The queried module.
    pub(crate) module: &'module ModuleQueryContext<'repository>,
    /// The queried file.
    pub(crate) file_id: FileId,
    /// The byte offset in the file.
    pub(crate) offset: u32,
    /// The enclosing spans ordered from smallest to largest.
    enclosing: Vec<EnclosingSpan>,
}

impl<'repository> ModuleQueryContext<'repository> {
    /// Select a source insertion point in this module.
    pub(crate) fn cursor(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Cursor<'_, 'repository>> {
        // collect source spans on both sides of the insertion point
        let index = self.source_index()?;
        let mut enclosing = index.get_enclosing_spans(file_id, offset, offset);
        if let Some(previous) = offset.checked_sub(1) {
            enclosing.extend(index.get_enclosing_spans(file_id, previous, previous));
        }

        // retain each authored owner once from innermost to outermost
        enclosing.sort_by_key(|span| (span.length, Reverse(span.source_id)));
        enclosing.dedup_by_key(|span| span.source_id);

        Ok(Cursor {
            module: self,
            file_id,
            offset,
            enclosing,
        })
    }
}

impl Cursor<'_, '_> {
    /// Return the enclosing spans from smallest to largest.
    pub(crate) fn enclosing(&self) -> &[EnclosingSpan] {
        &self.enclosing
    }

    /// Return whether one source region owns the cursor.
    pub(crate) fn is_in_region(&self, region: NodeSpanRegion) -> QueryResult<bool> {
        // inspect the requested region on each enclosing source node
        let index = self.module.source_index()?;
        for enclosing in self.enclosing() {
            let span = index.get_side(enclosing.source_id, NodeSpanType::Region(region));

            // accept the first matching region owner
            if span.is_some_and(|span| span.owns_cursor(self.offset)) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Resolve the innermost expression hole at the cursor.
    pub(crate) fn expression_hole(&self) -> QueryResult<Option<dir::LocalNodeId<dir::Expression>>> {
        let view = self.module.view()?;

        // walk inward to outward until one expression hole claims the cursor
        for enclosing in self.enclosing() {
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
