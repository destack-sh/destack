use destack_dir as dir;

use crate::check::CheckModuleState;

impl CheckModuleState {
    /// Resolve symbols visible from one source node.
    pub(in crate::check) fn resolve_name_symbols(
        &self,
        node: dir::LocalNodeIdAny,
        key: dir::StaticKey,
        space: dir::SymbolSpace,
    ) -> Vec<dir::GlobalSymbolId> {
        let bindings = self.binding_table();
        let Some(scope) = self.scope_for_node(&bindings, node) else {
            return Vec::new();
        };
        let symbols = self.scope_symbols(&bindings, scope, key, space);

        if !symbols.is_empty() {
            return symbols;
        }

        self.global_symbols(key)
            .map(<[dir::GlobalSymbolId]>::to_vec)
            .unwrap_or_default()
    }
}
