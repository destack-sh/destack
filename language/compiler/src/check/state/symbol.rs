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
        self.input(module)
            .binding_table()
            .symbol_for_declaration(node.into_global(module))
            .map(|symbol| symbol.into_global(module))
    }

    /// Return the implicit receiver symbol introduced for one member node.
    pub(in crate::check) fn implicit_receiver_symbol(
        &self,
        module: ModuleId,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        self.input(module)
            .binding_table()
            .implicit_receiver_symbol(node.into_global(module))
            .map(|symbol| symbol.into_global(module))
    }

    /// Return the symbol selected by a nominal member key.
    pub(in crate::check) fn member_symbol(
        &self,
        module: ModuleId,
        owner: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> Option<dir::GlobalSymbolId> {
        let binding_table = self.input(module).binding_table();

        // find the scope owned by the nominal declaration
        for scope_id in binding_table.scope_ids() {
            let scope = binding_table.get_scope_by_id(scope_id);
            if scope.owner != Some(owner.local_id) {
                continue;
            }

            return scope
                .find_symbol(key)
                .map(|symbol| symbol.into_global(module));
        }

        None
    }

    /// Return the nearest owner symbol for one source node's scope.
    pub(in crate::check) fn scope_owner_symbol(
        &self,
        module: ModuleId,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        let binding_table = self.input(module).binding_table();
        let mut current = Some(node);
        let view = self.input(module).view();

        // walk parents until a scoped symbol owner is found
        while let Some(node) = current {
            let global = node.into_global(module);
            if let Some(scope) = binding_table.scope_for_node(global) {
                let scope = binding_table.get_scope(scope);
                if let Some(owner) = scope.owner {
                    return Some(owner.into_global(module));
                }
            }

            current = view.get_parent(node.id);
        }

        None
    }

    /// Return one symbol's declaration node in this module.
    pub(in crate::check) fn symbol_source_node(
        &self,
        module: ModuleId,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<dir::LocalNodeIdAny> {
        if symbol_id.module_id != module {
            return None;
        }
        let binding_table = self.input(module).binding_table();
        let symbol = binding_table.get_symbol_maybe(symbol_id.local_id)?;

        symbol
            .declaration
            .filter(|declaration| declaration.module_id == module)
            .map(|declaration| declaration.local_id)
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

        self.input(module)
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
        if symbol.module_id == module {
            let binding_table = self.input(module).binding_table();
            let symbol = binding_table.get_symbol(symbol.local_id);

            return Some(symbol.kind);
        }

        self.imports(module)
            .symbol_kinds
            .get(&symbol)
            .copied()
    }

    /// Return whether one symbol names a transparent type constraint.
    pub(in crate::check) fn is_transparent_constraint_symbol(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> bool {
        matches!(
            self.symbol_kind(module, symbol),
            Some(
                dir::SymbolKind::AssociatedType
                    | dir::SymbolKind::Interface
                    | dir::SymbolKind::NewtypeInterface
                    | dir::SymbolKind::TypeAlias,
            )
        )
    }

    /// Return the type symbol named by one interface heritage expression.
    pub(in crate::check) fn require_interface_heritage_symbol(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        let view = self.input(module).view();
        match view.get(expression) {
            // Interface
            dir::Expression::Identifier { name } => {
                self.require_name(module, expression.into_any(), *name, dir::SymbolSpace::Type)
            }
            // Namespace.Interface
            dir::Expression::QualifiedReference {
                path,
                generic_arguments: _,
            } => {
                let [name] = path.segments.as_slice() else {
                    return None;
                };

                self.require_name(module, expression.into_any(), *name, dir::SymbolSpace::Type)
            }
            // not a heritage reference
            _ => None,
        }
    }
}
