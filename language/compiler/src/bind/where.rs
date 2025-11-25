use dyst_ast as ast;
use dyst_dir::{LocalNodeId, LocalScopeId, Module, NodeTree, SymbolTable, TypeTable, WhereClause};

use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a where clause to a DIR where clause.
    pub(super) fn bind_where_clause(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        where_clause_id: ast::LocalNodeId<ast::WhereClause>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<WhereClause> {
        let where_clause = module.get(where_clause_id);
        match where_clause {
            ast::WhereClause::Assertion { left, right } => {
                let left = self.program.strings.intern_from(&module.ast_strings, *left);
                let right = self.bind_expression(module, scope_id, *right, tree, symbols, types);
                tree.insert_from_source(
                    WhereClause::Assertion { left, right },
                    where_clause_id,
                    scope_id,
                )
            }
            ast::WhereClause::Guard { guard } => {
                let guard = self.bind_expression(module, scope_id, *guard, tree, symbols, types);
                tree.insert_from_source(WhereClause::Guard { guard }, where_clause_id, scope_id)
            }
        }
    }
}
