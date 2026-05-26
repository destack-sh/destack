use destack_dir as dir;

use super::super::state::BindState;

use crate::Compiler;

impl Compiler {
    /// Bind one binding pattern symbol.
    pub(in crate::bind) fn bind_pattern_symbol(
        &self,
        state: &mut BindState<'_>,
        node_id: dir::LocalNodeId<dir::Pattern>,
        pattern: &dir::Pattern,
    ) {
        // ignore non binding patterns
        let dir::Pattern::Binding { name, .. } = pattern else {
            return;
        };

        // declare pattern symbol
        let binding = state.binding();
        let symbol_id = state.insert_symbol(
            dir::SymbolRole::Local,
            dir::SymbolKind::Variable,
            Some(dir::StaticKey::Name(*name)),
            binding.export,
        );

        state.set_binding_mutability(symbol_id, binding.mutability);
        state.declare_symbol(symbol_id, node_id);
    }

    /// Bind one binding pattern field.
    pub(in crate::bind) fn bind_pattern_field(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        node_id: dir::LocalNodeId<dir::PatternField>,
        pattern_field: &dir::PatternField,
    ) {
        state.bind_node(node_id.into_any());

        // declare shorthand field binding
        if let dir::PatternField::Named {
            name,
            pattern: None,
            ..
        } = pattern_field
        {
            let binding = state.binding();
            let symbol_id = state.insert_symbol(
                dir::SymbolRole::Local,
                dir::SymbolKind::Variable,
                Some(name.static_key()),
                binding.export,
            );

            state.set_binding_mutability(symbol_id, binding.mutability);
            state.declare_symbol(symbol_id, node_id);
        }

        // visit nested field pattern
        dir::walk_pattern_field(state, tree, node_id, pattern_field);
    }
}
