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
            dir::TypeExpression::FunctionTypeDeclaration(function) => {
                // bind callable type surface
                self.bind_function_type_declaration(state, tree, id, function)
            }
            dir::TypeExpression::ConstructorTypeDeclaration(function) => {
                // bind constructor type surface
                self.bind_constructor_type_declaration(state, tree, id, function)
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

    /// Bind one function type declaration.
    fn bind_function_type_declaration(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeExpression>,
        function: &dir::FunctionTypeDeclaration,
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

    /// Bind one constructor type declaration.
    fn bind_constructor_type_declaration(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeExpression>,
        function: &dir::ConstructorTypeDeclaration,
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
            dir::SymbolForm::TypeAlias,
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

        // declare named inferred type parameter
        if let Some(name) = name {
            let symbol_id = state.insert_symbol(
                dir::SymbolRole::Local,
                dir::SymbolForm::TypeAlias,
                Some(dir::StaticKey::Name(name)),
                None,
            );
            state.declare_symbol(symbol_id, id);
        }

        // bind optional infer constraint
        if let Some(constraint) = constraint {
            let constraint_node = tree.get(constraint);
            state.visit_type_expression(tree, constraint, constraint_node);
        }
    }
}
