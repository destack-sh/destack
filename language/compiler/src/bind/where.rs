use destack_ast as ast;
use destack_dir::{
    LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark, NodeTree, NodeType, SymbolTable,
    TypeTable, WhereClause,
};

use crate::Compiler;

use destack_workspace::{Module, ModuleAst};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a where clause to a DIR where clause.
    pub(super) fn bind_where_clause(
        &self,
        module: &Module,
        ast: &ModuleAst,
        scope: (LocalScopeId, LocalScopeMark),
        ast_where_clause_id: ast::LocalNodeId<ast::WhereClause>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<WhereClause> {
        let ast_where_clause = ast.tree.get(ast_where_clause_id);
        let where_clause_id = tree.reserve_from_source(
            NodeType::WhereClause,
            ast_where_clause_id.id,
            scope,
            parent_id,
        );
        match ast_where_clause {
            ast::WhereClause::Assertion { left, right } => {
                let left = self.program.strings.intern_from(&ast.strings, *left);
                let right = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *right,
                    Some(where_clause_id),
                    tree,
                    symbols,
                    types,
                );
                tree.insert(where_clause_id, WhereClause::Assertion { left, right })
            }
            ast::WhereClause::Guard { guard } => {
                let guard = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *guard,
                    Some(where_clause_id),
                    tree,
                    symbols,
                    types,
                );
                tree.insert(where_clause_id, WhereClause::Guard { guard })
            }
        }
    }
}
