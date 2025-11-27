use dyst_ast as ast;
use dyst_dir::{
    LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark, Module, NodeTree, NodeType,
    SymbolTable, TypeTable, WithClause,
};

use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a with clause to a DIR with clause.
    pub(super) fn bind_with_clause(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        ast_with_clause_id: ast::LocalNodeId<ast::WithClause>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<WithClause> {
        let ast_with_clause = module.ast.get(ast_with_clause_id);
        let with_clause_id =
            tree.reserve_from_source(NodeType::WithClause, ast_with_clause_id, scope, parent_id);
        let alias = ast_with_clause
            .alias
            .map(|alias| self.program.strings.intern_from(&module.ast_strings, alias));
        let right = self.bind_expression(
            module,
            scope,
            ast_with_clause.right,
            Some(with_clause_id),
            tree,
            symbols,
            types,
        );
        tree.insert(with_clause_id, WithClause { alias, right })
    }
}
