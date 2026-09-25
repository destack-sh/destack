use tspp_dir as dir;

use crate::cursor::Cursor;
use crate::{ModuleQueryContext, QueryResult};

impl Cursor<'_, '_> {
    /// Return the recorded visible scope at one authored position.
    pub(crate) fn scope(&self) -> QueryResult<Option<dir::LocalScope>> {
        // read the declarations and lexical scopes recorded for this module
        let enclosing = self.enclosing();
        let view = self.module.view()?;
        let bindings = self.module.bindings()?;

        // select the first recorded scope from innermost to outermost
        for enclosing in enclosing {
            let Some(node_id) = view.get_node_id_by_source_id(enclosing.source_id) else {
                continue;
            };
            let source = node_id.into_global(self.module.module_id());
            let Some(scope) = bindings.scope_for_node(source) else {
                continue;
            };

            return Ok(Some(scope));
        }

        Ok(None)
    }
}

impl ModuleQueryContext<'_> {
    /// Return the recorded module-root scope before any authored declaration.
    pub(crate) fn module_start_scope(&self) -> QueryResult<dir::LocalScope> {
        Ok(dir::LocalScope::new(
            self.namespace_scope()?,
            dir::LocalScopeMark(0),
        ))
    }

    /// Return the recorded scope attached to one expression.
    pub(crate) fn expression_scope(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> QueryResult<Option<dir::LocalScope>> {
        let source = expression_id.into_global_any(self.module_id());
        let Some(scope) = self.bindings()?.scope_for_node(source) else {
            return Ok(None);
        };

        Ok(Some(scope))
    }
}
