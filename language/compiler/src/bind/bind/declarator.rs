use destack_dir as dir;
use dir::NodeVisitor as _;

use super::super::state::BindState;

use crate::Compiler;

impl Compiler {
    /// Bind one declarator with Rust-like rebinding order.
    pub(in crate::bind) fn bind_declarator(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declarator>,
        declarator: &dir::Declarator,
    ) {
        state.bind_node(id.into_any());

        // visit initializer before declaring the pattern
        if let Some(value_id) = declarator.value {
            let value = tree.get(value_id);
            state.visit_expression(tree, value_id, value);
        }

        // visit declared type before declaring the pattern
        if let Some(type_id) = declarator.ty {
            let ty = tree.get(type_id);
            state.visit_type_expression(tree, type_id, ty);
        }

        // bind pattern after type and value
        let binding = state.binding();
        state.push_binding(binding);
        let pattern = tree.get(declarator.pattern);
        state.visit_pattern(tree, declarator.pattern, pattern);

        state.pop_binding();
    }
}
