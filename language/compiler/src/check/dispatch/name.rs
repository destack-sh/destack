use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::CheckState;

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

impl CheckState<'_> {
    /// Return the nearest lexical scope visible at one node.
    pub(in crate::check) fn visible_scope(
        &self,
        module: ModuleId,
        bindings: &dir::BindingTable<'_>,
        node: dir::LocalNodeIdAny,
    ) -> dir::LocalScope {
        let mut current = Some(node);
        let view = self.module(module).view();

        // find nearest parent with a scope
        while let Some(node) = current {
            let global = node.into_global(module);
            if let Some(scope) = bindings.scope_for_node(global) {
                return scope;
            }
            current = view.get_parent(node.id);
        }

        // use module namespace when no child scope owns the node
        dir::LocalScope::new(
            self.module(module).bound.namespace_scope,
            dir::LocalScopeMark::end(),
        )
    }

    /// Return lexical symbols visible from one scope.
    pub(in crate::check) fn visible_scope_symbols(
        &self,
        module: ModuleId,
        bindings: &dir::BindingTable<'_>,
        scope: dir::LocalScope,
        key: dir::StaticKey,
        space: dir::SymbolSpace,
    ) -> SmallVec<[dir::GlobalSymbolId; 4]> {
        match bindings.lookup_symbol_from_scope(scope, key, space) {
            dir::SymbolLookup::Found(symbol) => {
                smallvec::smallvec![self.binding_target_symbol(module, symbol)]
            }
            dir::SymbolLookup::Ambiguous(symbols) => symbols
                .into_iter()
                .map(|symbol| self.binding_target_symbol(module, symbol))
                .collect(),
            dir::SymbolLookup::Missing => SmallVec::new(),
        }
    }

    /// Return the declaration symbol selected by one local binding.
    pub(in crate::check) fn binding_target_symbol(
        &self,
        module: ModuleId,
        symbol: dir::LocalSymbolId,
    ) -> dir::GlobalSymbolId {
        match self.module(module).resolved.imports.symbol_target(symbol) {
            // imported aliases use their resolved target
            Some(dir::ImportTarget::Symbol(symbol)) => symbol,

            // local and namespace bindings keep their local symbol
            Some(dir::ImportTarget::Namespace(_)) | None => symbol.into_global(module),
        }
    }

    /// Require one source name in the requested symbol space.
    pub(in crate::check) fn require_symbol_by_name(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        name: dir::StringId,
        space: dir::SymbolSpace,
    ) -> Option<dir::GlobalSymbolId> {
        match self.lookup_symbol_by_name(module, source, name, space) {
            NameLookup::Found(symbol) => Some(symbol),
            NameLookup::Missing => {
                let path = dir::Path {
                    segments: smallvec::smallvec![name],
                };

                self.report_unresolved_reference(module, source, &path);

                None
            }
            NameLookup::Ambiguous(_) => {
                let path = dir::Path {
                    segments: smallvec::smallvec![name],
                };

                self.report_ambiguous_reference(module, source, &path);

                None
            }
        }
    }

    /// Look up one source name in the requested symbol space.
    pub(in crate::check) fn lookup_symbol_by_name(
        &self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        name: dir::StringId,
        space: dir::SymbolSpace,
    ) -> NameLookup {
        let key = dir::StaticKey::Name(name);
        let bindings = self.module(module).binding_table();
        let scope = self.visible_scope(module, &bindings, source);
        let symbols = self.visible_scope_symbols(module, &bindings, scope, key, space);
        let symbols = if symbols.is_empty() {
            self.module(module)
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
