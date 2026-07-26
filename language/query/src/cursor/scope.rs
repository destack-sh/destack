use destack_dir as dir;
use destack_source::FileId;

use crate::{ModuleQueryContext, QueryResult};

/// One recorded lexical scope at a cursor position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScopeAtOffset {
    /// The scope id.
    pub scope_id: dir::LocalScopeId,
    /// The scope mark within the scope.
    pub scope_mark: dir::LocalScopeMark,
}

impl ScopeAtOffset {
    /// Create one scope cursor occurrence.
    pub(crate) fn new(scope_id: dir::LocalScopeId, scope_mark: dir::LocalScopeMark) -> Self {
        Self {
            scope_id,
            scope_mark,
        }
    }
}

impl ModuleQueryContext<'_> {
    /// Return the recorded visible scope at one authored position.
    pub(crate) fn scope_at_offset(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<ScopeAtOffset>> {
        let enclosing = self.enclosing_spans_at_cursor(file_id, offset)?;
        let view = self.view();

        // read the exact lexical cursor recorded by binding
        for enclosing in enclosing {
            let Some(node_id) = view.get_node_id_by_source_id(enclosing.source_id) else {
                continue;
            };
            let source = node_id.into_global(self.module_id());
            let Some(scope) = self.symbols().scope_for_node(source) else {
                continue;
            };

            return Ok(Some(ScopeAtOffset::new(scope.id, scope.mark)));
        }

        // FUGU #Incomplete: bind cursor marks for source gaps with no authored node
        Ok(None)
    }

    /// Return the recorded module-root scope before any authored declaration.
    pub(crate) fn module_start_scope(&self) -> ScopeAtOffset {
        ScopeAtOffset::new(self.namespace_scope(), dir::LocalScopeMark(0))
    }

    /// Return the recorded scope attached to one expression.
    pub(crate) fn expression_scope_at_offset(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<ScopeAtOffset> {
        let source = expression_id.into_global_any(self.module_id());
        let scope = self.symbols().scope_for_node(source)?;

        Some(ScopeAtOffset::new(scope.id, scope.mark))
    }
}
