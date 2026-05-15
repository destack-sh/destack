use destack_dir as dir;
use dir::NodeVisitor as _;

use super::super::state::BindState;

use crate::Compiler;

impl Compiler {
    /// Bind one member symbol.
    pub(in crate::bind) fn bind_member_symbol(
        &self,
        state: &mut BindState<'_>,
        node_id: dir::LocalNodeId<dir::Member>,
        member: &dir::Member,
    ) -> Option<dir::LocalScopeId> {
        // ignore non symbolic members
        let Some(key) = member.symbol_key() else {
            return None;
        };
        let Some(form) = member.symbol_form() else {
            return None;
        };

        // declare scoped or plain member symbol
        let (symbol_id, scope_id) = if let Some(scope_kind) = member.symbol_scope_kind() {
            let (symbol_id, scope_id) = state.insert_symbol_with_scope(
                dir::SymbolRole::Item,
                form,
                Some(key),
                None,
                scope_kind,
            );

            (symbol_id, Some(scope_id))
        } else {
            let symbol_id = state.insert_symbol(dir::SymbolRole::Item, form, Some(key), None);

            (symbol_id, None)
        };

        state.declare_symbol(symbol_id, node_id);

        scope_id
    }

    /// Bind member children inside the member scope.
    pub(in crate::bind) fn bind_member_body(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        member: &dir::Member,
    ) {
        match member {
            dir::Member::AssociatedType {
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
                if let Some(constraint_id) = constraint {
                    let constraint = tree.get(*constraint_id);
                    state.visit_type_expression(tree, *constraint_id, constraint);
                }

                // visit associated type value
                if let Some(value) = value {
                    let value_node = tree.get(*value);
                    state.visit_type_expression(tree, *value, value_node);
                }
            }
            dir::Member::AssociatedConst {
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
            dir::Member::Field {
                key,
                declared_type,
                default,
                ..
            } => {
                // visit field key
                dir::walk_key(state, tree, key);

                // visit field type
                if let Some(declared_type) = declared_type {
                    let declared_type_node = tree.get(*declared_type);
                    state.visit_type_expression(tree, *declared_type, declared_type_node);
                }

                // visit field default
                if let Some(default) = default {
                    let default_node = tree.get(*default);
                    state.visit_expression(tree, *default, default_node);
                }
            }
            dir::Member::Method {
                key,
                signature,
                body,
                ..
            } => {
                // visit method key
                if let Some(key) = key {
                    dir::walk_key(state, tree, key);
                }

                // visit method signature
                self.bind_function_signature(state, tree, signature);

                // visit method body
                if let Some(body) = body {
                    let body_node = tree.get(*body);
                    state.visit_expression(tree, *body, body_node);
                }
            }
            dir::Member::StaticBlock { body } | dir::Member::ComptimeBlock { body } => {
                // visit block body
                let body_node = tree.get(*body);
                state.visit_expression(tree, *body, body_node);
            }
            dir::Member::Error => {}
        }
    }
}
