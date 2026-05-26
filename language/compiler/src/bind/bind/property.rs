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
        // bind keyed members through normal member lookup
        if let Some(key) = member.symbol_key() {
            let kind = member.symbol_kind()?;
            let scope_kind = member.symbol_scope_kind();

            return self.bind_member_symbol_key(state, node_id, kind, Some(key), scope_kind);
        }

        // bind role members as anonymous symbols
        let Some(slot) = member.slot() else {
            return None;
        };
        if !matches!(
            slot,
            dir::MemberSlot::Constructor | dir::MemberSlot::New | dir::MemberSlot::Call
        ) {
            return None;
        }

        self.bind_member_symbol_key(
            state,
            node_id,
            dir::SymbolKind::Function,
            None,
            Some(dir::ScopeKind::Function),
        )
    }

    /// Bind one member symbol with an optional lookup key.
    fn bind_member_symbol_key(
        &self,
        state: &mut BindState<'_>,
        node_id: dir::LocalNodeId<dir::Member>,
        kind: dir::SymbolKind,
        key: Option<dir::StaticKey>,
        scope_kind: Option<dir::ScopeKind>,
    ) -> Option<dir::LocalScopeId> {
        // declare scoped or plain member symbol
        let (symbol_id, scope_id) = if let Some(scope_kind) = scope_kind {
            let (symbol_id, scope_id) =
                state.insert_symbol_with_scope(dir::SymbolRole::Item, kind, key, None, scope_kind);

            (symbol_id, Some(scope_id))
        } else {
            let symbol_id = state.insert_symbol(dir::SymbolRole::Item, kind, key, None);

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
        id: dir::LocalNodeId<dir::Member>,
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
                is_static,
                ..
            } => {
                if signature.this_parameter.is_none() && !*is_static {
                    self.bind_implicit_this_symbol(state, id);
                }

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

    /// Bind the implicit member receiver symbol in the current method scope.
    pub(in crate::bind) fn bind_implicit_this_symbol<T: dir::Node>(
        &self,
        state: &mut BindState<'_>,
        owner: dir::LocalNodeId<T>,
    ) {
        let key = dir::StaticKey::Name(self.strings().intern("this"));
        let symbol = state.insert_symbol(
            dir::SymbolRole::Local,
            dir::SymbolKind::Variable,
            Some(key),
            None,
        );

        state.bindings.bind_implicit_receiver(owner, symbol);
    }

    /// Bind one object property.
    pub(in crate::bind) fn bind_property(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Property>,
        property: &dir::Property,
    ) {
        state.bind_node(id.into_any());

        match property {
            dir::Property::Field { key, value, .. } => {
                // visit field key and value
                dir::walk_key(state, tree, key);
                let value_node = tree.get(*value);
                state.visit_expression(tree, *value, value_node);
            }
            dir::Property::Method {
                key,
                signature,
                body,
            } => {
                // declare anonymous function backing the method value
                let (symbol_id, scope_id) = state.insert_symbol_with_scope(
                    dir::SymbolRole::Local,
                    dir::SymbolKind::Function,
                    None,
                    None,
                    dir::ScopeKind::Function,
                );

                state.declare_symbol(symbol_id, id);
                state.bind_node_to_scope(id.into_any(), scope_id);

                // visit method key
                if let Some(key) = key {
                    dir::walk_key(state, tree, key);
                }

                // visit method signature and body
                state.push_scope(scope_id);
                self.bind_function_signature(state, tree, signature);
                if let Some(body) = body {
                    let body_node = tree.get(*body);
                    state.visit_expression(tree, *body, body_node);
                }
                state.pop_scope();
            }
            dir::Property::Spread { value } => {
                // visit spread value
                let value_node = tree.get(*value);
                state.visit_expression(tree, *value, value_node);
            }
            dir::Property::Error => {}
        }
    }
}
