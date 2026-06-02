use destack_dir as dir;
use dir::NodeVisitor as _;

use super::super::state::BindState;

use crate::Compiler;

impl Compiler {
    /// Bind one type expression and visit its children with type-specific scope rules.
    pub(in crate::bind) fn bind_type_expression(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeExpression>,
        type_expression: &dir::TypeExpression,
    ) {
        match type_expression {
            dir::TypeExpression::Function(function) => {
                // bind callable type surface
                self.bind_function_type(state, tree, id, function)
            }
            dir::TypeExpression::Constructor(function) => {
                // bind constructor type surface
                self.bind_constructor_type(state, tree, id, function)
            }
            dir::TypeExpression::Conditional {
                left,
                extends_type,
                then_type,
                else_type,
            } => {
                // bind conditional type scope
                self.bind_conditional_type(
                    state,
                    tree,
                    id,
                    *left,
                    *extends_type,
                    *then_type,
                    *else_type,
                )
            }
            dir::TypeExpression::Mapped {
                parameter, value, ..
            } => {
                // bind mapped type scope
                self.bind_mapped_type(state, tree, id, *parameter, *value)
            }
            dir::TypeExpression::Infer {
                name, constraint, ..
            } => {
                // bind inferred type parameter
                self.bind_infer_type(state, tree, id, *name, *constraint)
            }
            _ => {
                // visit regular type children
                dir::walk_type_expression(state, tree, id, type_expression)
            }
        }
    }

    /// Bind function type shared parts.
    pub(in crate::bind) fn bind_function_type_parts(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        generic_parameters: &[dir::LocalNodeId<dir::GenericParameter>],
        where_clauses: &[dir::LocalNodeId<dir::WhereClause>],
        this_parameter: Option<dir::LocalNodeId<dir::Parameter>>,
        parameters: &[dir::LocalNodeId<dir::Parameter>],
        return_type: Option<dir::LocalNodeId<dir::TypeExpression>>,
    ) {
        // bind static parameter surface
        self.bind_generic_parameters(state, tree, generic_parameters);
        self.bind_where_clauses(state, tree, where_clauses);

        // bind runtime parameter surface
        if let Some(this_parameter) = this_parameter {
            let parameter = tree.get(this_parameter);
            state.visit_parameter(tree, this_parameter, parameter);
        }
        for parameter_id in parameters {
            let parameter = tree.get(*parameter_id);
            state.visit_parameter(tree, *parameter_id, parameter);
        }

        // bind return type
        if let Some(return_type) = return_type {
            let return_type_node = tree.get(return_type);
            state.visit_type_expression(tree, return_type, return_type_node);
        }
    }

    /// Bind one function type.
    fn bind_function_type(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeExpression>,
        function: &dir::FunctionTypeExpression,
    ) {
        // create callable type scope
        state.bind_node(id.into_any());
        let scope_id = state.insert_child_scope(dir::ScopeKind::Type);
        state.bind_node_to_scope(id.into_any(), scope_id);

        // visit callable type body
        state.push_scope(scope_id);
        self.bind_function_type_parts(
            state,
            tree,
            &function.generic_parameters,
            &function.where_clauses,
            function.this_parameter,
            &function.parameters,
            function.return_type,
        );
        state.pop_scope();
    }

    /// Bind one constructor type.
    fn bind_constructor_type(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeExpression>,
        function: &dir::ConstructorType,
    ) {
        // create constructor type scope
        state.bind_node(id.into_any());
        let scope_id = state.insert_child_scope(dir::ScopeKind::Type);
        state.bind_node_to_scope(id.into_any(), scope_id);

        // visit constructor type body
        state.push_scope(scope_id);
        self.bind_function_type_parts(
            state,
            tree,
            &function.generic_parameters,
            &function.where_clauses,
            None,
            &function.parameters,
            function.return_type,
        );
        state.pop_scope();
    }

    /// Bind one conditional type.
    fn bind_conditional_type(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeExpression>,
        left: dir::LocalNodeId<dir::TypeExpression>,
        extends_type: dir::LocalNodeId<dir::TypeExpression>,
        then_type: dir::LocalNodeId<dir::TypeExpression>,
        else_type: dir::LocalNodeId<dir::TypeExpression>,
    ) {
        // bind checked type outside infer scope
        state.bind_node(id.into_any());
        let left_node = tree.get(left);
        state.visit_type_expression(tree, left, left_node);

        // bind infer candidates inside conditional scope
        let scope_id = state.insert_child_scope(dir::ScopeKind::TypeConditional);
        state.bind_node_to_scope(id.into_any(), scope_id);
        state.push_scope(scope_id);
        let extends_node = tree.get(extends_type);
        state.visit_type_expression(tree, extends_type, extends_node);
        let then_node = tree.get(then_type);
        state.visit_type_expression(tree, then_type, then_node);
        state.pop_scope();

        // bind else branch outside infer scope
        let else_node = tree.get(else_type);
        state.visit_type_expression(tree, else_type, else_node);
    }

    /// Bind one mapped type.
    fn bind_mapped_type(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeExpression>,
        parameter: dir::LocalNodeId<dir::TypeMappedParameter>,
        value: Option<dir::LocalNodeId<dir::TypeExpression>>,
    ) {
        // create mapped type scope
        state.bind_node(id.into_any());
        let scope_id = state.insert_child_scope(dir::ScopeKind::Type);
        state.bind_node_to_scope(id.into_any(), scope_id);

        // bind mapped source and parameter
        state.push_scope(scope_id);
        self.bind_type_mapped_parameter(state, tree, parameter);
        if let Some(value) = value {
            let value_node = tree.get(value);
            state.visit_type_expression(tree, value, value_node);
        }
        state.pop_scope();
    }

    /// Bind one mapped type parameter.
    pub(in crate::bind) fn bind_type_mapped_parameter(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeMappedParameter>,
    ) {
        let parameter = tree.get(id);
        state.bind_node(id.into_any());

        // bind mapped source before parameter declaration
        let source_type = tree.get(parameter.source_type);
        state.visit_type_expression(tree, parameter.source_type, source_type);

        let symbol_id = state.insert_symbol(
            dir::SymbolRole::Local,
            dir::SymbolKind::TypeAlias,
            Some(dir::StaticKey::Name(parameter.name)),
            None,
        );
        state.declare_symbol(symbol_id, id);

        // bind optional key remap after parameter declaration
        if let Some(key_remap) = parameter.key_remap {
            let key_remap_node = tree.get(key_remap);
            state.visit_type_expression(tree, key_remap, key_remap_node);
        }
    }

    /// Bind one infer type.
    fn bind_infer_type(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeExpression>,
        name: Option<dir::StringId>,
        constraint: Option<dir::LocalNodeId<dir::TypeExpression>>,
    ) {
        // bind infer node
        state.bind_node(id.into_any());

        // bind optional infer constraint before introducing the inferred parameter
        if let Some(constraint) = constraint {
            let constraint_node = tree.get(constraint);
            state.visit_type_expression(tree, constraint, constraint_node);
        }

        // declare named inferred type parameter
        if let Some(name) = name {
            let symbol_id = state.insert_symbol(
                dir::SymbolRole::Local,
                dir::SymbolKind::TypeAlias,
                Some(dir::StaticKey::Name(name)),
                None,
            );
            state.declare_symbol(symbol_id, id);
        }
    }

    /// Bind one type member.
    pub(in crate::bind) fn bind_type_member(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeMember>,
        type_member: &dir::TypeMember,
    ) {
        state.bind_node(id.into_any());

        // visit scoped type member body
        if let Some(scope_id) = self.bind_type_member_symbol(state, id, type_member) {
            state.bind_node_to_scope(id.into_any(), scope_id);
            state.push_scope(scope_id);
            self.bind_type_member_body(state, tree, id, type_member);
            state.pop_scope();
        }
        // visit unscoped type member body
        else {
            self.bind_type_member_body(state, tree, id, type_member);
        }
    }

    /// Bind one type member symbol.
    fn bind_type_member_symbol(
        &self,
        state: &mut BindState<'_>,
        id: dir::LocalNodeId<dir::TypeMember>,
        type_member: &dir::TypeMember,
    ) -> Option<dir::LocalScopeId> {
        // ignore non symbolic type members
        let Some(key) = type_member.symbol_key() else {
            return None;
        };
        let Some(kind) = type_member.symbol_kind() else {
            return None;
        };

        // declare scoped or plain type member symbol
        let (symbol_id, scope_id) = if let Some(scope_kind) = type_member.symbol_scope_kind() {
            let (symbol_id, scope_id) = state.insert_symbol_with_scope(
                dir::SymbolRole::Item,
                kind,
                Some(key),
                None,
                scope_kind,
            );

            (symbol_id, Some(scope_id))
        } else {
            let symbol_id = state.insert_symbol(dir::SymbolRole::Item, kind, Some(key), None);

            (symbol_id, None)
        };

        state.declare_symbol(symbol_id, id);

        scope_id
    }

    /// Bind one type member body.
    fn bind_type_member_body(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeMember>,
        type_member: &dir::TypeMember,
    ) {
        // bind implicit this
        if type_member.has_implicit_receiver() {
            let has_name = matches!(
                type_member,
                dir::TypeMember::Method {
                    signature,
                    is_static: false,
                    ..
                } if signature.this_parameter.is_none()
            );
            let key = has_name.then(|| dir::StaticKey::Name(self.strings().intern("this")));
            let symbol =
                state.insert_symbol(dir::SymbolRole::Local, dir::SymbolKind::Variable, key, None);

            state.bindings.bind_implicit_receiver(id, symbol);
        }

        match type_member {
            dir::TypeMember::Field {
                key, declared_type, ..
            } => {
                // visit field key
                dir::walk_key(state, tree, key);

                // visit field type
                if let Some(declared_type) = declared_type {
                    let declared_type_node = tree.get(*declared_type);
                    state.visit_type_expression(tree, *declared_type, declared_type_node);
                }
            }
            dir::TypeMember::Method {
                key,
                signature,
                body,
                is_static: _,
                ..
            } => {
                // visit method key
                dir::walk_key(state, tree, key);

                // visit method signature
                self.bind_function_signature(state, tree, signature);

                // visit method body
                if let Some(body) = body {
                    let body_node = tree.get(*body);
                    state.visit_expression(tree, *body, body_node);
                }
            }
            dir::TypeMember::CallSignature { signature } => {
                // visit call signature
                self.bind_function_type_parts(
                    state,
                    tree,
                    &signature.generic_parameters,
                    &signature.where_clauses,
                    signature.this_parameter,
                    &signature.parameters,
                    signature.return_type,
                );
            }
            dir::TypeMember::ConstructSignature { signature } => {
                // visit constructor signature
                self.bind_function_type_parts(
                    state,
                    tree,
                    &signature.generic_parameters,
                    &signature.where_clauses,
                    None,
                    &signature.parameters,
                    signature.return_type,
                );
            }
            dir::TypeMember::IndexSignature {
                key_type,
                value_type,
                ..
            } => {
                // visit index signature types
                let key_type_node = tree.get(*key_type);
                state.visit_type_expression(tree, *key_type, key_type_node);
                let value_type_node = tree.get(*value_type);
                state.visit_type_expression(tree, *value_type, value_type_node);
            }
            dir::TypeMember::AssociatedType {
                generic_parameters,
                where_clauses,
                constraint,
                value,
                ..
            } => {
                // bind associated type header
                self.bind_generic_parameters(state, tree, generic_parameters);
                self.bind_where_clauses(state, tree, where_clauses);

                // visit associated type constraint
                if let Some(constraint) = constraint {
                    let constraint_node = tree.get(*constraint);
                    state.visit_type_expression(tree, *constraint, constraint_node);
                }

                // visit associated type value
                if let Some(value) = value {
                    let value_node = tree.get(*value);
                    state.visit_type_expression(tree, *value, value_node);
                }
            }
            dir::TypeMember::AssociatedConst {
                declared_type,
                value,
                ..
            } => {
                // visit associated const type
                if let Some(declared_type) = declared_type {
                    let declared_type_node = tree.get(*declared_type);
                    state.visit_type_expression(tree, *declared_type, declared_type_node);
                }

                // visit associated const value
                if let Some(value) = value {
                    let value_node = tree.get(*value);
                    state.visit_expression(tree, *value, value_node);
                }
            }
            dir::TypeMember::Error => {}
        }
    }
}
