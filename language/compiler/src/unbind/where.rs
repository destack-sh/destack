use destack_ast::{self as ast};
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use super::UnbindContext;
use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Unbind a DIR where clause to an AST where clause.
    pub(super) fn unbind_where_clause(
        &self,
        module: &Module,
        clause_id: dir::LocalNodeId<dir::WhereClause>,
        tree: &dir::Tree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::Tree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::WhereClause> {
        let clause = tree.get(clause_id);
        let span = self.unbind_span(module, clause_id.into());
        let left = clause.left;
        let right = self.unbind_type_expression(
            module,
            clause.right,
            tree,
            symbols,
            types,
            ast_tree,
            ast_strings,
            context,
        );
        let ast_clause = ast::WhereClause { left, right };
        let ast_clause_id = ast_tree.insert(ast_clause, span);
        context.map(clause_id.into_any(), ast_clause_id.into_any());
        ast_clause_id
    }
}
