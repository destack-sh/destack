use dir::NodeVisitor as _;
use tspp_dir as dir;

use super::super::state::BindState;

use crate::Compiler;

impl Compiler {
    /// Bind one named declaration symbol and its owned scope.
    pub(in crate::bind) fn bind_declaration_symbol(
        &self,
        state: &mut BindState<'_>,
        node_id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) -> Option<dir::LocalScopeId> {
        // ignore anonymous declarations
        let kind = declaration.symbol_kind()?;
        let role = declaration.symbol_role()?;
        let scope_kind = declaration.symbol_scope_kind()?;
        let key = declaration.name().map(dir::StaticKey::from);
        let export = declaration.export();

        // declare symbol and owned scope
        let (symbol_id, scope_id) = state.insert_symbol_with_scope(
            role,
            kind,
            key,
            export,
            scope_kind,
            dir::SymbolVisibility::Scope,
        );

        state.declare_symbol(symbol_id, node_id);

        Some(scope_id)
    }

    /// Bind declaration children inside the declaration scope.
    pub(in crate::bind) fn bind_declaration_body(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        declaration: &dir::Declaration,
    ) {
        match declaration {
            dir::Declaration::Global(declaration) => {
                // visit global expressions
                for expression_id in &declaration.expressions {
                    let expression = tree.get(*expression_id);
                    state.visit_expression(tree, *expression_id, expression);
                }
            }
            dir::Declaration::Module(declaration) => {
                // visit module expressions
                for expression_id in &declaration.expressions {
                    let expression = tree.get(*expression_id);
                    state.visit_expression(tree, *expression_id, expression);
                }
            }
            dir::Declaration::Type(declaration) => {
                // bind type header
                self.bind_generic_parameters(state, tree, &declaration.generic_parameters);
                self.bind_where_clauses(state, tree, &declaration.where_clauses);

                // visit type value
                let value = tree.get(declaration.value);
                state.visit_type_expression(tree, declaration.value, value);
            }
            dir::Declaration::Struct(declaration) => {
                // bind struct header
                self.bind_generic_parameters(state, tree, &declaration.generic_parameters);
                self.bind_where_clauses(state, tree, &declaration.where_clauses);

                // visit implemented types
                for expression_id in &declaration.implements_types {
                    let expression = tree.get(*expression_id);
                    state.visit_type_expression(tree, *expression_id, expression);
                }

                // visit struct members
                for member_id in &declaration.members {
                    let member = tree.get(*member_id);
                    state.visit_member(tree, *member_id, member);
                }
            }
            dir::Declaration::Class(declaration) => {
                // bind class header
                self.bind_generic_parameters(state, tree, &declaration.generic_parameters);
                self.bind_where_clauses(state, tree, &declaration.where_clauses);

                // visit inherited type
                if let Some(extends_type_id) = declaration.extends_type {
                    let extends_type = tree.get(extends_type_id);
                    state.visit_type_expression(tree, extends_type_id, extends_type);
                }

                // visit implemented types
                for expression_id in &declaration.implements_types {
                    let expression = tree.get(*expression_id);
                    state.visit_type_expression(tree, *expression_id, expression);
                }

                // visit class members
                for member_id in &declaration.members {
                    let member = tree.get(*member_id);
                    state.visit_member(tree, *member_id, member);
                }
            }
            dir::Declaration::Enum(declaration) => {
                // bind enum header
                self.bind_generic_parameters(state, tree, &declaration.generic_parameters);
                self.bind_where_clauses(state, tree, &declaration.where_clauses);

                // visit implemented types
                for expression_id in &declaration.implements_types {
                    let expression = tree.get(*expression_id);
                    state.visit_type_expression(tree, *expression_id, expression);
                }

                // bind enum fields
                for field_id in &declaration.fields {
                    let field = tree.get(*field_id);
                    self.bind_enum_field(state, tree, *field_id, field);
                }

                // visit enum members
                for member_id in &declaration.members {
                    let member = tree.get(*member_id);
                    state.visit_member(tree, *member_id, member);
                }
            }
            dir::Declaration::Interface(declaration) => {
                // bind interface header
                self.bind_generic_parameters(state, tree, &declaration.generic_parameters);
                self.bind_where_clauses(state, tree, &declaration.where_clauses);

                // visit inherited types
                for extends_type_id in &declaration.extends_types {
                    let extends_type = tree.get(*extends_type_id);
                    state.visit_type_expression(tree, *extends_type_id, extends_type);
                }

                // visit interface members
                for member_id in &declaration.members {
                    let member = tree.get(*member_id);
                    state.visit_type_member(tree, *member_id, member);
                }
            }
            dir::Declaration::Extension(declaration) => {
                // bind extension header
                self.bind_generic_parameters(state, tree, &declaration.generic_parameters);
                self.bind_where_clauses(state, tree, &declaration.where_clauses);

                // visit extension target
                let target_type_expression = tree.get(declaration.target_type);
                state.visit_type_expression(tree, declaration.target_type, target_type_expression);

                // visit implemented types
                for expression_id in &declaration.implements_types {
                    let expression = tree.get(*expression_id);
                    state.visit_type_expression(tree, *expression_id, expression);
                }

                // visit extension members
                for member_id in &declaration.members {
                    let member = tree.get(*member_id);
                    state.visit_member(tree, *member_id, member);
                }
            }
            dir::Declaration::Function(declaration) => {
                // bind function signature
                self.bind_function_signature(state, tree, &declaration.signature);

                // visit function body
                if let Some(body_id) = declaration.body {
                    let expression = tree.get(body_id);
                    state.visit_expression(tree, body_id, expression);
                }
            }
        }
    }

    /// Bind one enum field symbol and visit its value.
    fn bind_enum_field(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::EnumField>,
        field: &dir::EnumField,
    ) {
        state.bind_node(id.into_any());

        // declare enum constant
        let symbol_id = state.insert_symbol(
            dir::SymbolRole::Item,
            dir::SymbolKind::Variant,
            Some(field.name.into()),
            None,
            dir::SymbolVisibility::Member,
        );
        state.declare_symbol(symbol_id, id);

        // visit enum value
        if let Some(value_id) = field.value {
            let value = tree.get(value_id);
            state.visit_expression(tree, value_id, value);
        }
    }
}
