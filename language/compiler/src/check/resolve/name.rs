use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{CheckState, Condition};

/// One target visible to source name lookup.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum NameTarget {
    /// A symbol target was resolved.
    Symbol(dir::GlobalSymbolId),
    /// A namespace target was resolved.
    Namespace(ModuleId),
}

/// One target visible to source name lookup.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct NameCandidate {
    /// The visible target.
    pub(in crate::check) target: NameTarget,
    /// The condition under which the target exists.
    pub(in crate::check) condition: Condition,
}

impl NameCandidate {
    /// Return the resolved symbol when this is a symbol candidate.
    pub(in crate::check) fn symbol(&self) -> Option<dir::GlobalSymbolId> {
        match self.target {
            NameTarget::Symbol(symbol) => Some(symbol),
            NameTarget::Namespace(_) => None,
        }
    }
}

/// Result of looking up one source name.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum NameLookup {
    /// Exactly one visible target matched.
    Found(NameCandidate),
    /// No visible target matched.
    Missing,
    /// More than one visible target matched.
    Ambiguous(SmallVec<[NameCandidate; 4]>),
}

impl NameLookup {
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
    pub(in crate::check) fn lexical_scope(
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
    pub(in crate::check) fn scope_symbols(
        &self,
        module: ModuleId,
        bindings: &dir::BindingTable<'_>,
        scope: dir::LocalScope,
        key: dir::StaticKey,
        space: dir::SymbolSpace,
    ) -> SmallVec<[dir::GlobalSymbolId; 4]> {
        // collect hoisted type bindings
        if space == dir::SymbolSpace::Type {
            return self.hoisted_type_symbols(module, bindings, scope, key);
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
    fn hoisted_type_symbols(
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

    /// Return one guarded source name in the requested symbol space.
    pub(in crate::check) fn symbol_by_name_under(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        name: dir::StringId,
        space: dir::SymbolSpace,
        guard: &Condition,
    ) -> Option<dir::GlobalSymbolId> {
        match self
            .lookup_name_by_name(module, source, name, space)
            .available_under(guard)
        {
            NameLookup::Found(candidate) => candidate.symbol(),
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
    pub(in crate::check) fn lookup_name_by_name(
        &self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        name: dir::StringId,
        space: dir::SymbolSpace,
    ) -> NameLookup {
        // look up local bindings first
        let key = dir::StaticKey::Name(name);
        let bindings = self.module(module).binding_table();
        let scope = self.lexical_scope(module, &bindings, source);
        let symbols = self.scope_symbols(module, &bindings, scope, key, space);

        // fall back to imported bindings
        let candidates = if symbols.is_empty() {
            self.global_candidates(module, key)
        } else {
            self.symbol_candidates(module, symbols)
        };

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
    fn symbol_candidates(
        &self,
        module: ModuleId,
        symbols: SmallVec<[dir::GlobalSymbolId; 4]>,
    ) -> SmallVec<[NameCandidate; 4]> {
        symbols
            .into_iter()
            .filter_map(|symbol| self.symbol_candidate(module, symbol))
            .collect()
    }

    /// Return one name candidate for a lexical symbol.
    fn symbol_candidate(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> Option<NameCandidate> {
        let condition = self.symbol_availability(symbol);
        if condition.is_never() {
            return None;
        }

        // expose imported targets at lookup time
        if symbol.module_id == module
            && let Some(target) = self
                .module(module)
                .resolved
                .imports
                .symbol_target(symbol.local_id)
        {
            return match target {
                dir::ImportTarget::Symbol(target) => {
                    let condition = condition.and(self.symbol_availability(target));
                    if condition.is_never() {
                        return None;
                    }

                    Some(NameCandidate {
                        target: NameTarget::Symbol(target),
                        condition,
                    })
                }
                dir::ImportTarget::Namespace(namespace) => Some(NameCandidate {
                    target: NameTarget::Namespace(namespace),
                    condition,
                }),
            };
        }

        Some(NameCandidate {
            target: NameTarget::Symbol(symbol),
            condition,
        })
    }

    /// Return candidates for imported globals that are not statically absent.
    fn global_candidates(
        &self,
        module: ModuleId,
        key: dir::StaticKey,
    ) -> SmallVec<[NameCandidate; 4]> {
        let Some(targets) = self.module(module).resolved.imports.global_targets(key) else {
            return SmallVec::new();
        };

        targets
            .iter()
            .filter_map(|target| match target {
                dir::ImportTarget::Symbol(symbol) => {
                    let condition = self.symbol_availability(*symbol);
                    if condition.is_never() {
                        return None;
                    }

                    Some(NameCandidate {
                        target: NameTarget::Symbol(*symbol),
                        condition,
                    })
                }
                dir::ImportTarget::Namespace(module) => Some(NameCandidate {
                    target: NameTarget::Namespace(*module),
                    condition: Condition::Always,
                }),
            })
            .collect()
    }
}
