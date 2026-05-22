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

    fn visit_block(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Block>,
        block: &dir::Block,
    ) {
        self.walk_block(tree, id, block);
    }

    fn visit_catch(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Catch>,
        catch: &dir::Catch,
    ) {
        self.walk_catch(tree, id, catch);
    }

    fn visit_declaration(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) {
        self.walk_declaration(tree, id, declaration);
    }

    fn visit_declarator(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declarator>,
        declarator: &dir::Declarator,
    ) {
        self.walk_declarator(tree, id, declarator);
    }

    fn visit_property(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Property>,
        property: &dir::Property,
    ) {
        self.walk_property(tree, id, property);
    }

    fn visit_type_member(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeMember>,
        type_member: &dir::TypeMember,
    ) {
        self.walk_type_member(tree, id, type_member);
    }

    fn visit_type_mapped_parameter(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeMappedParameter>,
        type_mapped_parameter: &dir::TypeMappedParameter,
    ) {
        self.walk_type_mapped_parameter(tree, id, type_mapped_parameter);
    }

    fn visit_member(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Member>,
        member: &dir::Member,
    ) {
        self.walk_member(tree, id, member);
    }

    fn visit_enum_field(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::EnumField>,
        enum_field: &dir::EnumField,
    ) {
        self.walk_enum_field(tree, id, enum_field);
    }

    fn visit_where_clause(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::WhereClause>,
        where_clause: &dir::WhereClause,
    ) {
        self.walk_where_clause(tree, id, where_clause);
    }

    fn visit_dependency_item(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::DependencyItem>,
        dependency_item: &dir::DependencyItem,
    ) {
        self.walk_dependency_item(tree, id, dependency_item);
    }

    fn visit_generic_parameter(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::GenericParameter>,
        generic_parameter: &dir::GenericParameter,
    ) {
        self.walk_generic_parameter(tree, id, generic_parameter);
    }

    fn visit_parameter(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Parameter>,
        parameter: &dir::Parameter,
    ) {
        self.walk_parameter(tree, id, parameter);
    }

    fn visit_generic_argument(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::GenericArgument>,
        generic_argument: &dir::GenericArgument,
    ) {
        self.walk_generic_argument(tree, id, generic_argument);
    }

    fn visit_tuple_element(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TupleElement>,
        tuple_element: &dir::TupleElement,
    ) {
        self.walk_tuple_element(tree, id, tuple_element);
    }

    fn visit_argument(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Argument>,
        argument: &dir::Argument,
    ) {
        self.walk_argument(tree, id, argument);
    }

    fn visit_match_case(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::MatchCase>,
        match_case: &dir::MatchCase,
    ) {
        self.walk_match_case(tree, id, match_case);
    }

    fn visit_pattern(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Pattern>,
        pattern: &dir::Pattern,
    ) {
        self.walk_pattern(tree, id, pattern);
    }

    fn visit_pattern_field(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::PatternField>,
        pattern_field: &dir::PatternField,
    ) {
        self.walk_pattern_field(tree, id, pattern_field);
    }

    fn visit_assign_pattern(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::AssignPattern>,
        assign_pattern: &dir::AssignPattern,
    ) {
        self.walk_assign_pattern(tree, id, assign_pattern);
    }

    fn visit_assign_pattern_field(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::AssignPatternField>,
        assign_pattern_field: &dir::AssignPatternField,
    ) {
        self.walk_assign_pattern_field(tree, id, assign_pattern_field);
    }

    fn visit_decorator(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Decorator>,
        decorator: &dir::Decorator,
    ) {
        self.walk_decorator(tree, id, decorator);
    }
}
