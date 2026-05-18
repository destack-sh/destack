use destack_dir as dir;

use super::super::state::BindState;

impl dir::NodeVisitor for BindState<'_> {
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    fn visit_any(&mut self, _tree: &dir::Tree, ty: dir::NodeType, id: u32) {
        self.bind_node(dir::LocalNodeIdAny::new(id, ty));
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        let compiler = self.compiler;
        compiler.bind_expression(self, tree, id, expression);
    }

    fn visit_block(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Block>,
        block: &dir::Block,
    ) {
        // create block scope
        let scope_id = self.insert_child_scope(dir::ScopeKind::Block);
        self.bind_node_to_scope(id.into_any(), scope_id);

        // visit block body
        self.push_scope(scope_id);
        dir::walk_block(self, tree, id, block);
        self.pop_scope();
    }

    fn visit_declaration(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) {
        self.bind_node(id.into_any());

        // route global blocks into the package global scope
        if matches!(declaration, dir::Declaration::Global(_)) {
            self.attach_global_scope();
            self.bind_node_to_scope(id.into_any(), self.global_scope);
            self.push_scope(self.global_scope);

            // visit global declarations
            let compiler = self.compiler;
            compiler.bind_declaration_body(self, tree, declaration);

            self.pop_scope();
            return;
        }

        // bind named declarations with their owned surface scope
        let compiler = self.compiler;
        if let Some(scope_id) = compiler.bind_declaration_symbol(self, id, declaration) {
            self.bind_node_to_scope(id.into_any(), scope_id);

            // visit declaration body
            self.push_scope(scope_id);
            compiler.bind_declaration_body(self, tree, declaration);
            self.pop_scope();
        }
        // bind anonymous module blocks as namespace scopes
        else {
            let scope_id = self.insert_child_scope(dir::ScopeKind::Namespace);
            self.bind_node_to_scope(id.into_any(), scope_id);

            // visit declaration body
            self.push_scope(scope_id);
            compiler.bind_declaration_body(self, tree, declaration);
            self.pop_scope();
        }
    }

    fn visit_dependency_item(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::DependencyItem>,
        dependency_item: &dir::DependencyItem,
    ) {
        let compiler = self.compiler;
        compiler.bind_dependency_item(self, tree, id, dependency_item);
    }

    fn visit_generic_parameter(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::GenericParameter>,
        generic_parameter: &dir::GenericParameter,
    ) {
        let compiler = self.compiler;
        compiler.bind_generic_parameter(self, tree, id, generic_parameter);
    }

    fn visit_parameter(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Parameter>,
        parameter: &dir::Parameter,
    ) {
        let compiler = self.compiler;
        compiler.bind_parameter(self, tree, id, parameter);
    }

    fn visit_pattern(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Pattern>,
        pattern: &dir::Pattern,
    ) {
        self.bind_node(id.into_any());

        // bind pattern symbol
        let compiler = self.compiler;
        compiler.bind_pattern_symbol(self, id, pattern);

        // visit nested pattern structure
        dir::walk_pattern(self, tree, id, pattern);
    }

    fn visit_pattern_field(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::PatternField>,
        pattern_field: &dir::PatternField,
    ) {
        let compiler = self.compiler;
        compiler.bind_pattern_field(self, tree, id, pattern_field);
    }

    fn visit_member(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Member>,
        member: &dir::Member,
    ) {
        self.bind_node(id.into_any());
        let compiler = self.compiler;

        // visit scoped member body
        if let Some(scope_id) = compiler.bind_member_symbol(self, id, member) {
            self.bind_node_to_scope(id.into_any(), scope_id);
            self.push_scope(scope_id);
            compiler.bind_member_body(self, tree, member);
            self.pop_scope();
        }
        // visit unscoped member body
        else {
            compiler.bind_member_body(self, tree, member);
        }
    }

    fn visit_type_member(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeMember>,
        type_member: &dir::TypeMember,
    ) {
        let compiler = self.compiler;
        compiler.bind_type_member(self, tree, id, type_member);
    }

    fn visit_property(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Property>,
        property: &dir::Property,
    ) {
        let compiler = self.compiler;
        compiler.bind_property(self, tree, id, property);
    }

    fn visit_match_case(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::MatchCase>,
        match_case: &dir::MatchCase,
    ) {
        // create match case scope
        let scope_id = self.insert_child_scope(dir::ScopeKind::Block);
        self.bind_node_to_scope(id.into_any(), scope_id);

        // visit case body
        self.push_scope(scope_id);
        dir::walk_match_case(self, tree, id, match_case);
        self.pop_scope();
    }

    fn visit_declarator(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declarator>,
        declarator: &dir::Declarator,
    ) {
        let compiler = self.compiler;
        compiler.bind_declarator(self, tree, id, declarator);
    }

    fn visit_catch(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Catch>,
        catch: &dir::Catch,
    ) {
        let compiler = self.compiler;
        compiler.bind_catch(self, tree, id, catch);
    }

    fn visit_type_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeExpression>,
        type_expression: &dir::TypeExpression,
    ) {
        let compiler = self.compiler;
        compiler.bind_type_expression(self, tree, id, type_expression);
    }

    fn visit_type_mapped_parameter(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeMappedParameter>,
        _parameter: &dir::TypeMappedParameter,
    ) {
        let compiler = self.compiler;
        compiler.bind_type_mapped_parameter(self, tree, id);
    }
}
