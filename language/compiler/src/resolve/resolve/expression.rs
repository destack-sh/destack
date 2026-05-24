use destack_dir as dir;

use crate::resolve::state::{ModuleClause, ResolveState};

impl ResolveState<'_> {
    /// Walk one expression and record module clauses.
    pub(in crate::resolve) fn walk_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        match expression {
            dir::Expression::Import { items, .. } => {
                let clause = ModuleClause::Import {
                    expression_id: id,
                    items: items.clone(),
                };
                self.record_module_clause(clause);
            }
            dir::Expression::Export {
                target: Some(_),
                items,
                ..
            } => {
                let clause = ModuleClause::ReExport {
                    expression_id: id,
                    items: items.clone(),
                };
                self.record_module_clause(clause);
            }
            _ => dir::walk_expression(self, tree, id, expression),
        }
    }
}
