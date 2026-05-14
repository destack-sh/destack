use destack_dir as dir;
use dir::NodeVisitor as _;

use super::super::state::BindState;

use crate::Compiler;

impl Compiler {
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
                // create method scope
                let scope_id = state.insert_child_scope(dir::ScopeKind::Function);
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
