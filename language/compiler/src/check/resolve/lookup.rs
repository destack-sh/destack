use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::CheckModuleState;

/// Result of looking up one source name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) enum NameLookup {
    /// Exactly one visible symbol matched.
    Found(dir::GlobalSymbolId),
    /// No visible symbol matched.
    Missing,
    /// More than one visible symbol matched.
    Ambiguous(SmallVec<[dir::GlobalSymbolId; 4]>),
}

impl NameLookup {
    /// Return the unique symbol when lookup found exactly one target.
    pub(in crate::check) fn unique_symbol(&self) -> Option<dir::GlobalSymbolId> {
        match self {
            Self::Found(symbol) => Some(*symbol),
            Self::Missing | Self::Ambiguous(_) => None,
        }
    }
}

impl CheckModuleState {
    /// Return the nearest lexical scope visible at one node.
    pub(in crate::check) fn visible_scope(
        &self,
        bindings: &dir::BindingTable<'_>,
        node: dir::LocalNodeIdAny,
    ) -> dir::LocalScope {
        let mut current = Some(node);
        let view = self.input.view();

        // find nearest parent with a scope
        while let Some(node) = current {
            let global = node.into_global(self.input.module_id);
            if let Some(scope) = bindings.scope_for_node(global) {
                return scope;
            }
            current = view.get_parent(node.id);
        }

        // use module namespace when no child scope owns the node
        dir::LocalScope::new(self.input.bound.namespace_scope, dir::LocalScopeMark::end())
    }

    /// Return lexical symbols visible from one scope.
    pub(in crate::check) fn visible_scope_symbols(
        &self,
        bindings: &dir::BindingTable<'_>,
        mut scope: dir::LocalScope,
        key: dir::StaticKey,
        space: dir::SymbolSpace,
    ) -> SmallVec<[dir::GlobalSymbolId; 4]> {
        loop {
            let current = bindings.get_scope(scope);
            let mut symbols = SmallVec::new();

            // collect matching symbols in the current scope
            for (binding_key, symbol) in current.named_symbols_up_to(scope.mark) {
                if binding_key == key && bindings.get_symbol(symbol).form.is_visible_in(space) {
                    symbols.push(self.binding_target_symbol(symbol));
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

    /// Return the declaration symbol selected by one local binding.
    pub(in crate::check) fn binding_target_symbol(
        &self,
        symbol: dir::LocalSymbolId,
    ) -> dir::GlobalSymbolId {
        match self.input.resolved.imports.symbol_target(symbol) {
            // imported aliases use their resolved target
            Some(dir::ImportTarget::Symbol(symbol)) => symbol,

            // local and namespace bindings keep their local symbol
            Some(dir::ImportTarget::Namespace(_)) | None => {
                symbol.into_global(self.input.module_id)
            }
        }
    }

    /// Require one source name in the requested symbol space.
    pub(in crate::check) fn require_name(
        &mut self,
        source: dir::LocalNodeIdAny,
        name: dir::StringId,
        space: dir::SymbolSpace,
    ) -> Option<dir::GlobalSymbolId> {
        match self.lookup_name(source, name, space) {
            NameLookup::Found(symbol) => Some(symbol),
            NameLookup::Missing => {
                let path = dir::Path {
                    segments: smallvec::smallvec![name],
                };

                self.report_unresolved_reference(source, &path);

                None
            }
            NameLookup::Ambiguous(_) => {
                let path = dir::Path {
                    segments: smallvec::smallvec![name],
                };

                self.report_ambiguous_reference(source, &path);

                None
            }
        }
    }

    /// Look up one source name in the requested symbol space.
    pub(in crate::check) fn lookup_name(
        &self,
        source: dir::LocalNodeIdAny,
        name: dir::StringId,
        space: dir::SymbolSpace,
    ) -> NameLookup {
        let key = dir::StaticKey::Name(name);
        let bindings = self.input.binding_table();
        let scope = self.visible_scope(&bindings, source);
        let symbols = self.visible_scope_symbols(&bindings, scope, key, space);
        let symbols = if symbols.is_empty() {
            self.input
                .resolved
                .imports
                .global_symbols(key)
                .map_or_else(SmallVec::new, SmallVec::from_slice)
        } else {
            symbols
        };

        match symbols.as_slice() {
            // no visible binding
            [] => NameLookup::Missing,
            // exactly one visible binding
            [symbol] => NameLookup::Found(*symbol),
            // multiple visible bindings
            _ => NameLookup::Ambiguous(symbols),
        }
    }
}
