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

/// One candidate visible to source name lookup.
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
    Ambiguous(Box<SmallVec<[NameCandidate; 4]>>),
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

    /// Look up one source name in the requested symbol space.
    pub(in crate::check) fn lookup_name(
        &self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        name: dir::StringId,
        space: dir::SymbolSpace,
    ) -> NameLookup {
        // look up local bindings first
        let key = dir::StaticKey::Name(name);
        let bindings = self.module(module).binding_table();
        let scope = self.lexical_scope(module, bindings, source);
        let symbols = self.scope_symbols(module, bindings, scope, key, space);

        // fall back to imported bindings
        let mut candidates = if symbols.is_empty() {
            self.global_candidates(module, key)
        } else {
            symbols
                .into_iter()
                .filter_map(|symbol| self.name_candidate(module, symbol))
                .collect()
        };

        match candidates.as_slice() {
            // no visible binding
            [] => NameLookup::Missing,
            // exactly one visible binding
            [_] => NameLookup::Found(candidates.remove(0)),
            // multiple visible bindings
            _ => NameLookup::Ambiguous(Box::new(candidates)),
        }
    }

    /// Return one symbol's @if availability condition.
    pub(in crate::check) fn symbol_availability(&self, symbol: dir::GlobalSymbolId) -> Condition {
        let Some(module) = self.modules.get(&symbol.module_id) else {
            return Condition::Always;
        };

        module
            .availability
            .get(&symbol)
            .cloned()
            .unwrap_or(Condition::Always)
    }

    /// Return whether one symbol's guard decided statically false.
    pub(in crate::check) fn is_unavailable_symbol(&self, symbol: dir::GlobalSymbolId) -> bool {
        self.modules
            .get(&symbol.module_id)
            .is_some_and(|module| module.unavailable.contains(&symbol))
    }

    /// Return one name candidate for a lexical symbol.
    fn name_candidate(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> Option<NameCandidate> {
        // drop declarations whose guards decided statically false
        if self.is_unavailable_symbol(symbol) {
            return None;
        }
        let condition = self.symbol_availability(symbol);

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
                    if self.is_unavailable_symbol(target) {
                        return None;
                    }
                    let condition = condition.and(self.symbol_availability(target));

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
                    if self.is_unavailable_symbol(*symbol) {
                        return None;
                    }

                    Some(NameCandidate {
                        target: NameTarget::Symbol(*symbol),
                        condition: self.symbol_availability(*symbol),
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
