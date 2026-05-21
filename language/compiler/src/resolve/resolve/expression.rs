use destack_dir as dir;

use crate::resolve::state::{DependencyClause, ResolveState};

impl ResolveState<'_> {
    /// Walk one expression and record dependency clauses.
    pub(in crate::resolve) fn walk_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        match expression {
            dir::Expression::Import { items, .. } => {
                let clause = DependencyClause::Import {
                    expression_id: id,
                    items: items.clone(),
                };
                self.record_dependency_clause(clause);
            }
            dir::Expression::Export {
                target: Some(_),
                items,
                ..
            } => {
                let clause = DependencyClause::ReExport {
                    expression_id: id,
                    items: items.clone(),
                };
                self.record_dependency_clause(clause);
            }
            _ => dir::walk_expression(self, tree, id, expression),
        }
    }
}
