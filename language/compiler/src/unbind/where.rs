use destack_ast::{self as ast};
use destack_base::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use crate::Compiler;

impl Compiler {
    /// Unbind a DIR where clause to an AST where clause.
    pub(super) fn unbind_where_clause(
        &self,
        module: &Module,
        clause_id: dir::LocalNodeId<dir::WhereClause>,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
    ) -> ast::LocalNodeId<ast::WhereClause> {
        let clause = tree.get(clause_id);
        let span = self.unbind_span(module, clause_id.into());
        let ast_clause = match clause {
            dir::WhereClause::Assertion { left, right } => {
                let left = ast_strings.intern_from(&self.program.strings, *left);
                let right =
                    self.unbind_expression(module, *right, tree, symbols, ast_tree, ast_strings);
                ast::WhereClause::Assertion { left, right }
            }
            dir::WhereClause::Guard { guard } => {
                let guard =
                    self.unbind_expression(module, *guard, tree, symbols, ast_tree, ast_strings);
                ast::WhereClause::Guard { guard }
            }
        };
        ast_tree.insert(ast_clause, span)
    }
}
