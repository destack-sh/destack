use destack_dir::{
    Expression, GlobalScopeId, LocalScopeId, LocalScopeMark, LocalSymbolId, Scope, StaticKey,
    StringId, SymbolSpace, SymbolSpaceOrder, SymbolTable,
};
use rustc_hash::FxHashMap;

/// Cache resolve results for expressions and scopes.
#[derive(Debug, Default)]
pub(crate) struct ResolveExpressionCache {
    /// The absolute symbol cache.
    absolute_symbols: FxHashMap<ResolveAbsoluteSymbolCacheKey, LocalSymbolId>,
    /// The resolved root expression cache.
    path_roots: FxHashMap<ResolvePathCacheKey, Expression>,
    /// The scope index cache.
    scope_indices: ResolveScopeIndexCache,
}

impl ResolveExpressionCache {
    /// Return the cached absolute symbol for a key.
    pub(crate) fn absolute_symbol(
        &self,
        key: ResolveAbsoluteSymbolCacheKey,
    ) -> Option<LocalSymbolId> {
        self.absolute_symbols.get(&key).copied()
    }

    /// Store a resolved absolute symbol.
    pub(crate) fn insert_absolute_symbol(
        &mut self,
        key: ResolveAbsoluteSymbolCacheKey,
        symbol_id: LocalSymbolId,
    ) {
        self.absolute_symbols.insert(key, symbol_id);
    }

    /// Return the cached root expression for a key.
    pub(crate) fn path_root(&self, key: ResolvePathCacheKey) -> Option<Expression> {
        self.path_roots.get(&key).cloned()
    }

    /// Store a resolved root expression.
    pub(crate) fn insert_path_root(&mut self, key: ResolvePathCacheKey, value: Expression) {
        self.path_roots.insert(key, value);
    }

    /// Return the scope index cache for mutation.
    pub(crate) fn scope_indices(&mut self) -> &mut ResolveScopeIndexCache {
        &mut self.scope_indices
    }
}

/// Cache key for absolute symbol resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ResolveAbsoluteSymbolCacheKey {
    /// The module id to resolve within.
    module_id: destack_source::ModuleId,
    /// The local scope id to resolve within.
    scope_id: LocalScopeId,
    /// The scope mark to resolve within.
    scope_mark: LocalScopeMark,
    /// The symbol name to resolve.
    name: StringId,
    /// The preferred space order.
    space_order: SymbolSpaceOrder,
}

impl ResolveAbsoluteSymbolCacheKey {
    /// Build a cache key for absolute symbol resolution.
    pub(crate) fn new(
        module_id: destack_source::ModuleId,
        scope_id: LocalScopeId,
        scope_mark: LocalScopeMark,
        name: StringId,
        space_order: SymbolSpaceOrder,
    ) -> Self {
        Self {
            module_id,
            scope_id,
            scope_mark,
            name,
            space_order,
        }
    }
}

/// Cache key for root path resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ResolvePathCacheKey {
    /// The module id to resolve within.
    pub(crate) module_id: destack_source::ModuleId,
    /// The local scope id to resolve within.
    pub(crate) scope_id: LocalScopeId,
    /// The scope mark to resolve within.
    pub(crate) scope_mark: LocalScopeMark,
    /// The module binding scope id for global resolution.
    pub(crate) module_binding_scope_id: Option<LocalScopeId>,
    /// The root name to resolve.
    pub(crate) name: StringId,
    /// The preferred space order.
    pub(crate) space_order: SymbolSpaceOrder,
}

/// Cache scope symbol indices for fast name lookups.
#[derive(Debug, Default)]
pub(crate) struct ResolveScopeIndexCache {
    /// The cached scope indices.
    scopes: FxHashMap<GlobalScopeId, ScopeSymbolIndex>,
}

impl ResolveScopeIndexCache {
    /// Return the symbol index for a scope, building it when missing.
    pub(crate) fn scope_index<'a>(
        &'a mut self,
        scope_id: LocalScopeId,
        scope: &Scope,
        symbols: &SymbolTable,
    ) -> &'a mut ScopeSymbolIndex {
        let global_scope_id = GlobalScopeId::new(scope.module_id, scope_id);
        self.scopes
            .entry(global_scope_id)
            .or_insert_with(|| ScopeSymbolIndex::build(scope, symbols))
    }
}

/// Cached scope entry metadata for symbol lookups.
#[derive(Debug, Clone, Copy)]
struct ScopeSymbolEntry {
    /// The declaration index inside the scope.
    index: u32,
    /// The resolved symbol id.
    symbol_id: LocalSymbolId,
    /// The symbol space.
    space: SymbolSpace,
}

/// A cached index of symbols for a single scope.
#[derive(Debug, Default)]
pub(crate) struct ScopeSymbolIndex {
    /// The map of name keys to scoped symbol entries.
    entries: FxHashMap<StaticKey, Vec<ScopeSymbolEntry>>,
    /// The cached lookup results for end of scope queries.
    resolved_end:
        FxHashMap<(StaticKey, SymbolSpaceOrder), (Option<LocalSymbolId>, Option<LocalSymbolId>)>,
}

impl ScopeSymbolIndex {
    /// Build a symbol index from the scope's active named symbols.
    fn build(scope: &Scope, symbols: &SymbolTable) -> Self {
        let mut entries: FxHashMap<StaticKey, Vec<ScopeSymbolEntry>> = FxHashMap::default();
        for (index, (key, symbol_id)) in scope.named_symbols.iter().enumerate() {
            let symbol = symbols.get_symbol(*symbol_id);
            if !symbol.is_active() {
                continue;
            }
            entries.entry(*key).or_default().push(ScopeSymbolEntry {
                index: index as u32,
                symbol_id: *symbol_id,
                space: symbol.space,
            });
        }
        Self {
            entries,
            resolved_end: FxHashMap::default(),
        }
    }

    /// Lookup the preferred and fallback symbols for a key within a scope mark.
    pub(crate) fn lookup(
        &mut self,
        key: StaticKey,
        mark: LocalScopeMark,
        space_order: SymbolSpaceOrder,
    ) -> (Option<LocalSymbolId>, Option<LocalSymbolId>) {
        if mark == LocalScopeMark::end()
            && let Some(cached) = self.resolved_end.get(&(key, space_order))
        {
            return *cached;
        }

        let result = self.lookup_uncached(key, mark, space_order);
        if mark == LocalScopeMark::end() {
            self.resolved_end.insert((key, space_order), result);
        }

        result
    }

    /// Lookup symbols without using cached end of scope results.
    fn lookup_uncached(
        &self,
        key: StaticKey,
        mark: LocalScopeMark,
        space_order: SymbolSpaceOrder,
    ) -> (Option<LocalSymbolId>, Option<LocalSymbolId>) {
        // resolve the entry list for this key
        let Some(entries) = self.entries.get(&key) else {
            return (None, None);
        };

        // find the last entry below the scope mark
        let limit = mark.0;
        let mut low = 0;
        let mut high = entries.len();
        while low < high {
            let mid = (low + high) / 2;
            if entries[mid].index < limit {
                low = mid + 1;
            } else {
                high = mid;
            }
        }

        // return early when there are no entries in scope
        if low == 0 {
            return (None, None);
        }

        // scan backwards to locate the nearest matches
        let mut type_symbol = None;
        let mut value_symbol = None;
        let mut type_value_symbol = None;
        let mut fallback = None;

        let mut index = low;
        while index > 0 {
            index -= 1;
            let entry = entries[index];
            if fallback.is_none() {
                fallback = Some(entry.symbol_id);
            }
            match entry.space {
                SymbolSpace::Type => {
                    if type_symbol.is_none() {
                        type_symbol = Some(entry.symbol_id);
                    }
                }
                SymbolSpace::Value => {
                    if value_symbol.is_none() {
                        value_symbol = Some(entry.symbol_id);
                    }
                }
                SymbolSpace::TypeValue => {
                    if type_value_symbol.is_none() {
                        type_value_symbol = Some(entry.symbol_id);
                    }
                }
                SymbolSpace::Label => {}
            }

            // stop once all preferred spaces are found
            if type_symbol.is_some() && value_symbol.is_some() && type_value_symbol.is_some() {
                break;
            }
        }

        // select the preferred symbol for the requested space order
        let preferred = space_order.spaces().iter().find_map(|space| match space {
            SymbolSpace::Type => type_symbol.or(type_value_symbol),
            SymbolSpace::Value => value_symbol.or(type_value_symbol),
            SymbolSpace::TypeValue => type_value_symbol,
            SymbolSpace::Label => None,
        });

        (preferred, fallback)
    }
}
