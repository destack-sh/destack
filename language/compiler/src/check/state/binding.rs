use destack_dir as dir;

use super::{CheckModuleState, CheckState};

impl CheckModuleState {
    /// Return the symbol introduced by a source declaration node.
    pub(in crate::check) fn declaration_symbol(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        let symbol = self
            .binding_table()
            .symbol_for_declaration(node.into_global(self.module.id))?;

        Some(symbol.into_global(self.module.id))
    }

    /// Return the implicit receiver symbol introduced for one member node.
    pub(in crate::check) fn implicit_receiver_symbol(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        let symbol = self
            .binding_table()
            .implicit_receiver_symbol(node.into_global(self.module.id))?;

        Some(symbol.into_global(self.module.id))
    }

    /// Return the nearest owner symbol for one source node's scope.
    pub(in crate::check) fn scope_owner_symbol(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        let bindings = self.binding_table();
        let mut current = Some(node);
        let view = self.view();

        // walk parents until a scoped symbol owner is found
        while let Some(node) = current {
            let global = node.into_global(self.module.id);
            if let Some(scope) = bindings.scope_for_node(global) {
                let scope = bindings.get_scope(scope);
                if let Some(owner) = scope.owner {
                    return Some(owner.into_global(self.module.id));
                }
            }

            current = view.get_parent_any(node);
        }

        None
    }

    /// Return one source symbol's declaration node.
    pub(in crate::check) fn symbol_declaration_node(
        &self,
        symbol: dir::LocalSymbolId,
    ) -> dir::LocalNodeIdAny {
        let bindings = self.binding_table();
        let binding = bindings.get_symbol(symbol);

        // require source symbols to have local declaration nodes
        let Some(declaration) = binding.declaration else {
            panic!("check source symbol {symbol:?} has no declaration node");
        };
        if declaration.module_id != self.module.id {
            panic!("check source symbol {symbol:?} declaration points outside its module");
        }

        declaration.local_id
    }
}

impl CheckState<'_> {
    /// Return the declaration kind for one symbol.
    pub(in crate::check) fn symbol_kind(&self, symbol: dir::GlobalSymbolId) -> dir::SymbolKind {
        if let Some(state) = self.modules.get(&symbol.module_id) {
            let binding_table = state.binding_table();
            let symbol = binding_table.get_symbol(symbol.local_id);
            symbol.kind
        } else {
            let dependency = self.dependency(symbol.module_id);
            let symbol = dependency.bindings.get_symbol(symbol.local_id);
            symbol.kind
        }
    }
}
