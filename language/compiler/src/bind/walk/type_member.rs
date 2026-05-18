use destack_dir as dir;
use dir::NodeVisitor as _;

use super::super::state::BindState;

use crate::Compiler;

impl Compiler {
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
            self.bind_type_member_body(state, tree, type_member);
            state.pop_scope();
        }
        // visit unscoped type member body
        else {
            self.bind_type_member_body(state, tree, type_member);
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
        let Some(form) = type_member.symbol_form() else {
            return None;
        };
        // declare scoped or plain type member symbol
        let (symbol_id, scope_id) = if let Some(scope_kind) = type_member.symbol_scope_kind() {
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

        state.declare_symbol(symbol_id, id);

        scope_id
    }

    /// Bind one type member body.
    fn bind_type_member_body(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        type_member: &dir::TypeMember,
    ) {
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
