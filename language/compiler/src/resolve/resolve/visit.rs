use destack_dir as dir;
use dir::NodeVisitor as _;

use crate::resolve::state::ResolveState;

impl ResolveState<'_> {
    /// Walk active roots and collect module clauses.
    pub(in crate::resolve) fn walk(&mut self, roots: &[dir::LocalNodeId<dir::Expression>]) {
        let tree = self.view.tree();

        for root in roots {
            let expression = tree.get(*root);
            self.visit_expression(tree, *root, expression);
        }
    }
}

impl dir::NodeVisitor for ResolveState<'_> {
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        self.walk_expression(tree, id, expression);
    }

    fn visit_type_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeExpression>,
        type_expression: &dir::TypeExpression,
    ) {
        self.walk_type_expression(tree, id, type_expression);
    }

    fn visit_declaration(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) {
        if let dir::Declaration::Function(function) = declaration {
            self.require_function_language_items(&function.signature);
            self.enter_function(&function.signature);
            dir::walk_declaration(self, tree, id, declaration);
            self.leave_function();

            return;
        }

        dir::walk_declaration(self, tree, id, declaration);
    }

    fn visit_property(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Property>,
        property: &dir::Property,
    ) {
        if let dir::Property::Method { signature, .. } = property {
            self.require_function_language_items(signature);
            self.enter_function(signature);
            dir::walk_property(self, tree, id, property);
            self.leave_function();

            return;
        }

        dir::walk_property(self, tree, id, property);
    }

    fn visit_type_member(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeMember>,
        member: &dir::TypeMember,
    ) {
        if let dir::TypeMember::Method { signature, .. } = member {
            self.require_function_language_items(signature);
        }

        dir::walk_type_member(self, tree, id, member);
    }

    fn visit_member(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Member>,
        member: &dir::Member,
    ) {
        if let dir::Member::Method { signature, .. } = member {
            self.require_function_language_items(signature);
            self.enter_function(signature);
            dir::walk_member(self, tree, id, member);
            self.leave_function();

            return;
        }

        dir::walk_member(self, tree, id, member);
    }
}
