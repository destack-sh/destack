use dyst_ast as ast;
use dyst_dir::{NodeId, UseItem};
use dyst_source::SourceId;

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Lower a use clause to a DIR use items.
    /// Use clauses with multiple items are flattened into multiple use items.
    pub fn lower_use_clause(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        use_clause_id: ast::NodeId<ast::UseClause>,
    ) -> Vec<NodeId<UseItem>> {
        let use_clause = ast.get(use_clause_id);
        let path = self.lower_path(source_id, ast, &use_clause.target);

        // flatten items
        if let Some(items) = &use_clause.items {
            items
                .iter()
                .map(|item| {
                    let use_item = ast.get(*item);
                    let alias = use_item
                        .alias
                        .map(|alias| self.intern_string(source_id, alias));
                    let use_item = UseItem {
                        source: path.clone(),
                        alias,
                    };
                    self.tree.allocate(use_item, source_id, use_clause_id)
                })
                .collect()
        }
        // single item
        else {
            let alias = use_clause
                .alias
                .map(|alias| self.intern_string(source_id, alias));
            let use_item = UseItem {
                source: path,
                alias,
            };
            vec![self.tree.allocate(use_item, source_id, use_clause_id)]
        }
    }
}
