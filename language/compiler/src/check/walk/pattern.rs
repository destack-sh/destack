use destack_dir as dir;

use crate::check::CheckModuleState;

impl CheckModuleState {
    /// Walk one pattern and collect check work.
    pub(in crate::check) fn walk_pattern(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Pattern>,
        pattern: &dir::Pattern,
    ) {
        dir::walk_pattern(self, tree, id, pattern);
    }

    /// Walk one pattern field and collect check work.
    pub(in crate::check) fn walk_pattern_field(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::PatternField>,
        pattern_field: &dir::PatternField,
    ) {
        dir::walk_pattern_field(self, tree, id, pattern_field);
    }

    /// Walk one assign pattern and collect check work.
    pub(in crate::check) fn walk_assign_pattern(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::AssignPattern>,
        assign_pattern: &dir::AssignPattern,
    ) {
        dir::walk_assign_pattern(self, tree, id, assign_pattern);
    }

    /// Walk one assign pattern field and collect check work.
    pub(in crate::check) fn walk_assign_pattern_field(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::AssignPatternField>,
        assign_pattern_field: &dir::AssignPatternField,
    ) {
        dir::walk_assign_pattern_field(self, tree, id, assign_pattern_field);
    }
}
