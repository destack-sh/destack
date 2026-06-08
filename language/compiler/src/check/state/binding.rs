use super::{CheckModuleState, CheckState};
use destack_dir as dir;

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

    /// Return one source symbol's declaration node.
    pub(in crate::check) fn symbol_declaration_node(
        &self,
        symbol: dir::LocalSymbolId,
    ) -> dir::LocalNodeIdAny {
        let bindings = self.binding_table();
        let binding = bindings.get_symbol(symbol);

        // require source symbols to have local declaration nodes
        let Some(declaration) = binding.declaration else {
            unreachable!("source symbol {symbol:?} has no declaration node");
        };
        if declaration.module_id != self.module.id {
            unreachable!("source symbol {symbol:?} declaration points outside its module");
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
