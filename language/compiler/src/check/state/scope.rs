use destack_dir as dir;
use smallvec::SmallVec;

use super::CheckModuleState;

impl CheckModuleState {
    /// Return the nearest lexical scope visible at one node.
    pub(in crate::check) fn find_visible_scope(
        &self,
        bindings: &dir::BindingTable<'_>,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::LocalScope> {
        let mut current = Some(node);

        // find nearest parent with a scope
        while let Some(node) = current {
            let global = node.into_global(self.module);
            if let Some(scope) = bindings.scope_for_node(global) {
                return Some(scope);
            }
            current = self.parsed.tree.get_parent(node.id);
        }

        // use module namespace when no child scope owns the node
        Some(dir::LocalScope::new(
            self.bound.namespace_scope,
            dir::LocalScopeMark::end(),
        ))
    }

    /// Return lexical symbols visible from one scope.
    pub(in crate::check) fn find_scope_symbols(
        &self,
        bindings: &dir::BindingTable<'_>,
        mut scope: dir::LocalScope,
        key: dir::StaticKey,
        space: dir::SymbolSpace,
    ) -> SmallVec<[dir::GlobalSymbolId; 4]> {
        loop {
            // collect matching symbols in the current scope
            let current = bindings.get_scope(scope);
            let mut symbols = SmallVec::new();
            for (binding_key, symbol) in current.named_symbols_up_to(scope.mark) {
                if binding_key == key && bindings.get_symbol(symbol).form.is_visible_in(space) {
                    symbols.push(self.visible_symbol(symbol));
                }
            }

            // use nearest visible scope hits
            if !symbols.is_empty() {
                return symbols;
            }

            // climb to the parent scope
            let Some(parent) = current.parent else {
                return SmallVec::new();
            };

            scope = dir::LocalScope::new(parent.id, dir::LocalScopeMark::end());
        }
    }
}
