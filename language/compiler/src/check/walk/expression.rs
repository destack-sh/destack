use destack_dir as dir;

use crate::check::CheckModuleState;

impl CheckModuleState {
    /// Walk one catch clause and collect check work.
    pub(in crate::check) fn walk_catch(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Catch>,
        catch: &dir::Catch,
    ) {
        dir::walk_catch(self, tree, id, catch);
    }

    /// Walk one expression and collect check work.
    pub(in crate::check) fn walk_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        dir::walk_expression(self, tree, id, expression);
    }

    /// Walk one where clause and collect check work.
    pub(in crate::check) fn walk_where_clause(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::WhereClause>,
        where_clause: &dir::WhereClause,
    ) {
        dir::walk_where_clause(self, tree, id, where_clause);
    }
}
