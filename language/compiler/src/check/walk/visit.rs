use destack_dir as dir;
use dir::NodeVisitor as _;

use crate::check::{CheckModuleState, CheckResult};

impl CheckModuleState {
    /// Visit DIR and collect check constraints and obligations.
    pub(in crate::check) fn walk(&mut self) -> CheckResult<()> {
        let parsed = self.parsed_arc();
        let expanded = self.expanded_arc();
        let tree = &parsed.tree;

        for root in &expanded.roots {
            let expression = tree.get(*root);
            self.visit_expression(tree, *root, expression);
        }

        Ok(())
    }
}

impl dir::NodeVisitor for CheckModuleState {
    fn options(&self) -> &dir::NodeVisitorOptions {
        self.visitor_options()
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

    fn visit_declarator(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declarator>,
        declarator: &dir::Declarator,
    ) {
        self.walk_declarator(tree, id, declarator);
    }

    fn visit_declaration(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) {
        self.walk_declaration(tree, id, declaration);
    }

    fn visit_parameter(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Parameter>,
        parameter: &dir::Parameter,
    ) {
        self.walk_parameter(tree, id, parameter);
    }

    fn visit_member(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Member>,
        member: &dir::Member,
    ) {
        self.walk_member(tree, id, member);
    }

    fn visit_type_member(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeMember>,
        member: &dir::TypeMember,
    ) {
        self.walk_type_member(tree, id, member);
    }
}
