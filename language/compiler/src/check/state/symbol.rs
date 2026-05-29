use destack_dir as dir;
use destack_source::ModuleId;

use super::CheckState;

impl CheckState<'_> {
    /// Return the symbol introduced by a source declaration node.
    pub(in crate::check) fn declaration_symbol(
        &self,
        module: ModuleId,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        let symbol = self
            .module(module)
            .binding_table()
            .symbol_for_declaration(node.into_global(module))?;

        Some(symbol.into_global(module))
    }

    /// Return the implicit receiver symbol introduced for one member node.
    pub(in crate::check) fn implicit_receiver_symbol(
        &self,
        module: ModuleId,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        let symbol = self
            .module(module)
            .binding_table()
            .implicit_receiver_symbol(node.into_global(module))?;

        Some(symbol.into_global(module))
    }

    /// Return the symbol selected by a nominal member key.
    pub(in crate::check) fn member_symbol(
        &self,
        module: ModuleId,
        owner: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> Option<dir::GlobalSymbolId> {
        let binding_table = self.module(module).binding_table();
        let lookup = binding_table.lookup_key_member(owner.local_id, key);
        if let dir::SymbolLookup::Found(symbol) = lookup {
            return Some(symbol.into_global(module));
        }

        None
    }

    /// Return the nearest owner symbol for one source node's scope.
    pub(in crate::check) fn scope_owner_symbol(
        &self,
        module: ModuleId,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        let binding_table = self.module(module).binding_table();
        let mut current = Some(node);
        let view = self.module(module).view();

        // walk parents until a scoped symbol owner is found
        while let Some(node) = current {
            let global = node.into_global(module);
            if let Some(scope) = binding_table.scope_for_node(global) {
                let scope = binding_table.get_scope(scope);
                if let Some(owner) = scope.owner {
                    return Some(owner.into_global(module));
                }
            }

            current = view.get_parent_any(node);
        }

        None
    }

    /// Return one symbol's required declaration node.
    pub(in crate::check) fn symbol_source_node(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> dir::LocalNodeIdAny {
        match self.local_symbol_source_node(symbol_id) {
            Some(source) => source,
            None => panic!("check symbol {symbol_id:?} has no source node"),
        }
    }

    /// Return one symbol's declaration node when it belongs to a checked source module.
    pub(in crate::check) fn local_symbol_source_node(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<dir::LocalNodeIdAny> {
        let module = self.modules.get(&symbol_id.module_id)?;
        let binding_table = module.binding_table();
        let symbol = binding_table.get_symbol(symbol_id.local_id);
        let declaration = symbol.declaration?;
        if declaration.module_id != symbol_id.module_id {
            return None;
        }

        Some(declaration.local_id)
    }

    /// Return whether one local symbol is an imported alias.
    pub(in crate::check) fn is_import_symbol(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> bool {
        if symbol.module_id != module {
            return false;
        }

        self.module(module)
            .resolved
            .imports
            .symbol_target(symbol.local_id)
            .is_some()
    }

    /// Return the declaration kind for one visible symbol.
    pub(in crate::check) fn symbol_kind(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::SymbolKind> {
        if let Some(state) = self.modules.get(&symbol.module_id) {
            let binding_table = state.binding_table();
            let symbol = binding_table.get_symbol(symbol.local_id);

            return Some(symbol.kind);
        }

        if !self.module(module).dependencies.contains(&symbol.module_id) {
            return None;
        }
        let dependency = self.dependency(symbol.module_id);
        let symbol = dependency.bindings.get_symbol(symbol.local_id);

        Some(symbol.kind)
    }

    /// Return the type symbol named by one interface heritage expression.
    pub(in crate::check) fn require_interface_heritage_symbol(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        let view = self.module(module).view();
        match view.get(expression) {
            // Interface
            dir::Expression::Identifier { name } => self.require_symbol_by_name(
                module,
                expression.into_any(),
                *name,
                dir::SymbolSpace::Type,
            ),
            // Namespace.Interface
            dir::Expression::QualifiedReference {
                path,
                generic_arguments: _,
            } => {
                let [name] = path.segments.as_slice() else {
                    return None;
                };

                self.require_symbol_by_name(
                    module,
                    expression.into_any(),
                    *name,
                    dir::SymbolSpace::Type,
                )
            }
            // not a heritage reference
            _ => None,
        }
    }
}
