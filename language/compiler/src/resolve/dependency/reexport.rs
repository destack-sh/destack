use destack_artifact::DirPrepared;
use destack_ast::StringId;
use destack_dir::{
    Declaration, DependencyItem, DependencyKind, DependencyMode, Export, ExportKind,
    GlobalNodeIdAny, GlobalSymbolId, LocalNodeId, LocalScopeId, LocalScopeMark, ModuleTarget,
    NodeTree, StaticKey, SymbolSpace, SymbolSpaceOrder,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId, Revision};
use indexmap::IndexMap;

use crate::resolve::binding::ResolveState;
use crate::resolve::dependency::cache::{
    BindingExportCacheKey, ExportAssignmentTarget, ReexportChainCacheKey, RemoteSymbolCacheKey,
    ResolveDependencyItemCache,
};
use crate::resolve::dependency::dependency::{ReexportVisitStack, ResolvedExportSymbol};
use crate::{Compiler, ResolveError, ResolveResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve a symbol from a module export table.
    pub(crate) fn resolve_exported_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        exports: &IndexMap<(SymbolSpace, StaticKey), Export>,
        tree: &NodeTree,
        order: SymbolSpaceOrder,
        key: StaticKey,
    ) -> Option<GlobalSymbolId> {
        // walk export spaces in priority order
        for space in order.spaces() {
            let export = exports.get(&(*space, key)).or_else(|| {
                if *space == SymbolSpace::Type || *space == SymbolSpace::Value {
                    return exports.get(&(SymbolSpace::TypeValue, key));
                }

                None
            });
            let Some(export) = export else {
                continue;
            };

            let symbol = export.target.resolved().or_else(|| match export.kind {
                ExportKind::Local => export.symbol.map(|symbol| symbol.into_global(module.id)),
                ExportKind::ReExport => export.item.and_then(|item| tree.get(item).target_symbol()),
            });

            // return the first matching symbol
            if let Some(symbol) = symbol {
                let symbol_type = self.symbol_type_for_global(profile, symbol);
                return Some(GlobalSymbolId::new(
                    symbol.module_id,
                    symbol.local_id.with_type(symbol_type),
                ));
            }
        }

        None
    }

    /// Resolve an export entry by walking reexport chains.
    pub(super) fn resolve_reexport_chain_symbol(
        &self,
        revision: Revision,
        origin_module_id: ModuleId,
        origin_symbol: Option<GlobalSymbolId>,
        node: GlobalNodeIdAny,
        target: ModuleTarget,
        profile: ProfileId,
        kind: DependencyKind,
        key: StaticKey,
        default_name: StringId,
        visited: &mut ReexportVisitStack,
        mut cache: Option<&mut ResolveDependencyItemCache>,
    ) -> ResolveResult<Option<ResolvedExportSymbol>> {
        // check symbol spaces in priority order
        let order = self.export_spaces_for_kind(kind);
        for space in order.spaces() {
            let symbol = self.resolve_reexport_chain_symbol_for_space(
                revision,
                origin_module_id,
                origin_symbol,
                node,
                target,
                profile,
                *space,
                key,
                default_name,
                visited,
                cache.as_deref_mut(),
            )?;
            if let Some(symbol) = symbol {
                return Ok(Some(ResolvedExportSymbol {
                    symbol,
                    export_space: *space,
                }));
            }
        }

        Ok(None)
    }

    /// Resolve an export entry for a target with an explicit symbol space order.
    pub(crate) fn resolve_export_symbol_for_target(
        &self,
        revision: Revision,
        origin_module_id: ModuleId,
        node: GlobalNodeIdAny,
        target: ModuleTarget,
        profile: ProfileId,
        order: SymbolSpaceOrder,
        key: StaticKey,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        // resolve in the requested symbol space order
        let default_name = self.repository.strings.intern("default");
        let mut visited = ReexportVisitStack::default();
        for space in order.spaces() {
            let symbol = self.resolve_reexport_chain_symbol_for_space(
                revision,
                origin_module_id,
                None,
                node,
                target,
                profile,
                *space,
                key,
                default_name,
                &mut visited,
                None,
            )?;
            if let Some(symbol) = symbol {
                return Ok(Some(symbol));
            }
        }

        Ok(None)
    }

    /// Resolve an export entry for a specific symbol space.
    fn resolve_reexport_chain_symbol_for_space(
        &self,
        revision: destack_workspace::Revision,
        origin_module_id: ModuleId,
        origin_symbol: Option<GlobalSymbolId>,
        node: GlobalNodeIdAny,
        target: ModuleTarget,
        profile: ProfileId,
        space: SymbolSpace,
        key: StaticKey,
        default_name: StringId,
        visited: &mut ReexportVisitStack,
        mut cache: Option<&mut ResolveDependencyItemCache>,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        let cache_key = cache.as_ref().map(|_| ReexportChainCacheKey {
            target,
            space,
            key,
            origin_module_id: self.cache_origin_module_id(origin_module_id, target),
        });
        if let (Some(cache), Some(cache_key)) = (cache.as_deref(), cache_key)
            && let Some(&cached) = cache.reexport_chain_symbols.get(&cache_key)
        {
            return Ok(cached);
        }

        // detect cycles in reexport chains
        let visit_key = (target, key, space);
        if visited.contains(&visit_key) {
            // report cyclic symbols when an origin symbol is known
            if let Some(symbol) = origin_symbol {
                return Err(ResolveError::CyclicSymbol {
                    node: node.into_anchored(Some(profile)),
                    symbol,
                });
            }
            // report missing symbols when a chain loops without an origin symbol
            let (scope, via_module) =
                self.export_chain_scope_for_target(revision, origin_module_id, target, profile)?;
            return Err(ResolveError::MissingSymbol {
                node: node.into_anchored(Some(profile)),
                scope,
                via_module,
                key,
            });
        }

        // record this visit for cycle detection
        visited.push(visit_key);

        // resolve the export entry for this space and key
        let result = self.resolve_export_entry_for_target(
            revision,
            origin_module_id,
            origin_symbol,
            node,
            target,
            profile,
            space,
            key,
            default_name,
            visited,
            cache.as_deref_mut(),
        );

        // drop the visit marker
        visited.pop();

        if let (Some(cache), Some(cache_key)) = (cache, cache_key) {
            if let Ok(resolved) = result {
                cache.reexport_chain_symbols.insert(cache_key, resolved);
                return Ok(resolved);
            }
            return result;
        }

        result
    }

    /// Get a scope for errors while resolving export chains.
    pub(super) fn export_chain_scope_for_target(
        &self,
        revision: Revision,
        origin_module_id: ModuleId,
        target: ModuleTarget,
        profile: ProfileId,
    ) -> ResolveResult<(destack_dir::GlobalScopeId, Option<ModuleId>)> {
        match target {
            ModuleTarget::Module(module_id) => {
                // non-code modules don't have scopes - use origin module scope
                let module = self
                    .cache_module_snapshot(revision, module_id)
                    .map_err(|error| ResolveError::Internal {
                        message: format!("failed to load module snapshot: {error}"),
                    })?;
                if !module.is_code() {
                    let dir = self
                        .require_artifact_dir_prepared(revision, origin_module_id, profile)
                        .map_err(ResolveError::from)?;
                    return Ok((dir.namespace_scope.into_global(origin_module_id), None));
                }

                // ensure the target module is prepared
                self.require_dir_prepared_if_other(revision, origin_module_id, module_id, profile)?;

                // use the module namespace scope for error reporting
                let dir = self
                    .require_artifact_dir_prepared(revision, module_id, profile)
                    .map_err(ResolveError::from)?;
                Ok((dir.namespace_scope.into_global(module_id), Some(module_id)))
            }
            ModuleTarget::External(_) => {
                let dir = self
                    .require_artifact_dir_prepared(revision, origin_module_id, profile)
                    .map_err(ResolveError::from)?;
                Ok((
                    dir.namespace_scope.into_global(origin_module_id),
                    Some(origin_module_id),
                ))
            }
            ModuleTarget::Binding(specifier) => {
                // load the module bindings for this specifier
                let bindings = self.module_bindings_for_specifier(
                    revision,
                    origin_module_id,
                    profile,
                    specifier,
                )?;

                // fall back to the origin scope when bindings are missing
                let Some(bindings) = bindings else {
                    let dir = self
                        .require_artifact_dir_prepared(revision, origin_module_id, profile)
                        .map_err(ResolveError::from)?;
                    return Ok((
                        dir.namespace_scope.into_global(origin_module_id),
                        Some(origin_module_id),
                    ));
                };

                // fall back to the origin scope when the list is empty
                let Some(binding_ref) = bindings.first() else {
                    let dir = self
                        .require_artifact_dir_prepared(revision, origin_module_id, profile)
                        .map_err(ResolveError::from)?;
                    return Ok((
                        dir.namespace_scope.into_global(origin_module_id),
                        Some(origin_module_id),
                    ));
                };

                // ensure the binding module is prepared
                self.require_dir_prepared_if_other(
                    revision,
                    origin_module_id,
                    binding_ref.module_id,
                    profile,
                )?;

                // load the binding scope from the owning module
                let dir = self
                    .require_artifact_dir_prepared(revision, binding_ref.module_id, profile)
                    .map_err(ResolveError::from)?;
                let Some((scope_id, _, _)) =
                    self.binding_info_for_declaration(&dir, binding_ref.declaration)
                else {
                    return Ok((
                        dir.namespace_scope.into_global(binding_ref.module_id),
                        Some(binding_ref.module_id),
                    ));
                };

                Ok((
                    scope_id.into_global(binding_ref.module_id),
                    Some(binding_ref.module_id),
                ))
            }
        }
    }

    /// Resolve an export entry for a target to a concrete symbol.
    fn resolve_export_entry_for_target(
        &self,
        revision: destack_workspace::Revision,
        origin_module_id: ModuleId,
        origin_symbol: Option<GlobalSymbolId>,
        node: GlobalNodeIdAny,
        target: ModuleTarget,
        profile: ProfileId,
        space: SymbolSpace,
        key: StaticKey,
        default_name: StringId,
        visited: &mut ReexportVisitStack,
        mut cache: Option<&mut ResolveDependencyItemCache>,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        match target {
            ModuleTarget::Module(module_id) => {
                // ensure the target module is prepared
                self.require_dir_prepared_if_other(revision, origin_module_id, module_id, profile)?;

                // load the module dir for this profile
                let module_context =
                    self.cache_module_snapshot(revision, module_id)
                        .map_err(|error| ResolveError::Internal {
                            message: format!("failed to load module snapshot: {error}"),
                        })?;
                let dir = self
                    .require_artifact_dir_prepared(revision, module_id, profile)
                    .map_err(ResolveError::from)?;

                // read the export entry for the requested key
                let export = if let Some(cache) = cache.as_deref_mut() {
                    cache
                        .module_exports_from_artifact(module_id, &dir)
                        .get(&(space, key))
                        .cloned()
                } else {
                    dir.exported_symbols.get(&(space, key)).cloned()
                };
                let Some(export) = export else {
                    return Ok(None);
                };

                // for data/text/binary modules, return the default symbol directly
                // (no need to resolve through export entry since it's a simple local export)
                if !module_context.is_code() {
                    if let Some(symbol) = export.symbol {
                        let symbol = symbol.into_global(module_id);
                        let symbol_type = self.symbol_type_for_global(profile, symbol);
                        let symbol =
                            GlobalSymbolId::new(module_id, symbol.local_id.with_type(symbol_type));
                        return Ok(Some(symbol));
                    }
                    return Ok(None);
                }

                // resolve the export entry for this module
                self.resolve_export_entry_symbol(
                    revision,
                    &module_context,
                    &dir,
                    dir.namespace_scope,
                    origin_module_id,
                    origin_symbol,
                    node,
                    profile,
                    default_name,
                    export,
                    visited,
                    cache.as_deref_mut(),
                )
            }
            ModuleTarget::External(_) => Ok(None),
            ModuleTarget::Binding(specifier) => {
                // load bindings for the specifier
                let bindings = self.module_bindings_for_specifier(
                    revision,
                    origin_module_id,
                    profile,
                    specifier,
                )?;
                let Some(bindings) = bindings else {
                    return Ok(None);
                };

                // track the first resolved symbol for conflict checks
                let mut resolved: Option<(GlobalSymbolId, Option<GlobalNodeIdAny>)> = None;
                for binding_ref in bindings {
                    // ensure the binding module is prepared
                    self.require_dir_prepared_if_other(
                        revision,
                        origin_module_id,
                        binding_ref.module_id,
                        profile,
                    )?;

                    // load the binding export table
                    let module_context = self
                        .cache_module_snapshot(revision, binding_ref.module_id)
                        .map_err(|error| ResolveError::Internal {
                            message: format!("failed to load module snapshot: {error}"),
                        })?;
                    let dir = self
                        .require_artifact_dir_prepared(revision, binding_ref.module_id, profile)
                        .map_err(ResolveError::from)?;
                    let binding_exports = if let Some(cache) = cache.as_deref_mut() {
                        let cache_key = BindingExportCacheKey {
                            module_id: binding_ref.module_id,
                            declaration: binding_ref.declaration.into_any(),
                        };
                        cache
                            .binding_exports
                            .entry(cache_key)
                            .or_insert_with(|| {
                                dir.module_binding_exports
                                    .get(&binding_ref.declaration.into_any())
                                    .cloned()
                            })
                            .clone()
                    } else {
                        dir.module_binding_exports
                            .get(&binding_ref.declaration.into_any())
                            .cloned()
                    };
                    let Some(binding_exports) = binding_exports else {
                        continue;
                    };

                    // load the binding scope and export entry
                    let Some((scope_id, _, _)) =
                        self.binding_info_for_declaration(&dir, binding_ref.declaration)
                    else {
                        continue;
                    };
                    let Some(export) = binding_exports.exports.get(&(space, key)).cloned() else {
                        continue;
                    };

                    // resolve the export entry for this binding
                    let symbol = self.resolve_export_entry_symbol(
                        revision,
                        &module_context,
                        &dir,
                        scope_id,
                        origin_module_id,
                        origin_symbol,
                        node,
                        profile,
                        default_name,
                        export.clone(),
                        visited,
                        cache.as_deref_mut(),
                    )?;
                    let Some(symbol) = symbol else {
                        continue;
                    };

                    // report conflicts when bindings disagree
                    if let Some((existing, other_node)) = resolved {
                        if existing != symbol {
                            let symbols = &dir.symbols;
                            let node =
                                self.export_entry_node(binding_ref.module_id, &export, symbols);
                            if let (Some(node), Some(other_node)) = (node, other_node) {
                                self.check_can_merge_declarations(
                                    revision,
                                    existing,
                                    symbol,
                                    node,
                                    other_node,
                                    binding_ref.module_id,
                                    Some(key),
                                );
                            }
                            return Ok(Some(existing));
                        }
                        resolved = Some((existing, other_node));
                        continue;
                    }

                    // record the first resolved symbol and its node
                    let symbols = &dir.symbols;
                    let node = self.export_entry_node(binding_ref.module_id, &export, symbols);
                    resolved = Some((symbol, node));
                }

                Ok(resolved.map(|(symbol, _)| symbol))
            }
        }
    }

    /// Resolve a single export entry into a concrete symbol.
    fn resolve_export_entry_symbol(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        dir: &DirPrepared,
        scope_id: LocalScopeId,
        origin_module_id: ModuleId,
        origin_symbol: Option<GlobalSymbolId>,
        node: GlobalNodeIdAny,
        profile: ProfileId,
        default_name: StringId,
        export: Export,
        visited: &mut ReexportVisitStack,
        cache: Option<&mut ResolveDependencyItemCache>,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        match export.kind {
            ExportKind::Local => {
                // return the local symbol when present
                Ok(export.symbol.map(|symbol| symbol.into_global(module.id)))
            }
            ExportKind::ReExport => {
                // load the dependency item for this export
                let Some(item_id) = export.item else {
                    return Ok(None);
                };
                let item_node = item_id.into_global_any(module.id);
                let item = { dir.tree.get(item_id).clone() };

                // resolve the reexport target
                self.resolve_reexport_item_target(
                    revision,
                    module,
                    dir,
                    scope_id,
                    origin_module_id,
                    origin_symbol,
                    node,
                    profile,
                    default_name,
                    item_node,
                    item,
                    visited,
                    cache,
                )
            }
        }
    }

    /// Locate the scope and export symbols for a module binding declaration.
    pub(super) fn binding_info_for_declaration(
        &self,
        dir: &DirPrepared,
        declaration: LocalNodeId<Declaration>,
    ) -> Option<(
        LocalScopeId,
        destack_dir::LocalSymbolId,
        destack_dir::LocalSymbolId,
    )> {
        // find the binding entry for the declaration
        let binding = dir
            .module_bindings
            .iter()
            .find(|binding| binding.declaration == declaration)?;

        // return the binding scope and export symbols
        Some((
            binding.scope,
            binding.default_symbol,
            binding.export_assignment_symbol,
        ))
    }

    /// Resolve a node for an export entry, if any.
    fn export_entry_node(
        &self,
        module_id: ModuleId,
        export: &Export,
        symbols: &destack_dir::SymbolTable,
    ) -> Option<GlobalNodeIdAny> {
        // resolve a node for the export entry
        match export.kind {
            ExportKind::Local => export
                .symbol
                .and_then(|symbol| symbols.get_symbol(symbol).primary_declaration),
            ExportKind::ReExport => export.item.map(|item| item.into_global_any(module_id)),
        }
    }

    /// Resolve a reexport item to a concrete target symbol.
    fn resolve_reexport_item_target(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        dir: &DirPrepared,
        scope_id: LocalScopeId,
        origin_module_id: ModuleId,
        origin_symbol: Option<GlobalSymbolId>,
        node: GlobalNodeIdAny,
        profile: ProfileId,
        default_name: StringId,
        item_node: GlobalNodeIdAny,
        item: DependencyItem,
        visited: &mut ReexportVisitStack,
        mut cache: Option<&mut ResolveDependencyItemCache>,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        // resolve local or remote target symbols
        match item {
            DependencyItem::Local { target_symbol, .. }
            | DependencyItem::Remote { target_symbol, .. } => Ok(Some(target_symbol)),
            DependencyItem::UnresolvedLocal { kind, name, .. } => {
                // resolve the name from the module namespace scope
                let Some(name) = name else {
                    return Ok(None);
                };
                let symbols = &dir.symbols;
                let scope = symbols.get_scope_by_id(scope_id);
                let space_order = self.export_spaces_for_kind(kind);
                let key = StaticKey::Name(name.string());

                // try resolving in the local scope first, fall back to global augmentation scope
                // (for types defined in `global { }` blocks within module declarations)
                let pass = ResolveState::artifact(
                    revision,
                    module,
                    profile,
                    item_node,
                    space_order,
                    symbols,
                    dir.namespace_symbol,
                    dir.namespace_scope,
                    dir.global_augmentation_scope,
                    &dir.exported_symbols,
                    Some(&dir.tree),
                );
                let symbol_id = self
                    .resolve_absolute_symbol(
                        pass,
                        (scope_id, scope, LocalScopeMark::end()),
                        key,
                        cache.as_deref_mut().map(|cache| cache.scope_indices()),
                    )
                    .or_else(|_| {
                        let global_scope_id = dir.global_augmentation_scope;
                        let global_scope = symbols.get_scope_by_id(global_scope_id);
                        let pass = ResolveState::artifact(
                            revision,
                            module,
                            profile,
                            item_node,
                            space_order,
                            symbols,
                            dir.namespace_symbol,
                            dir.namespace_scope,
                            dir.global_augmentation_scope,
                            &dir.exported_symbols,
                            Some(&dir.tree),
                        );

                        self.resolve_absolute_symbol(
                            pass,
                            (global_scope_id, global_scope, LocalScopeMark::end()),
                            key,
                            cache.as_deref_mut().map(|cache| cache.scope_indices()),
                        )
                    })?;
                Ok(Some(symbol_id.into_global(module.id)))
            }
            DependencyItem::UnresolvedRemote {
                source,
                mode,
                kind,
                name,
                target,
                target_module,
                ..
            } => {
                let item_id = item_node.local_id.try_into_typed::<DependencyItem>().ok();
                let (expression_target_module, loader_override) = if let Some(item_id) = item_id {
                    let tree = &dir.tree;
                    let expression_target_module =
                        self.parent_expression_target_module_for_dependency_item(tree, item_id);
                    let loader_override = self
                        .parent_expression_loader_override_for_dependency_item(
                            module, profile, tree, item_id,
                        )?;
                    (expression_target_module, loader_override)
                } else {
                    (None, None)
                };

                // resolve the target module
                let target_module = if let Some(target_module) = target_module {
                    let Some(target_module) = target_module.for_kind(kind) else {
                        return Ok(None);
                    };
                    target_module
                } else if let Some(target_module) = expression_target_module {
                    target_module
                } else {
                    let Some(target_module) = self.resolve_import_maybe_from_artifact(
                        revision,
                        module,
                        dir,
                        profile,
                        item_node,
                        source,
                        target,
                        kind,
                        loader_override,
                    )?
                    else {
                        return Ok(None);
                    };
                    target_module
                };

                // namespace reexports produce a namespace symbol directly
                if mode == DependencyMode::Namespace {
                    if matches!(target_module, ModuleTarget::External(_)) {
                        return Ok(None);
                    }

                    let target_symbol = self.resolve_namespace_symbol(
                        revision,
                        origin_module_id,
                        node,
                        target_module,
                        profile,
                    )?;
                    return Ok(Some(target_symbol));
                }

                // resolve the import key and follow the reexport chain
                let Some(target_key) = self.reexport_import_key(mode, name, default_name) else {
                    return Ok(None);
                };
                let resolved = self.resolve_reexport_chain_symbol(
                    revision,
                    origin_module_id,
                    origin_symbol,
                    node,
                    target_module,
                    profile,
                    kind,
                    target_key,
                    default_name,
                    visited,
                    cache,
                )?;
                Ok(resolved.map(|resolved| resolved.symbol))
            }
            DependencyItem::Value { .. } | DependencyItem::Error => Ok(None),
        }
    }

    /// Resolve a remote item symbol result with cache access and optional binding fallback.
    pub(super) fn resolve_remote_item_symbol_result(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        node: GlobalNodeIdAny,
        remote_target: ModuleTarget,
        fallback_remote_target: Option<ModuleTarget>,
        profile: ProfileId,
        kind: DependencyKind,
        origin_symbol: Option<GlobalSymbolId>,
        key: StaticKey,
        target_specifier: Option<StringId>,
        mut cache: Option<&mut ResolveDependencyItemCache>,
    ) -> ResolveResult<(GlobalSymbolId, DependencyKind)> {
        // resolve against the selected module target first
        let resolved = self.resolve_remote_item_symbol_result_for_target(
            revision,
            module,
            node,
            remote_target,
            profile,
            kind,
            origin_symbol,
            key,
            cache.as_deref_mut(),
        );

        // retry against one explicit fallback target first
        if let Err(ResolveError::MissingSymbol { .. }) = &resolved
            && let Some(fallback_remote_target) = fallback_remote_target
            && fallback_remote_target != remote_target
        {
            return self.resolve_remote_item_symbol_result_for_target(
                revision,
                module,
                node,
                fallback_remote_target,
                profile,
                kind,
                origin_symbol,
                key,
                cache,
            );
        }

        // only fall back to module bindings when a module target is missing the symbol
        if let Err(ResolveError::MissingSymbol { .. }) = &resolved
            && matches!(remote_target, ModuleTarget::Module(_))
            && let Some(target_specifier) = target_specifier
            && let Some(binding_target) =
                self.resolve_module_binding_target(revision, module.id, profile, target_specifier)?
        {
            return self.resolve_remote_item_symbol_result_for_target(
                revision,
                module,
                node,
                binding_target,
                profile,
                kind,
                origin_symbol,
                key,
                cache,
            );
        }

        resolved
    }

    /// Resolve a remote item symbol result for one module target, using cache when present.
    fn resolve_remote_item_symbol_result_for_target(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        node: GlobalNodeIdAny,
        remote_target: ModuleTarget,
        profile: ProfileId,
        kind: DependencyKind,
        origin_symbol: Option<GlobalSymbolId>,
        key: StaticKey,
        cache: Option<&mut ResolveDependencyItemCache>,
    ) -> ResolveResult<(GlobalSymbolId, DependencyKind)> {
        // resolve with cache for repeated symbol lookups
        if let Some(cache) = cache {
            let cache_key = RemoteSymbolCacheKey {
                target: remote_target,
                kind,
                key,
                origin_module_id: self.cache_origin_module_id(module.id, remote_target),
            };
            if let Some(&cached) = cache.remote_symbols.get(&cache_key) {
                return Ok(cached);
            }

            let resolved = self.resolve_remote_item_symbol_result_uncached(
                revision,
                module,
                node,
                remote_target,
                profile,
                kind,
                origin_symbol,
                key,
                Some(cache),
            )?;
            cache.remote_symbols.insert(cache_key, resolved);

            return Ok(resolved);
        }

        // resolve directly when no cache is available
        self.resolve_remote_item_symbol_result_uncached(
            revision,
            module,
            node,
            remote_target,
            profile,
            kind,
            origin_symbol,
            key,
            None,
        )
    }

    /// Resolve a remote item symbol result and align dependency kind to export space.
    fn resolve_remote_item_symbol_result_uncached(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        node: GlobalNodeIdAny,
        remote_target: ModuleTarget,
        profile: ProfileId,
        kind: DependencyKind,
        origin_symbol: Option<GlobalSymbolId>,
        key: StaticKey,
        cache: Option<&mut ResolveDependencyItemCache>,
    ) -> ResolveResult<(GlobalSymbolId, DependencyKind)> {
        let resolved = self.resolve_remote_item_symbol(
            revision,
            module,
            node,
            remote_target,
            profile,
            kind,
            origin_symbol,
            key,
            cache,
        )?;
        let resolved_kind = self.effective_dependency_kind_for_export_space(kind, resolved);

        Ok((resolved.symbol, resolved_kind))
    }

    /// Resolve an item symbol in a remote module, searching through namespace exports if needed.
    fn resolve_remote_item_symbol(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        node: GlobalNodeIdAny,
        remote_target: ModuleTarget,
        profile: ProfileId,
        kind: DependencyKind,
        origin_symbol: Option<GlobalSymbolId>,
        key: StaticKey,
        mut cache: Option<&mut ResolveDependencyItemCache>,
    ) -> ResolveResult<ResolvedExportSymbol> {
        // resolve the requested kind first
        let resolved = self.resolve_remote_item_symbol_for_requested_kind(
            revision,
            module,
            node,
            remote_target,
            profile,
            kind,
            origin_symbol,
            key,
            cache.as_deref_mut(),
        );

        // fall back to type lookups for declaration modules
        if kind == DependencyKind::Value
            && let Err(ResolveError::MissingSymbol { .. }) = resolved
            && self.target_is_declaration_module(revision, module.id, remote_target, profile)?
        {
            return self.resolve_remote_item_symbol_for_requested_kind(
                revision,
                module,
                node,
                remote_target,
                profile,
                DependencyKind::Type,
                origin_symbol,
                key,
                cache,
            );
        }

        resolved
    }

    /// Adjust a dependency kind based on the export space of the resolved symbol.
    fn effective_dependency_kind_for_export_space(
        &self,
        kind: DependencyKind,
        resolved: ResolvedExportSymbol,
    ) -> DependencyKind {
        if kind == DependencyKind::Value && resolved.export_space == SymbolSpace::Type {
            DependencyKind::Type
        } else {
            kind
        }
    }

    /// Resolve an item symbol for a specific dependency kind.
    fn resolve_remote_item_symbol_for_requested_kind(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        node: GlobalNodeIdAny,
        remote_target: ModuleTarget,
        profile: ProfileId,
        kind: DependencyKind,
        origin_symbol: Option<GlobalSymbolId>,
        key: StaticKey,
        mut cache: Option<&mut ResolveDependencyItemCache>,
    ) -> ResolveResult<ResolvedExportSymbol> {
        let default_name = self.repository.strings.intern("default");

        // resolve explicit exports and reexport chains first
        let mut visited = ReexportVisitStack::default();
        let resolved_symbol = self.resolve_reexport_chain_symbol(
            revision,
            module.id,
            origin_symbol,
            node,
            remote_target,
            profile,
            kind,
            key,
            default_name,
            &mut visited,
            cache.as_deref_mut(),
        )?;
        if let Some(symbol_id) = resolved_symbol {
            return Ok(symbol_id);
        }

        // resolve default imports from export assignments
        if kind == DependencyKind::Value && key == StaticKey::Name(default_name) {
            if let Some(symbol) =
                self.resolve_export_assignment_symbol(revision, module.id, remote_target, profile)?
            {
                return Ok(ResolvedExportSymbol {
                    symbol,
                    export_space: SymbolSpace::Value,
                });
            }
        }

        // fall back to export assignment target (e.g., `export = X`)
        // handles both module redirects and local namespace lookups
        if let Some(target) = self.resolve_export_assignment_target(
            revision,
            module.id,
            remote_target,
            profile,
            cache.as_deref_mut(),
        )? {
            match target {
                ExportAssignmentTarget::Module(redirect_target) => {
                    // recursively resolve the symbol in the redirected module
                    return self.resolve_remote_item_symbol(
                        revision,
                        module,
                        node,
                        redirect_target,
                        profile,
                        kind,
                        origin_symbol,
                        key,
                        cache.as_deref_mut(),
                    );
                }
                ExportAssignmentTarget::Namespace(namespace_symbol) => {
                    // look up the key in the namespace symbol's scope
                    if let Some(symbol) = self.resolve_symbol_in_namespace(
                        revision,
                        node,
                        namespace_symbol,
                        profile,
                        kind,
                        key,
                        cache.as_deref_mut(),
                    )? {
                        let export_space = match kind {
                            DependencyKind::Type => SymbolSpace::Type,
                            DependencyKind::Value => SymbolSpace::Value,
                        };
                        return Ok(ResolvedExportSymbol {
                            symbol,
                            export_space,
                        });
                    }
                }
            }
        }

        // fall back to namespace exports
        let global_symbol = self.resolve_symbol_via_namespace_exports(
            revision,
            module,
            node,
            remote_target,
            profile,
            kind,
            key,
            origin_symbol,
            default_name,
            cache,
        )?;

        Ok(global_symbol)
    }

    /// Check if a target module represents declaration-only sources.
    ///
    /// Resolve needs this to decide whether a value reexport may fall back to the type space.
    /// Declaration modules often export types without `export type`, so value lookups can fail.
    /// We only allow the fallback when the *target* is declaration-only to avoid masking errors
    /// in runtime modules.
    fn target_is_declaration_module(
        &self,
        revision: Revision,
        origin_module_id: ModuleId,
        target: ModuleTarget,
        profile: ProfileId,
    ) -> ResolveResult<bool> {
        // module targets are declaration-only when the module says so
        if let ModuleTarget::Module(module_id) = target {
            let module = self
                .cache_module_snapshot(revision, module_id)
                .map_err(|error| ResolveError::Internal {
                    message: format!("failed to load module snapshot: {error}"),
                })?;
            let module = module.as_ref();
            return Ok(module.language_type.is_declaration());
        }

        // specifier bindings are declaration-only when all bindings are declaration modules
        if let ModuleTarget::Binding(specifier) = target {
            let bindings =
                self.module_bindings_for_specifier(revision, origin_module_id, profile, specifier)?;
            let Some(bindings) = bindings else {
                return Ok(false);
            };

            // if any binding module is not a declaration module, treat it as mixed
            for binding_ref in bindings {
                let module = self
                    .cache_module_snapshot(revision, binding_ref.module_id)
                    .map_err(|error| ResolveError::Internal {
                        message: format!("failed to load module snapshot: {error}"),
                    })?;
                let module = module.as_ref();
                if !module.language_type.is_declaration() {
                    return Ok(false);
                }
            }

            return Ok(true);
        }

        if let ModuleTarget::External(_) = target {
            return Ok(false);
        }

        Ok(false)
    }
}
