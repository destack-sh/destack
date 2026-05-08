use destack_ast as ast;
use destack_dir::{
    DeclaredModule, LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark, NodeType,
    SymbolSpace, SymbolTable, Tree, TypeExpression, TypeTable, WhereClause,
};

use crate::Compiler;

use destack_artifact::Ast;
use destack_workspace::Module;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a where clause to a DIR where clause.
    pub(super) fn bind_where_clause(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_scope: LocalScopeId,
        declared_modules: &mut Vec<DeclaredModule>,
        scope: (LocalScopeId, LocalScopeMark),
        ast_where_clause_id: ast::LocalNodeId<ast::WhereClause>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut Tree,
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
        let left = ast_where_clause.left;
        let right: LocalNodeId<TypeExpression> = self.bind_type_expression(
            module,
            ast,
            namespace_scope,
            global_scope,
            declared_modules,
            scope,
            ast_where_clause.right,
            Some(where_clause_id),
            tree,
            symbols,
            types,
            SymbolSpace::Type,
        );
        tree.insert(where_clause_id, WhereClause { left, right })
    }
}
