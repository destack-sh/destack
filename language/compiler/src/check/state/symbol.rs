use destack_dir as dir;

use super::CheckModuleState;

impl CheckModuleState {
    /// Record one resolved name directly into the checked DIR table.
    pub(in crate::check) fn record_name_resolution(
        &mut self,
        source: dir::GlobalNodeIdAny,
        target: dir::NameResolution,
    ) {
        self.output.resolutions.set_name_resolution(source, target);
    }

    /// Return the symbol introduced by a source declaration node.
    pub(in crate::check) fn declaration_symbol(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        self.binding_table()
            .symbol_for_declaration(node.into_global(self.input.module))
            .map(|symbol| symbol.into_global(self.input.module))
    }

    /// Return the symbol selected by a nominal member key.
    pub(in crate::check) fn member_symbol(
        &self,
        owner: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> Option<dir::GlobalSymbolId> {
        let binding_table = self.binding_table();

        // find the scope owned by the nominal declaration
        for scope_id in binding_table.scope_ids() {
            let scope = binding_table.get_scope_by_id(scope_id);
            if scope.owner != Some(owner.local_id) {
                continue;
            }

            return scope
                .find_symbol(key)
                .map(|symbol| symbol.into_global(self.input.module));
        }

        None
    }

    /// Return the nearest generic owner symbol for one source node.
    pub(in crate::check) fn generic_owner_symbol(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        let binding_table = self.binding_table();
        let mut current = Some(node);
        let view = self.input.view();

        // walk parents until a scoped symbol owner is found
        while let Some(node) = current {
            let global = node.into_global(self.input.module);
            if let Some(scope) = binding_table.scope_for_node(global) {
                let scope = binding_table.get_scope(scope);
                if let Some(owner) = scope.owner {
                    return Some(owner.into_global(self.input.module));
                }
            }

            current = view.get_parent(node.id);
        }

        None
    }

    /// Return one symbol's declaration node in this module.
    pub(in crate::check) fn symbol_source_node(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<dir::LocalNodeIdAny> {
        let binding_table = self.binding_table();
        let symbol = binding_table.get_symbol(symbol_id.local_id);

        symbol
            .declaration
            .filter(|declaration| declaration.module_id == self.input.module)
            .map(|declaration| declaration.local_id)
    }

    /// Return the declaration symbol visible through one local binding.
    pub(in crate::check) fn visible_symbol(
        &self,
        symbol: dir::LocalSymbolId,
    ) -> dir::GlobalSymbolId {
        match self.input.resolved.imports.symbol_target(symbol) {
            // imported aliases use their resolved target
            Some(dir::ImportTarget::Symbol(symbol)) => symbol,

            // local and namespace bindings keep their local symbol
            Some(dir::ImportTarget::Namespace(_)) | None => symbol.into_global(self.input.module),
        }
    }

    /// Return the type symbol named by one interface heritage expression.
    pub(in crate::check) fn interface_heritage_symbol(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        let view = self.input.view();

        match view.get(expression) {
            dir::Expression::Identifier { name } => {
                let path = dir::Path {
                    segments: smallvec::smallvec![*name],
                };

                self.reference_type_symbol(expression.into_any(), &path)
            }
            dir::Expression::QualifiedReference {
                path,
                generic_arguments: _,
            } => self.reference_type_symbol(expression.into_any(), path),
            _ => None,
        }
    }

    /// Return one visible type symbol for a source reference.
    pub(in crate::check) fn reference_type_symbol(
        &self,
        source: dir::LocalNodeIdAny,
        path: &dir::Path,
    ) -> Option<dir::GlobalSymbolId> {
        if path.segments.len() != 1 {
            return None;
        }

        let key = dir::StaticKey::Name(path.segments[0]);
        let bindings = self.binding_table();
        let scope = self.visible_scope(&bindings, source);
        let symbols = self.visible_scope_symbols(&bindings, scope, key, dir::SymbolSpace::Type);

        if symbols.len() == 1 {
            symbols.first().copied()
        } else {
            None
        }
    }

    /// Return the visible checked type id for one symbol.
    pub(in crate::check) fn visible_symbol_type_id(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<dir::LocalTypeId> {
        // prefer checked tables produced during this phase
        self.output
            .types
            .get_symbol_type_id(symbol_id)
            .or_else(|| self.input_type_table().get_symbol_type_id(symbol_id))
    }

    /// Return the visible checked static value id for one symbol.
    pub(in crate::check) fn visible_symbol_static_id(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<dir::LocalStaticId> {
        // prefer checked tables produced during this phase
        self.output
            .statics
            .get_symbol_static_id(symbol_id)
            .or_else(|| self.input_static_table().get_symbol_static_id(symbol_id))
    }

    /// Return a stable local label for one symbol.
    pub(in crate::check) fn symbol_label(&self, symbol_id: dir::GlobalSymbolId) -> String {
        let bindings = self.binding_table();
        let symbol = bindings.get_symbol(symbol_id.local_id);

        // derive an internal label for unnamed owners
        if let Some(name) = symbol.name() {
            self.input.strings.get(name).to_string()
        } else {
            format!("symbol{}", symbol_id.local_id.id)
        }
    }
}
