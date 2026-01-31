use destack_dir::{
    DependencyItem, DependencyKind, Export, Expression, GlobalScopeId, GlobalSymbolId, LocalNodeId,
    LocalNodeIdAny, LocalScopeId, LocalScopeMark, LocalSymbolId, ModuleBindingExports,
    ModuleTarget, NamespaceExport, NodeTree, Scope, StaticKey, StringId, SymbolSpace,
    SymbolSpaceOrder, SymbolTable,
};
use destack_source::ModuleId;
use destack_workspace::ModuleDir;
use indexmap::IndexMap;
use rustc_hash::FxHashMap;

/// Cache resolve results for expressions and scopes.
#[derive(Debug, Default)]
pub(super) struct ResolveExpressionCache {
    /// The absolute symbol cache.
    absolute_symbols: FxHashMap<ResolveAbsoluteSymbolCacheKey, LocalSymbolId>,
    /// The resolved root expression cache.
    path_roots: FxHashMap<ResolvePathCacheKey, Expression>,
    /// The scope index cache.
    scope_indices: ResolveScopeIndexCache,
}

impl ResolveExpressionCache {
    /// Return the cached absolute symbol for a key.
    pub(super) fn absolute_symbol(
        &self,
        key: ResolveAbsoluteSymbolCacheKey,
    ) -> Option<LocalSymbolId> {
        self.absolute_symbols.get(&key).copied()
    }

    /// Store a resolved absolute symbol.
    pub(super) fn insert_absolute_symbol(
        &mut self,
        key: ResolveAbsoluteSymbolCacheKey,
        symbol_id: LocalSymbolId,
    ) {
        self.absolute_symbols.insert(key, symbol_id);
    }

    /// Return the cached root expression for a key.
    pub(super) fn path_root(&self, key: ResolvePathCacheKey) -> Option<Expression> {
        self.path_roots.get(&key).cloned()
    }

    /// Store a resolved root expression.
    pub(super) fn insert_path_root(&mut self, key: ResolvePathCacheKey, value: Expression) {
        self.path_roots.insert(key, value);
    }

    /// Return the scope index cache for mutation.
    pub(super) fn scope_indices(&mut self) -> &mut ResolveScopeIndexCache {
        &mut self.scope_indices
    }
}

/// Cache for dependency resolution within a module.
#[derive(Debug, Default)]
pub(super) struct ResolveDependencyItemCache {
    /// Cached remote symbol resolutions by target, key, and kind.
    pub(super) remote_symbols: FxHashMap<RemoteSymbolCacheKey, (GlobalSymbolId, DependencyKind)>,
    /// Cached reexport chain lookups by target, space, and key.
    pub(super) reexport_chain_symbols: FxHashMap<ReexportChainCacheKey, Option<GlobalSymbolId>>,
    /// Cached dependency item ids by module.
    dependency_item_ids: FxHashMap<ModuleId, Vec<LocalNodeId<DependencyItem>>>,
    /// Cached export tables for resolved modules.
    module_exports: FxHashMap<ModuleId, IndexMap<(SymbolSpace, StaticKey), Export>>,
    /// Cached export tables for module bindings.
    pub(super) binding_exports: FxHashMap<BindingExportCacheKey, Option<ModuleBindingExports>>,
    /// Cached namespace symbol lookups.
    pub(super) namespace_symbols: FxHashMap<NamespaceSymbolCacheKey, Option<GlobalSymbolId>>,
    /// Cached namespace exports grouped by scope.
    pub(super) namespace_exports_by_scope:
        FxHashMap<ModuleId, FxHashMap<LocalScopeId, Vec<NamespaceExport>>>,
    /// Cached namespace export lists by target.
    pub(super) namespace_exports: FxHashMap<TargetCacheKey, Vec<(ModuleId, NamespaceExport)>>,
    /// Cached namespace export resolutions by target.
    pub(super) namespace_export_symbols: FxHashMap<NamespaceExportSymbolCacheKey, GlobalSymbolId>,
    /// Cached export assignment targets by target.
    pub(super) export_assignment_targets: FxHashMap<TargetCacheKey, Option<ExportAssignmentTarget>>,
    /// Cached import redirect targets by scope and name.
    pub(super) import_redirects_by_scope:
        FxHashMap<ModuleId, FxHashMap<LocalScopeId, FxHashMap<StringId, ModuleTarget>>>,
    /// Cached scope symbol indices for name lookups.
    scope_indices: ResolveScopeIndexCache,
}

impl ResolveDependencyItemCache {
    /// Return the scope index cache for name lookups.
    pub(super) fn scope_indices(&mut self) -> &mut ResolveScopeIndexCache {
        &mut self.scope_indices
    }

    /// Return cached dependency item ids or compute them once.
    pub(super) fn dependency_item_ids_for(
        &mut self,
        module_id: ModuleId,
        tree: &NodeTree,
    ) -> Vec<LocalNodeId<DependencyItem>> {
        if let Some(item_ids) = self.dependency_item_ids.get(&module_id) {
            return item_ids.clone();
        }

        // collect and cache the dependency item ids
        let item_ids = tree.iter_node_ids_of_type::<DependencyItem>();
        self.dependency_item_ids.insert(module_id, item_ids.clone());

        item_ids
    }

    /// Ensure module exports are cached for fast lookups.
    pub(super) fn ensure_module_exports(&mut self, module_id: ModuleId, dir: &ModuleDir) {
        if self.module_exports.contains_key(&module_id) {
            return;
        }

        // cache the export table once per module
        let exports = dir.exported_symbols.read();
        self.module_exports.insert(module_id, exports.clone());
    }

    /// Return the cached module exports, inserting when missing.
    pub(super) fn module_exports_for(
        &mut self,
        module_id: ModuleId,
        dir: &ModuleDir,
    ) -> &mut IndexMap<(SymbolSpace, StaticKey), Export> {
        self.module_exports.entry(module_id).or_insert_with(|| {
            let exports = dir.exported_symbols.read();
            exports.clone()
        })
    }
}

/// Target of an export assignment resolution.
#[derive(Debug, Clone, Copy)]
pub(super) enum ExportAssignmentTarget {
    /// Redirect to another module's exports.
    Module(ModuleTarget),
    /// Look in a namespace symbol's members.
    Namespace(GlobalSymbolId),
}

/// Cache key for remote symbol memoization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct RemoteSymbolCacheKey {
    /// The resolved module target.
    pub(super) target: ModuleTarget,
    /// The dependency kind being resolved.
    pub(super) kind: DependencyKind,
    /// The exported symbol key.
    pub(super) key: StaticKey,
    /// The origin module when resolving module bindings.
    pub(super) origin_module_id: Option<ModuleId>,
}

/// Cache key for reexport chain memoization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct ReexportChainCacheKey {
    /// The resolved module target.
    pub(super) target: ModuleTarget,
    /// The export space being searched.
    pub(super) space: SymbolSpace,
    /// The exported symbol key.
    pub(super) key: StaticKey,
    /// The origin module when resolving module bindings.
    pub(super) origin_module_id: Option<ModuleId>,
}

/// Cache key for target based cache entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct TargetCacheKey {
    /// The resolved module target.
    pub(super) target: ModuleTarget,
    /// The origin module when resolving module bindings.
    pub(super) origin_module_id: Option<ModuleId>,
}

/// Cache key for namespace export resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct NamespaceExportSymbolCacheKey {
    /// The resolved module target.
    pub(super) target: ModuleTarget,
    /// The origin module when resolving module bindings.
    pub(super) origin_module_id: Option<ModuleId>,
    /// The dependency kind being resolved.
    pub(super) kind: DependencyKind,
    /// The exported symbol key.
    pub(super) key: StaticKey,
}

/// Cache key for module binding export table memoization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct BindingExportCacheKey {
    /// The module that owns the binding.
    pub(super) module_id: ModuleId,
    /// The binding declaration node id.
    pub(super) declaration: LocalNodeIdAny,
}

/// Cache key for namespace symbol memoization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct NamespaceSymbolCacheKey {
    /// The namespace symbol being queried.
    pub(super) symbol: GlobalSymbolId,
    /// The dependency kind being resolved.
    pub(super) kind: DependencyKind,
    /// The exported symbol key.
    pub(super) key: StaticKey,
}

/// Cache key for absolute symbol resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct ResolveAbsoluteSymbolCacheKey {
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
    pub(super) fn new(
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
pub(super) struct ResolvePathCacheKey {
    /// The module id to resolve within.
    pub(super) module_id: destack_source::ModuleId,
    /// The local scope id to resolve within.
    pub(super) scope_id: LocalScopeId,
    /// The scope mark to resolve within.
    pub(super) scope_mark: LocalScopeMark,
    /// The module binding scope id for global resolution.
    pub(super) module_binding_scope_id: Option<LocalScopeId>,
    /// The root name to resolve.
    pub(super) name: StringId,
    /// The preferred space order.
    pub(super) space_order: SymbolSpaceOrder,
}

/// Cache scope symbol indices for fast name lookups.
#[derive(Debug, Default)]
pub(super) struct ResolveScopeIndexCache {
    /// The cached scope indices.
    scopes: FxHashMap<GlobalScopeId, ScopeSymbolIndex>,
}

impl ResolveScopeIndexCache {
    /// Return the symbol index for a scope, building it when missing.
    pub(super) fn scope_index<'a>(
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
pub(super) struct ScopeSymbolIndex {
    /// The map of name keys to scoped symbol entries.
    entries: FxHashMap<StaticKey, Vec<ScopeSymbolEntry>>,
    /// The cached lookup results for end-of-scope queries.
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
    pub(super) fn lookup(
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
