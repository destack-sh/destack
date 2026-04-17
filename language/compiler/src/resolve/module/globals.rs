use destack_artifact::DirPrepared;
use destack_builtin::builtin_library;
use destack_core::StringId;
use destack_dir::{
    DependencyKind, Expression, GenericArgument, GlobalNodeIdAny, GlobalSymbolId, ImportSource,
    LocalNodeId, ModuleResolution, ModuleTarget, NodeTree, Path, StaticKey, SymbolKind,
    SymbolSpace, SymbolSpaceOrder, SymbolTable,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};
use indexmap::{IndexMap, IndexSet};
use std::collections::VecDeque;
use std::sync::Arc;

use crate::resolve::binding::cache::ResolveScopeIndexCache;
use crate::resolve::binding::{ResolveState, ResolvedPathSymbolTargets};
use crate::{Compiler, RequirementError, ResolveError, ResolveResult};

/// Key for grouping global symbols by name and space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct GlobalSymbolGroupKey {
    /// The symbol key.
    pub key: StaticKey,
    /// The symbol space.
    pub space: SymbolSpace,
}

/// Track global symbols from declare global blocks reachable from a root set.
#[derive(Debug, Clone)]
pub(crate) struct GlobalSymbolTable {
    /// First symbol observed for each global key.
    pub symbols: IndexMap<StaticKey, GlobalSymbolId>,
    /// First symbol observed for each global key and space.
    pub symbols_by_space: IndexMap<GlobalSymbolGroupKey, GlobalSymbolId>,
    /// All symbols observed for each global key.
    pub sources: IndexMap<StaticKey, Vec<GlobalSymbolId>>,
    /// All symbols observed for each global key and space.
    pub sources_by_space: IndexMap<GlobalSymbolGroupKey, Vec<GlobalSymbolId>>,
    /// Modules remaining to process.
    pub pending: VecDeque<ModuleId>,
}

/// Cache key for one global symbol table snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct GlobalSymbolTableCacheKey {
    /// The profile that selected the global roots.
    pub profile_id: ProfileId,
    /// The selected root modules for the table.
    pub roots: Arc<[ModuleId]>,
}

impl Default for GlobalSymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalSymbolTable {
    /// Create an empty table.
    fn new() -> Self {
        Self {
            symbols: IndexMap::new(),
            symbols_by_space: IndexMap::new(),
            sources: IndexMap::new(),
            sources_by_space: IndexMap::new(),
            pending: VecDeque::new(),
        }
    }

    /// Insert a global symbol and preserve the first binding for the key.
    fn insert_symbol(&mut self, key: StaticKey, space: SymbolSpace, symbol: GlobalSymbolId) {
        self.sources.entry(key).or_default().push(symbol);
        self.symbols.entry(key).or_insert(symbol);

        let group_key = GlobalSymbolGroupKey { key, space };
        self.sources_by_space
            .entry(group_key)
            .or_default()
            .push(symbol);
        self.symbols_by_space.entry(group_key).or_insert(symbol);
    }
}

/// Track dependency targets while scanning module trees.
#[derive(Debug, Clone, Copy)]
struct DependencyTarget {
    /// The dependency source syntax.
    source: ImportSource,
    /// The module specifier.
    target: StringId,
    /// The node that referenced the module.
    node: GlobalNodeIdAny,
    /// The dependency kind (type vs value).
    kind: DependencyKind,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Build the global symbol table for a module and profile.
    pub(crate) fn global_symbol_table_for_module(
        &self,
        revision: destack_workspace::Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ResolveResult<Arc<GlobalSymbolTable>> {
        let roots = self.select_global_symbol_table(revision, module_id, profile_id)?;
        let cache_key = GlobalSymbolTableCacheKey {
            profile_id,
            roots: Arc::clone(&roots),
        };

        if let Some(cached) = self.current_global_symbol_table(revision, &cache_key) {
            return Ok(cached);
        }

        let Some(module_graph) = self.module_graph(profile_id) else {
            let cache =
                Arc::new(self.build_global_symbol_table(revision, roots.as_ref(), profile_id)?);
            self.store_current_global_symbol_table(revision, cache_key, Arc::clone(&cache));

            return Ok(cache);
        };

        if let Some(cached) = self.index.global_symbol_tables.get(&cache_key) {
            let (cached_graph, cached_table) = cached.value();
            if Arc::ptr_eq(cached_graph, &module_graph)
                || cached_graph.matches_snapshot(module_graph.as_ref())
            {
                let cached_table = Arc::clone(cached_table);
                self.store_current_global_symbol_table(
                    revision,
                    cache_key,
                    Arc::clone(&cached_table),
                );

                return Ok(cached_table);
            }
        }

        let cache =
            Arc::new(self.build_global_symbol_table(revision, roots.as_ref(), profile_id)?);
        self.index
            .global_symbol_tables
            .insert(cache_key.clone(), (module_graph, cache.clone()));
        self.store_current_global_symbol_table(revision, cache_key, Arc::clone(&cache));

        Ok(cache)
    }

    /// Resolve a path against the global symbol table.
    pub(crate) fn resolve_global_path(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        node: GlobalNodeIdAny,
        profile_id: ProfileId,
        path: &Path,
        generic_arguments: Option<Vec<LocalNodeId<GenericArgument>>>,
        space_order: SymbolSpaceOrder,
        mut scope_cache: Option<&mut ResolveScopeIndexCache>,
        tree: &mut NodeTree,
    ) -> ResolveResult<Option<(Expression, ResolvedPathSymbolTargets)>> {
        // load the cached table for this module
        let cache = self.global_symbol_table_for_module(revision, module.id, profile_id)?;

        // resolve the first path segment against global symbols
        let first_segment = path.first_segment().expect("path is empty");
        let symbol_key = StaticKey::Name(first_segment);
        let preferred_spaces = space_order.spaces();
        let mut target_symbol = None;
        for space in preferred_spaces {
            let group_key = GlobalSymbolGroupKey {
                key: symbol_key,
                space: *space,
            };
            if let Some(symbol) = cache.symbols_by_space.get(&group_key).copied() {
                target_symbol = Some(symbol);
                break;
            }
            if *space == SymbolSpace::Type || *space == SymbolSpace::Value {
                let type_value_key = GlobalSymbolGroupKey {
                    key: symbol_key,
                    space: SymbolSpace::TypeValue,
                };
                if let Some(symbol) = cache.symbols_by_space.get(&type_value_key).copied() {
                    target_symbol = Some(symbol);
                    break;
                }
            }
        }
        let target_symbol = target_symbol.or_else(|| cache.symbols.get(&symbol_key).copied());

        // fall back to selected lib symbol cache when global cache misses
        let target_symbol = target_symbol
            .or_else(|| self.get_library_symbol_from(profile_id, first_segment, space_order));
        let target_symbol = match target_symbol {
            Some(target_symbol) => Some(target_symbol),
            None => self.resolve_selected_lib_symbol(
                revision,
                module,
                profile_id,
                node,
                symbol_key,
                space_order,
                scope_cache.as_deref_mut(),
            )?,
        };

        let Some(target_symbol) = target_symbol else {
            return Ok(None);
        };
        let mut receiver_targets = ResolvedPathSymbolTargets::new();
        receiver_targets.push(target_symbol);

        // ensure the target module is prepared before reading its symbols
        self.require_dir_prepared_if_other(
            revision,
            module.id,
            target_symbol.module_id,
            profile_id,
        )
        .map_err(|error| match error {
            RequirementError::NotReady { requirement } => ResolveError::Yield { requirement },
            RequirementError::Failed { requirement } => {
                ResolveError::UnsatisfiedRequirement { requirement }
            }
        })?;

        // return the global reference when the path is a single segment
        if path.segments.len() == 1 {
            return Ok(Some((
                Expression::GlobalReference {
                    path: path.clone(),
                    generic_arguments: generic_arguments.unwrap_or_default(),
                    target_symbol,
                },
                receiver_targets,
            )));
        }

        // load the target module symbols for namespace resolution
        let target_context = self
            .cache_module_snapshot(revision, target_symbol.module_id)
            .map_err(|error| ResolveError::Internal {
                message: format!("failed to load module snapshot: {error}"),
            })?;
        let target_dir = self
            .require_artifact_dir_prepared(revision, target_symbol.module_id, profile_id)
            .map_err(ResolveError::from)?;
        let symbols = &target_dir.symbols;
        let local_symbol_id = target_symbol.local_id;
        let symbol = symbols.get_symbol(local_symbol_id);

        // resolve namespace members when the root is a namespace
        if symbol.kind == SymbolKind::Namespace {
            let remaining_path = path.slice(1..);
            let pass = ResolveState::artifact(
                revision,
                &target_context,
                profile_id,
                node,
                space_order,
                symbols,
                target_dir.namespace_symbol,
                target_dir.namespace_scope,
                target_dir.global_augmentation_scope,
                &target_dir.exported_symbols,
                Some(&target_dir.tree),
            );

            match self.resolve_relative_symbol_with_ambient_merge(
                pass,
                local_symbol_id,
                &remaining_path,
                scope_cache,
            ) {
                // resolve namespace members directly when the local scope has the full path
                Ok((resolved_id, None, resolved_targets)) => {
                    return Ok(Some((
                        Expression::GlobalReference {
                            path: path.clone(),
                            generic_arguments: generic_arguments.unwrap_or_default(),
                            target_symbol: resolved_id,
                        },
                        resolved_targets,
                    )));
                }
                // build a member chain when only a path prefix resolved
                Ok((resolved_id, Some(remaining), resolved_targets)) => {
                    // if local namespace lookup consumed no member segments, retry via module exports
                    if remaining.segments.len() == remaining_path.segments.len()
                        && let Some((export_symbol, export_remaining)) = self
                            .resolve_global_namespace_member_via_exports(
                                revision,
                                &target_context,
                                profile_id,
                                &remaining_path,
                                space_order,
                            )?
                    {
                        let resolved_path =
                            path.slice(0..path.segments.len() - export_remaining.segments.len());
                        if export_remaining.segments.is_empty() {
                            return Ok(Some((
                                Expression::GlobalReference {
                                    path: resolved_path,
                                    generic_arguments: generic_arguments
                                        .clone()
                                        .unwrap_or_default(),
                                    target_symbol: export_symbol,
                                },
                                receiver_targets.clone(),
                            )));
                        }

                        let root_expr = Expression::GlobalReference {
                            path: resolved_path,
                            generic_arguments: vec![],
                            target_symbol: export_symbol,
                        };
                        return Ok(Some((
                            self.build_member_chain(
                                expression_id,
                                root_expr,
                                &export_remaining,
                                generic_arguments.clone(),
                                tree,
                            ),
                            receiver_targets.clone(),
                        )));
                    }
                    // build regular global reference member chain
                    else {
                        let resolved_path =
                            path.slice(0..path.segments.len() - remaining.segments.len());
                        let root_expr = Expression::GlobalReference {
                            path: resolved_path,
                            generic_arguments: vec![],
                            target_symbol: resolved_id,
                        };
                        return Ok(Some((
                            self.build_member_chain(
                                expression_id,
                                root_expr,
                                &remaining,
                                generic_arguments.clone(),
                                tree,
                            ),
                            resolved_targets,
                        )));
                    }
                }
                // fall back to module exports for export-as-namespace alias members
                Err(error @ ResolveError::MissingSymbol { .. }) => {
                    if let Some((resolved_id, remaining)) = self
                        .resolve_global_namespace_member_via_exports(
                            revision,
                            &target_context,
                            profile_id,
                            &remaining_path,
                            space_order,
                        )?
                    {
                        let resolved_path =
                            path.slice(0..path.segments.len() - remaining.segments.len());
                        if remaining.segments.is_empty() {
                            return Ok(Some((
                                Expression::GlobalReference {
                                    path: resolved_path,
                                    generic_arguments: generic_arguments
                                        .clone()
                                        .unwrap_or_default(),
                                    target_symbol: resolved_id,
                                },
                                receiver_targets.clone(),
                            )));
                        }

                        let root_expr = Expression::GlobalReference {
                            path: resolved_path,
                            generic_arguments: vec![],
                            target_symbol: resolved_id,
                        };
                        return Ok(Some((
                            self.build_member_chain(
                                expression_id,
                                root_expr,
                                &remaining,
                                generic_arguments.clone(),
                                tree,
                            ),
                            receiver_targets.clone(),
                        )));
                    }

                    return Err(error);
                }
                Err(error) => return Err(error),
            }
        }

        // build a member chain for the remaining segments
        let root_path = Path {
            segments: vec![first_segment].into(),
        };
        let root_expr = Expression::GlobalReference {
            path: root_path,
            generic_arguments: vec![],
            target_symbol,
        };
        Ok(Some((
            self.build_member_chain(
                expression_id,
                root_expr,
                &path.slice(1..),
                generic_arguments,
                tree,
            ),
            receiver_targets,
        )))
    }

    /// Resolve a global namespace member through module exports.
    fn resolve_global_namespace_member_via_exports(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile_id: ProfileId,
        path: &Path,
        space_order: SymbolSpaceOrder,
    ) -> ResolveResult<Option<(GlobalSymbolId, Path)>> {
        // only declaration modules support export-as-namespace fallback
        if !module.language_type.is_declaration() {
            return Ok(None);
        }

        // no member path means there is nothing to resolve through exports
        let Some(first_segment) = path.first_segment() else {
            return Ok(None);
        };

        // look up the first segment through the full export resolution chain
        let key = StaticKey::Name(first_segment);
        let dir_base = self
            .artifact_dir_base(module.id)
            .unwrap_or_else(|| panic!("missing committed base dir artifact for {:?}", module.id));
        let anchor = dir_base.anchor_node.into_global(module.id);
        let Some(resolved_symbol) = self.resolve_export_symbol_for_target(
            revision,
            module.id,
            anchor,
            ModuleTarget::Module(module.id),
            profile_id,
            space_order,
            key,
        )?
        else {
            return Ok(None);
        };

        // keep the unresolved tail for member-chain building
        let remaining = path.slice(1..);
        Ok(Some((resolved_symbol, remaining)))
    }

    /// Build a global symbol cache for freestanding modules.
    /// This collects all global symbols from the given modules in one go.
    pub(crate) fn build_global_symbol_table_freestanding(
        &self,
        revision: destack_workspace::Revision,
        modules: &[ModuleId],
        profile_id: ProfileId,
    ) -> ResolveResult<GlobalSymbolTable> {
        let mut cache = GlobalSymbolTable::new();
        let mut processed_modules = IndexSet::new();
        cache.pending.extend(modules.iter().copied());

        // process all lib modules
        while let Some(module_id) = cache.pending.pop_front() {
            // skip already processed modules
            if !processed_modules.insert(module_id) {
                continue;
            }

            // load the module tree and symbols
            self.require_dir_base(revision, module_id)?;
            self.require_dir_prepared(revision, module_id, profile_id)?;
            let module = self
                .cache_module_snapshot(revision, module_id)
                .map_err(|error| ResolveError::Internal {
                    message: format!("failed to load module snapshot: {error}"),
                })?;
            let dir = self
                .require_artifact_dir_prepared(revision, module_id, profile_id)
                .map_err(ResolveError::from)?;
            let tree = &dir.tree;
            let symbols = &dir.symbols;

            // collect global declarations from this module
            self.collect_global_augmentation_symbols(&module, &dir, symbols, &mut cache);
            if module.language_type.is_declaration() {
                self.collect_export_namespace_globals(&module, tree, symbols, &mut cache);
            }
            if self.module_exposes_namespace_scope_globals(&module) {
                self.collect_namespace_scope_globals(&module, &dir, symbols, &mut cache);
            }
        }

        Ok(cache)
    }

    /// Build the global symbol table for a root module set.
    fn build_global_symbol_table(
        &self,
        revision: destack_workspace::Revision,
        roots: &[ModuleId],
        profile_id: ProfileId,
    ) -> ResolveResult<GlobalSymbolTable> {
        let mut cache = GlobalSymbolTable::new();
        let mut processed_modules = IndexSet::new();
        cache.pending.extend(roots.iter().copied());

        // walk the module graph
        while let Some(module_id) = cache.pending.pop_front() {
            // skip already processed modules
            if !processed_modules.insert(module_id) {
                continue;
            }

            // ensure bind validation before reading dir data
            if let Err(error) = self.require_dir_base(revision, module_id) {
                return Err(error.into());
            }

            // ensure per profile module data is prepared before reading dir data
            if let Err(error) = self.require_dir_prepared(revision, module_id, profile_id) {
                return Err(error.into());
            }

            // load the module tree and symbols
            let module = self
                .cache_module_snapshot(revision, module_id)
                .map_err(|error| ResolveError::Internal {
                    message: format!("failed to load module snapshot: {error}"),
                })?;
            let dir = self
                .require_artifact_dir_prepared(revision, module_id, profile_id)
                .map_err(ResolveError::from)?;
            let tree = &dir.tree;
            let symbols = &dir.symbols;

            // collect global declarations from this module
            self.collect_global_augmentation_symbols(&module, &dir, symbols, &mut cache);
            if module.language_type.is_declaration() {
                self.collect_export_namespace_globals(&module, tree, symbols, &mut cache);
            }
            if self.module_exposes_namespace_scope_globals(&module) {
                self.collect_namespace_scope_globals(&module, &dir, symbols, &mut cache);
            }

            // enqueue dependency targets for further discovery
            let dependency_targets = self.collect_dependency_targets(module_id, tree, symbols);
            for dependency in dependency_targets {
                // skip module bindings before resolving file targets
                if self
                    .resolve_module_binding_target(
                        revision,
                        module_id,
                        profile_id,
                        dependency.target,
                    )?
                    .is_some()
                {
                    continue;
                }

                // resolve specifiers to modules for traversal
                let resolved_targets = match self.resolve_dependency_targets_for_global_traversal(
                    revision, module_id, profile_id, dependency,
                ) {
                    Ok(targets) => targets,
                    Err(_) if module.is_builtin() && dependency.kind == DependencyKind::Type => {
                        continue;
                    }
                    Err(error) => {
                        self.handle_unresolved_module(error);
                        continue;
                    }
                };

                // enqueue primary and companion module targets for global symbol traversal
                for remote_module_id in
                    self.resolved_dependency_module_ids(resolved_targets, dependency.kind)
                {
                    cache.pending.push_back(remote_module_id);
                }
            }
        }

        Ok(cache)
    }

    /// Return true when a module's namespace scope contributes global symbols.
    fn module_exposes_namespace_scope_globals(&self, module: &Module) -> bool {
        // ambient libs always contribute top-level global declarations
        let builtins = self.repository.builtins.as_ref();
        if let Some(lib_name) = builtins.library_name_for_module(module.id)
            && let Some(lib) = builtin_library(lib_name)
            && lib.is_ambient
        {
            return true;
        }

        // declaration scripts also contribute top-level global declarations
        module.language_type.is_declaration() && module.source_type.is_script()
    }

    /// Resolve one dependency target for global symbol table traversal.
    fn resolve_dependency_targets_for_global_traversal(
        &self,
        revision: destack_workspace::Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
        dependency: DependencyTarget,
    ) -> ResolveResult<ModuleResolution> {
        // resolve triple slash reference lib directives through builtin library loading
        if dependency.source == ImportSource::ReferenceLibDirective {
            let target_text = self.repository.strings.get(dependency.target);
            let module_id = self
                .resolve_reference_lib_to_module(revision, profile_id, target_text.as_ref())
                .map_err(|_| ResolveError::UnresolvedModule {
                    node: dependency.node.into_anchored(Some(profile_id)),
                    target: dependency.target,
                })?;

            return Ok(ModuleResolution::from_target(ModuleTarget::Module(
                module_id,
            )));
        }

        // normalize triple slash reference path directives before specifier resolution
        let target =
            self.resolve_target_for_dependency_source(dependency.source, dependency.target);
        let source_module = self
            .cache_module_snapshot(revision, module_id)
            .map_err(|error| ResolveError::Internal {
                message: format!("failed to load module snapshot: {error}"),
            })?;
        let source_module = source_module.as_ref();
        let target = self.canonical_import_specifier(
            revision,
            source_module,
            profile_id,
            dependency.node,
            target,
        )?;

        // match direct resolve import edge semantics
        let is_typescript_commonjs = source_module.module_format.is_commonjs()
            && source_module.language_type.is_typescript();
        let edge_relation =
            Self::import_edge_kind_for_dependency(dependency.source, is_typescript_commonjs);

        if let Ok(targets) = self.resolve_specifier_to_module_resolution(
            revision,
            profile_id,
            target,
            Some(module_id),
            edge_relation,
            None,
        ) {
            return Ok(targets);
        }

        if let Some(binding_target) =
            self.resolve_module_binding_target(revision, module_id, profile_id, target)?
        {
            return Ok(ModuleResolution::from_target(binding_target));
        }

        if let Some(external_target) =
            self.externalized_package_import_target(revision, source_module, profile_id, target)?
        {
            return Ok(ModuleResolution::from_target(external_target));
        }

        Err(self.unresolved_error_for_specifier(
            dependency.node,
            profile_id,
            dependency.target,
            target,
        ))
    }

    /// Collect module ids from primary and companion resolution targets.
    fn resolved_dependency_module_ids(
        &self,
        targets: ModuleResolution,
        kind: DependencyKind,
    ) -> Vec<ModuleId> {
        let mut module_ids = Vec::new();

        // include the target for the requested dependency kind
        if let Some(module_id) = targets.for_kind(kind).and_then(|target| target.module_id()) {
            module_ids.push(module_id);
        }

        // include the companion target to cover mixed value and type declaration globals
        let companion = match kind {
            DependencyKind::Value => targets.ty,
            DependencyKind::Type => targets.value,
        };
        if let Some(module_id) = companion.and_then(|target| target.module_id())
            && !module_ids.contains(&module_id)
        {
            module_ids.push(module_id);
        }

        module_ids
    }

    /// Collect global symbols from the global augmentation scope.
    /// Symbols inside `declare global { }` blocks are bound into this scope by the binder.
    fn collect_global_augmentation_symbols(
        &self,
        module: &Module,
        dir: &DirPrepared,
        symbols: &SymbolTable,
        cache: &mut GlobalSymbolTable,
    ) {
        let scope = symbols.get_scope_by_id(dir.global_augmentation_scope);
        for (key, symbol_id) in symbols.active_named_symbols(scope) {
            let symbol = symbols.get_symbol(symbol_id);
            cache.insert_symbol(key, symbol.space, symbol_id.into_global(module.id));
        }
    }

    /// Collect global symbols from `export as namespace` declarations.
    /// Only call this for declaration modules (`.d.ts`).
    fn collect_export_namespace_globals(
        &self,
        module: &Module,
        tree: &NodeTree,
        symbols: &SymbolTable,
        cache: &mut GlobalSymbolTable,
    ) {
        let Some(dir_base) = self.dir_base(module.id) else {
            return;
        };
        let symbol_id = dir_base.namespace_symbol.into_global(module.id);
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            if !self.is_node_active(tree, symbols, expression_id.into_any()) {
                continue;
            }
            let Expression::ExportNamespace { name } = tree.get(expression_id) else {
                continue;
            };
            cache.insert_symbol(StaticKey::Name(*name), SymbolSpace::Value, symbol_id);
            cache.insert_symbol(StaticKey::Name(*name), SymbolSpace::Type, symbol_id);
        }
    }

    /// Collect top-level declarations from a module's namespace scope as globals.
    /// Intended only for ambient/global script modules where top-level symbols are globals.
    fn collect_namespace_scope_globals(
        &self,
        module: &Module,
        dir: &DirPrepared,
        symbols: &SymbolTable,
        cache: &mut GlobalSymbolTable,
    ) {
        let scope = symbols.get_scope_by_id(dir.namespace_scope);
        for (key, symbol_id) in symbols.active_named_symbols(scope) {
            let symbol = symbols.get_symbol(symbol_id);
            cache.insert_symbol(key, symbol.space, symbol_id.into_global(module.id));
        }
    }

    /// Collect module specifiers referenced by imports and reexports.
    fn collect_dependency_targets(
        &self,
        module_id: ModuleId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Vec<DependencyTarget> {
        // collect import and reexport targets
        let mut targets = Vec::new();
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            if !self.is_node_active(tree, symbols, expression_id.into_any()) {
                continue;
            }
            match tree.get(expression_id) {
                Expression::UnresolvedImport {
                    source,
                    target,
                    kind,
                    ..
                } => {
                    let target = match target {
                        destack_dir::ImportTarget::String(target) => *target,
                        destack_dir::ImportTarget::Expression { .. } => continue,
                    };
                    targets.push(DependencyTarget {
                        source: *source,
                        target,
                        node: expression_id.into_global_any(module_id),
                        kind: *kind,
                    });
                }
                Expression::UnresolvedReExport { target, kind, .. }
                | Expression::ReExport { target, kind, .. } => {
                    targets.push(DependencyTarget {
                        source: ImportSource::ExportStatement,
                        target: *target,
                        node: expression_id.into_global_any(module_id),
                        kind: *kind,
                    });
                }
                Expression::Import {
                    source,
                    target,
                    kind,
                    ..
                } => {
                    targets.push(DependencyTarget {
                        source: *source,
                        target: *target,
                        node: expression_id.into_global_any(module_id),
                        kind: *kind,
                    });
                }
                _ => {}
            }
        }
        targets
    }
}
