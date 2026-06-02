use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{CheckState, Condition};

/// One symbol visible to source name lookup.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct SymbolCandidate {
    /// The visible symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The condition under which the symbol exists.
    pub(in crate::check) condition: Condition,
}

/// Result of looking up one source name.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum NameLookup {
    /// Exactly one visible symbol matched.
    Found(SymbolCandidate),
    /// No visible symbol matched.
    Missing,
    /// More than one visible symbol matched.
    Ambiguous(SmallVec<[SymbolCandidate; 4]>),
}

impl NameLookup {
    /// Return the unique symbol when lookup found exactly one target.
    pub(in crate::check) fn unique_symbol(&self) -> Option<dir::GlobalSymbolId> {
        match self {
            Self::Found(candidate) => Some(candidate.symbol),
            Self::Missing | Self::Ambiguous(_) => None,
        }
    }

    /// Return candidates whose availability is guaranteed by one active guard.
    pub(in crate::check) fn available_under(self, guard: &Condition) -> Self {
        // collect visible candidates
        let candidates = match self {
            Self::Found(candidate) => smallvec::smallvec![candidate],
            Self::Missing => return Self::Missing,
            Self::Ambiguous(candidates) => candidates,
        };

        // remove candidates not guaranteed in this branch
        let mut candidates = candidates
            .into_iter()
            .filter(|candidate| candidate.condition.is_guaranteed_by(guard))
            .collect::<SmallVec<_>>();

        // preserve lookup cardinality after filtering
        if candidates.is_empty() {
            Self::Missing
        } else if candidates.len() == 1 {
            let candidate = candidates.remove(0);

            Self::Found(candidate)
        } else {
            Self::Ambiguous(candidates)
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
        // collect hoisted type bindings
        if space == dir::SymbolSpace::Type {
            return self.visible_hoisted_type_symbols(module, bindings, scope, key);
        }

        // collect ordinary lexical bindings
        match bindings.lookup_symbol_from_scope(scope, key, space) {
            dir::SymbolLookup::Found(symbol) => {
                smallvec::smallvec![symbol.into_global(module)]
            }
            dir::SymbolLookup::Ambiguous(symbols) => symbols
                .into_iter()
                .map(|symbol| symbol.into_global(module))
                .collect(),
            dir::SymbolLookup::Missing => SmallVec::new(),
        }
    }

    /// Return hoisted type symbols visible from one lexical scope.
    fn visible_hoisted_type_symbols(
        &self,
        module: ModuleId,
        bindings: &dir::BindingTable<'_>,
        mut scope: dir::LocalScope,
        key: dir::StaticKey,
    ) -> SmallVec<[dir::GlobalSymbolId; 4]> {
        loop {
            let current = bindings.get_scope(scope);
            let mut symbols = SmallVec::new();

            // collect full-scope type bindings at this lexical level
            current.for_symbols_by_key(key, |symbol| {
                if bindings
                    .get_symbol(symbol)
                    .kind
                    .is_visible_in(dir::SymbolSpace::Type)
                {
                    symbols.push(symbol.into_global(module));
                }
            });
            if !symbols.is_empty() {
                return symbols;
            }

            let Some(parent) = current.parent else {
                return SmallVec::new();
            };
            scope = parent;
        }
    }

    /// Require one guarded source name in the requested symbol space.
    pub(in crate::check) fn require_symbol_by_name_under(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        name: dir::StringId,
        space: dir::SymbolSpace,
        guard: &Condition,
    ) -> Option<dir::GlobalSymbolId> {
        match self
            .lookup_symbol_by_name(module, source, name, space)
            .available_under(guard)
        {
            NameLookup::Found(candidate) => Some(candidate.symbol),
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
        // look up local bindings first
        let key = dir::StaticKey::Name(name);
        let bindings = self.module(module).binding_table();
        let scope = self.visible_scope(module, &bindings, source);
        let symbols = self.visible_scope_symbols(module, &bindings, scope, key, space);

        // fall back to imported bindings
        let symbols = if symbols.is_empty() {
            self.module(module)
                .resolved
                .imports
                .global_symbols(key)
                .map_or_else(SmallVec::new, SmallVec::from_slice)
        } else {
            symbols
        };

        let candidates = self.visible_symbol_candidates(symbols);
        match candidates.as_slice() {
            // no visible binding
            [] => NameLookup::Missing,
            // exactly one visible binding
            [candidate] => NameLookup::Found(candidate.clone()),
            // multiple visible bindings
            _ => NameLookup::Ambiguous(candidates),
        }
    }

    /// Return candidates for symbols that are not statically absent.
    fn visible_symbol_candidates(
        &self,
        symbols: SmallVec<[dir::GlobalSymbolId; 4]>,
    ) -> SmallVec<[SymbolCandidate; 4]> {
        symbols
            .into_iter()
            .filter_map(|symbol| {
                let condition = self.symbol_availability(symbol);
                if condition.is_never() {
                    return None;
                }

                Some(SymbolCandidate { symbol, condition })
            })
            .collect()
    }
}
