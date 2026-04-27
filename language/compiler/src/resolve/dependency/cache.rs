use destack_artifact::{DirPrepared, ExportedSymbolTable};
use destack_dir::{
    DependencyItem, DependencyKind, Export, GlobalSymbolId, LocalNodeId, LocalNodeIdAny,
    LocalScopeId, ModuleBindingExports, ModuleTarget, NamespaceExport, StaticKey, StringId,
    SymbolSpace, Tree,
};
use destack_source::ModuleId;
use indexmap::IndexMap;
use rustc_hash::FxHashMap;

use crate::resolve::binding::cache::ResolveScopeIndexCache;

/// Cache for dependency resolution within a module.
#[derive(Debug, Default)]
pub(crate) struct ResolveDependencyItemCache {
    /// Cached remote symbol resolutions by target, key, and kind.
    pub(crate) remote_symbols: FxHashMap<RemoteSymbolCacheKey, (GlobalSymbolId, DependencyKind)>,
    /// Cached reexport chain lookups by target, space, and key.
    pub(crate) reexport_chain_symbols: FxHashMap<ReexportChainCacheKey, Option<GlobalSymbolId>>,
    /// Cached dependency item ids by module.
    dependency_item_ids: FxHashMap<ModuleId, Vec<LocalNodeId<DependencyItem>>>,
    /// Cached export tables for resolved modules.
    module_exports: FxHashMap<ModuleId, IndexMap<(SymbolSpace, StaticKey), Export>>,
    /// Cached export tables for module bindings.
    pub(crate) binding_exports: FxHashMap<BindingExportCacheKey, Option<ModuleBindingExports>>,
    /// Cached namespace symbol lookups.
    pub(crate) namespace_symbols: FxHashMap<NamespaceSymbolCacheKey, Option<GlobalSymbolId>>,
    /// Cached namespace exports grouped by scope.
    pub(crate) namespace_exports_by_scope:
        FxHashMap<ModuleId, FxHashMap<LocalScopeId, Vec<NamespaceExport>>>,
    /// Cached namespace export lists by target.
    pub(crate) namespace_exports: FxHashMap<TargetCacheKey, Vec<(ModuleId, NamespaceExport)>>,
    /// Cached namespace export resolutions by target.
    pub(crate) namespace_export_symbols:
        FxHashMap<NamespaceExportSymbolCacheKey, (GlobalSymbolId, SymbolSpace)>,
    /// Cached export assignment targets by target.
    pub(crate) export_assignment_targets: FxHashMap<TargetCacheKey, Option<ExportAssignmentTarget>>,
    /// Cached import redirect targets by scope and name.
    pub(crate) import_redirects_by_scope:
        FxHashMap<ModuleId, FxHashMap<LocalScopeId, FxHashMap<StringId, ModuleTarget>>>,
    /// Cached scope symbol indices for name lookups.
    scope_indices: ResolveScopeIndexCache,
}

impl ResolveDependencyItemCache {
    /// Return the scope index cache for name lookups.
    pub(crate) fn scope_indices(&mut self) -> &mut ResolveScopeIndexCache {
        &mut self.scope_indices
    }

    /// Return cached dependency item ids or compute them once.
    pub(crate) fn dependency_item_ids_for(
        &mut self,
        module_id: ModuleId,
        tree: &Tree,
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
    pub(crate) fn ensure_module_exports(
        &mut self,
        module_id: ModuleId,
        exported_symbols: &ExportedSymbolTable,
    ) {
        if self.module_exports.contains_key(&module_id) {
            return;
        }

        // cache the export table once per module
        self.module_exports
            .insert(module_id, exported_symbols.clone());
    }

    /// Return cached module exports for one immutable artifact, inserting when missing.
    pub(crate) fn module_exports_from_artifact(
        &mut self,
        module_id: ModuleId,
        dir: &DirPrepared,
    ) -> &mut IndexMap<(SymbolSpace, StaticKey), Export> {
        self.module_exports
            .entry(module_id)
            .or_insert_with(|| dir.exported_symbols.as_ref().clone())
    }
}

/// Target of an export assignment resolution.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ExportAssignmentTarget {
    /// Redirect to another module's exports.
    Module(ModuleTarget),
    /// Look in a namespace symbol's members.
    Namespace(GlobalSymbolId),
}

/// Cache key for remote symbol memoization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct RemoteSymbolCacheKey {
    /// The resolved module target.
    pub(crate) target: ModuleTarget,
    /// The dependency kind being resolved.
    pub(crate) kind: DependencyKind,
    /// The exported symbol key.
    pub(crate) key: StaticKey,
    /// The origin module when resolving module bindings.
    pub(crate) origin_module_id: Option<ModuleId>,
}

/// Cache key for reexport chain memoization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ReexportChainCacheKey {
    /// The resolved module target.
    pub(crate) target: ModuleTarget,
    /// The export space being searched.
    pub(crate) space: SymbolSpace,
    /// The exported symbol key.
    pub(crate) key: StaticKey,
    /// The origin module when resolving module bindings.
    pub(crate) origin_module_id: Option<ModuleId>,
}

/// Cache key for target based cache entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct TargetCacheKey {
    /// The resolved module target.
    pub(crate) target: ModuleTarget,
    /// The origin module when resolving module bindings.
    pub(crate) origin_module_id: Option<ModuleId>,
}

/// Cache key for namespace export resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct NamespaceExportSymbolCacheKey {
    /// The resolved module target.
    pub(crate) target: ModuleTarget,
    /// The origin module when resolving module bindings.
    pub(crate) origin_module_id: Option<ModuleId>,
    /// The dependency kind being resolved.
    pub(crate) kind: DependencyKind,
    /// The exported symbol key.
    pub(crate) key: StaticKey,
}

/// Cache key for module binding export table memoization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct BindingExportCacheKey {
    /// The module that owns the binding.
    pub(crate) module_id: ModuleId,
    /// The binding declaration node id.
    pub(crate) declaration: LocalNodeIdAny,
}

/// Cache key for namespace symbol memoization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct NamespaceSymbolCacheKey {
    /// The namespace symbol being queried.
    pub(crate) symbol: GlobalSymbolId,
    /// The dependency kind being resolved.
    pub(crate) kind: DependencyKind,
    /// The exported symbol key.
    pub(crate) key: StaticKey,
}
