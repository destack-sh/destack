use destack_dir as dir;
use destack_source::FileId;

use crate::{ModuleQueryContext, QueryResult};

impl ModuleQueryContext<'_> {
    /// Return the recorded visible scope at one authored position.
    pub(crate) fn scope_at_offset(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<dir::LocalScope>> {
        let enclosing = self.enclosing_spans_at_cursor(file_id, offset);
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

            return Ok(Some(scope));
        }

        Ok(None)
    }

    /// Return the recorded module-root scope before any authored declaration.
    pub(crate) fn module_start_scope(&self) -> dir::LocalScope {
        dir::LocalScope::new(self.namespace_scope(), dir::LocalScopeMark(0))
    }

    /// Return the recorded scope attached to one expression.
    pub(crate) fn expression_scope_at_offset(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalScope> {
        let source = expression_id.into_global_any(self.module_id());
        let scope = self.symbols().scope_for_node(source)?;

        Some(scope)
    }
}
