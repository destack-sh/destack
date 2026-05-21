use destack_dir as dir;

use super::CheckModuleState;

impl CheckModuleState {
    /// Return the nearest lexical scope visible at one node.
    pub(in crate::check) fn scope_for_node(
        &self,
        bindings: &dir::BindingTable<'_>,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::LocalScope> {
        let mut current = Some(node);

        while let Some(node) = current {
            let global = node.into_global(self.module());
            if let Some(scope) = bindings.scope_for_node(global) {
                return Some(scope);
            }

            current = self.parsed().tree.get_parent(node.id);
        }

        Some(dir::LocalScope::new(
            self.bound().namespace_scope,
            dir::LocalScopeMark::end(),
        ))
    }

    /// Return lexical symbols visible from one scope.
    pub(in crate::check) fn scope_symbols(
        &self,
        bindings: &dir::BindingTable<'_>,
        mut scope: dir::LocalScope,
        key: dir::StaticKey,
        space: dir::SymbolSpace,
    ) -> Vec<dir::GlobalSymbolId> {
        loop {
            let current = bindings.get_scope(scope);
            let symbols = current
                .named_symbols_up_to(scope.mark)
                .filter_map(|(binding_key, symbol)| (binding_key == key).then_some(symbol))
                .filter(|symbol| bindings.get_symbol(*symbol).form.is_visible_in(space))
                .map(|symbol| self.resolve_imported_symbol(symbol))
                .collect::<Vec<_>>();

            if !symbols.is_empty() {
                return symbols;
            }

            let Some(parent) = current.parent else {
                return Vec::new();
            };

            scope = dir::LocalScope::new(parent.id, dir::LocalScopeMark::end());
        }
    }
}
