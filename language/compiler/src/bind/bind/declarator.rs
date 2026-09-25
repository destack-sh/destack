use dir::NodeVisitor as _;
use tspp_dir as dir;

use super::super::state::{BindState, BindingModifiers};

use crate::Compiler;

impl Compiler {
    /// Bind one declarator.
    ///
    /// A later declarator rebinds the name an earlier one bound.
    pub(in crate::bind) fn bind_declarator(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declarator>,
        declarator: &dir::Declarator,
        modifiers: BindingModifiers,
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
        state.push_binding_modifiers(modifiers);
        let pattern = tree.get(declarator.pattern);
        state.visit_pattern(tree, declarator.pattern, pattern);
        state.pop_binding_modifiers();
    }
}
