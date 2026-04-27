use destack_artifact::DirPrepared;
use destack_ast::StringId;
use destack_dir::{
    DependencyItem, DependencyKind, DependencyMode, Expression, GlobalNodeIdAny, GlobalSymbolId,
    ImportSource, LocalScopeId, ModuleTarget, StaticKey, SymbolTable, Tree,
};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Revision};
use rustc_hash::FxHashMap;

use crate::resolve::dependency::cache::{
    BindingExportCacheKey, ExportAssignmentTarget, ResolveDependencyItemCache,
};
use crate::{Compiler, ResolveError, ResolveResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve the namespace symbol for a target module.
    pub(super) fn resolve_namespace_symbol(
        &self,
        revision: Revision,
        origin_module_id: ModuleId,
        node: GlobalNodeIdAny,
        target: ModuleTarget,
        profile: ProfileId,
    ) -> ResolveResult<GlobalSymbolId> {
        match target {
            ModuleTarget::Module(module_id) => {
                // ensure the target module is prepared
                self.require_dir_prepared_if_other(revision, origin_module_id, module_id, profile)?;

                // load the module namespace symbol
                let target_dir = self
                    .require_artifact_dir_prepared(revision, module_id, profile)
                    .map_err(ResolveError::from)?;
                Ok(target_dir.namespace_symbol.into_global(module_id))
            }
            ModuleTarget::Binding(specifier) => {
                // load bindings for the specifier
                let bindings = self.module_bindings_for_specifier(
                    revision,
                    origin_module_id,
                    profile,
                    specifier,
                )?;

                // report unresolved modules when no bindings match
                let Some(bindings) = bindings else {
                    return Err(ResolveError::UnresolvedModule {
                        node: node.into_anchored(Some(profile)),
                        target: specifier,
                    });
                };

                // select the first binding declaration
                let Some(binding_ref) = bindings.first() else {
                    return Err(ResolveError::UnresolvedModule {
                        node: node.into_anchored(Some(profile)),
                        target: specifier,
                    });
                };

                // ensure the binding module is prepared
                self.require_dir_prepared_if_other(
                    revision,
                    origin_module_id,
                    binding_ref.module_id,
                    profile,
                )?;

                // resolve the declaration symbol for the binding
                let dir = self
                    .require_artifact_dir_prepared(revision, binding_ref.module_id, profile)
                    .map_err(ResolveError::from)?;
                let tree = &dir.tree;
                let declaration = tree.get(binding_ref.declaration);
                let symbol_id = declaration.symbol();
                Ok(symbol_id.into_global(binding_ref.module_id))
            }
            ModuleTarget::External(specifier) => Err(ResolveError::UnresolvedModule {
                node: node.into_anchored(Some(profile)),
                target: specifier,
            }),
        }
    }

    /// Resolve the export assignment symbol for a target module, if present.
    pub(super) fn resolve_export_assignment_symbol(
        &self,
        revision: Revision,
        origin_module_id: ModuleId,
        target: ModuleTarget,
        profile: ProfileId,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        match target {
            ModuleTarget::Module(module_id) => {
                // non-code modules (data, text, binary) don't have export assignments
                let module = self
                    .cache_module_snapshot(revision, module_id)
                    .map_err(|error| ResolveError::Internal {
                        message: format!("failed to load module snapshot: {error}"),
                    })?;
                if !module.is_code() {
                    return Ok(None);
                }

                // ensure the target module is prepared
                self.require_dir_prepared_if_other(revision, origin_module_id, module_id, profile)?;

                // load the module export assignment state
                let target_dir = self
                    .require_artifact_dir_prepared(revision, module_id, profile)
                    .map_err(ResolveError::from)?;
                if target_dir.export_assignment.is_some() {
                    return Ok(Some(
                        target_dir.export_assignment_symbol.into_global(module_id),
                    ));
                }
                Ok(None)
            }
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

                // track the first resolved export assignment symbol
                let mut resolved: Option<(GlobalSymbolId, Option<GlobalNodeIdAny>)> = None;
                for binding_ref in bindings {
                    // ensure the binding module is prepared
                    self.require_dir_prepared_if_other(
                        revision,
                        origin_module_id,
                        binding_ref.module_id,
                        profile,
                    )?;

                    // load the binding export assignment entry
                    let dir = self
                        .require_artifact_dir_prepared(revision, binding_ref.module_id, profile)
                        .map_err(ResolveError::from)?;
                    let binding_exports = dir
                        .module_binding_exports
                        .get(&binding_ref.declaration.into_any())
                        .cloned();
                    let Some(binding_exports) = binding_exports else {
                        continue;
                    };
                    let Some((_, _, export_assignment_symbol)) =
                        self.binding_info_for_declaration(&dir, binding_ref.declaration)
                    else {
                        continue;
                    };
                    let Some(item_id) = binding_exports.export_assignment else {
                        continue;
                    };

                    // report conflicting export assignment symbols
                    let symbol = export_assignment_symbol.into_global(binding_ref.module_id);
                    if let Some((existing, other_node)) = resolved {
                        if existing != symbol {
                            let node = item_id.into_global_any(binding_ref.module_id);
                            if let Some(other_node) = other_node {
                                self.check_can_merge_declarations(
                                    revision,
                                    existing,
                                    symbol,
                                    node,
                                    other_node,
                                    binding_ref.module_id,
                                    None,
                                );
                            }
                            return Ok(Some(existing));
                        }
                        resolved = Some((existing, other_node));
                        continue;
                    }

                    // record the first export assignment symbol
                    resolved = Some((symbol, Some(item_id.into_global_any(binding_ref.module_id))));
                }

                Ok(resolved.map(|(symbol, _)| symbol))
            }
            ModuleTarget::External(_) => Ok(None),
        }
    }

    /// Resolve the export assignment target for a module, if present.
    /// Returns either a module to redirect to, or a namespace symbol to look inside.
    /// This is used to follow `export =` chains when resolving named imports.
    pub(super) fn resolve_export_assignment_target(
        &self,
        revision: destack_workspace::Revision,
        origin_module_id: ModuleId,
        target: ModuleTarget,
        profile: ProfileId,
        mut cache: Option<&mut ResolveDependencyItemCache>,
    ) -> ResolveResult<Option<ExportAssignmentTarget>> {
        let cache_key = cache
            .as_ref()
            .map(|_| self.cache_target_key(origin_module_id, target));
        if let (Some(cache), Some(cache_key)) = (cache.as_deref(), cache_key)
            && let Some(&cached) = cache.export_assignment_targets.get(&cache_key)
        {
            return Ok(cached);
        }

        match target {
            ModuleTarget::Module(module_id) => {
                // ensure the target module is prepared
                self.require_dir_prepared_if_other(revision, origin_module_id, module_id, profile)?;

                // check if module has an export assignment
                let dir = self
                    .require_artifact_dir_prepared(revision, module_id, profile)
                    .map_err(ResolveError::from)?;
                let export_assignment = dir.export_assignment;
                let Some(item_id) = export_assignment else {
                    if let (Some(cache), Some(cache_key)) = (cache.as_deref_mut(), cache_key) {
                        cache.export_assignment_targets.insert(cache_key, None);
                    }
                    return Ok(None);
                };

                // resolve the export assignment item
                let tree = &dir.tree;
                let item = tree.get(item_id);
                let resolved = self.resolve_export_assignment_target_for_item(
                    revision,
                    module_id,
                    profile,
                    &dir,
                    tree,
                    dir.namespace_scope,
                    item,
                    cache.as_deref_mut(),
                )?;
                if let (Some(cache), Some(cache_key)) = (cache.as_deref_mut(), cache_key) {
                    cache.export_assignment_targets.insert(cache_key, resolved);
                }
                Ok(resolved)
            }
            ModuleTarget::Binding(specifier) => {
                // load bindings for the specifier
                let bindings = self.module_bindings_for_specifier(
                    revision,
                    origin_module_id,
                    profile,
                    specifier,
                )?;
                let Some(bindings) = bindings else {
                    if let (Some(cache), Some(cache_key)) = (cache.as_deref_mut(), cache_key) {
                        cache.export_assignment_targets.insert(cache_key, None);
                    }
                    return Ok(None);
                };

                // check each binding for an export assignment target
                for binding_ref in bindings {
                    // ensure the binding module is prepared
                    self.require_dir_prepared_if_other(
                        revision,
                        origin_module_id,
                        binding_ref.module_id,
                        profile,
                    )?;

                    // load the binding export assignment entry
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
                    let Some(item_id) = binding_exports.export_assignment else {
                        continue;
                    };

                    // get binding scope for import lookup
                    let Some((scope_id, _, _)) =
                        self.binding_info_for_declaration(&dir, binding_ref.declaration)
                    else {
                        continue;
                    };

                    // read the export assignment dependency item
                    let tree = &dir.tree;
                    let item = tree.get(item_id);

                    // resolve the export assignment item
                    if let Some(target) = self.resolve_export_assignment_target_for_item(
                        revision,
                        binding_ref.module_id,
                        profile,
                        &dir,
                        tree,
                        scope_id,
                        item,
                        cache.as_deref_mut(),
                    )? {
                        if let (Some(cache), Some(cache_key)) = (cache.as_deref_mut(), cache_key) {
                            cache
                                .export_assignment_targets
                                .insert(cache_key, Some(target));
                        }
                        return Ok(Some(target));
                    }
                }

                if let (Some(cache), Some(cache_key)) = (cache, cache_key) {
                    cache.export_assignment_targets.insert(cache_key, None);
                }
                Ok(None)
            }
            ModuleTarget::External(_) => {
                if let (Some(cache), Some(cache_key)) = (cache, cache_key) {
                    cache.export_assignment_targets.insert(cache_key, None);
                }
                Ok(None)
            }
        }
    }

    /// Resolve an export assignment target from a dependency item.
    fn resolve_export_assignment_target_for_item(
        &self,
        revision: destack_workspace::Revision,
        module_id: ModuleId,
        profile: ProfileId,
        dir: &DirPrepared,
        tree: &Tree,
        scope_id: LocalScopeId,
        item: &DependencyItem,
        cache: Option<&mut ResolveDependencyItemCache>,
    ) -> ResolveResult<Option<ExportAssignmentTarget>> {
        // resolve the assignment target based on the dependency item
        match item {
            DependencyItem::Remote { target_module, .. } => Ok(target_module
                .for_kind(DependencyKind::Value)
                .map(ExportAssignmentTarget::Module)),
            DependencyItem::UnresolvedRemote {
                target_module: Some(target_module),
                ..
            } => Ok(target_module
                .for_kind(DependencyKind::Value)
                .map(ExportAssignmentTarget::Module)),
            DependencyItem::Local { target_symbol, .. } => self
                .resolve_export_assignment_target_for_local_symbol(
                    revision,
                    module_id,
                    profile,
                    dir,
                    tree,
                    scope_id,
                    *target_symbol,
                    cache,
                ),
            DependencyItem::Value { value, .. } => self.resolve_export_assignment_value_target(
                revision, module_id, profile, dir, tree, scope_id, *value, cache,
            ),
            _ => Ok(None),
        }
    }

    /// Resolve an export assignment target from a local dependency symbol.
    fn resolve_export_assignment_target_for_local_symbol(
        &self,
        revision: destack_workspace::Revision,
        module_id: ModuleId,
        profile: ProfileId,
        dir: &DirPrepared,
        tree: &Tree,
        scope_id: LocalScopeId,
        target_symbol: GlobalSymbolId,
        cache: Option<&mut ResolveDependencyItemCache>,
    ) -> ResolveResult<Option<ExportAssignmentTarget>> {
        // prefer import alias redirects for `export = alias` targets
        let symbols = &dir.symbols;
        let symbol = symbols.get_symbol(target_symbol.local_id);
        let symbol_name = symbol.name();

        if let Some(symbol_name) = symbol_name
            && let Some(redirect) = self.find_import_redirect_for_name(
                revision,
                module_id,
                profile,
                tree,
                scope_id,
                symbol_name,
                cache,
            )?
        {
            return Ok(Some(ExportAssignmentTarget::Module(redirect)));
        }

        // otherwise the assignment points at a local namespace-like symbol
        Ok(Some(ExportAssignmentTarget::Namespace(target_symbol)))
    }

    /// Resolve an export assignment target from a value expression.
    fn resolve_export_assignment_value_target(
        &self,
        revision: destack_workspace::Revision,
        module_id: ModuleId,
        profile: ProfileId,
        dir: &DirPrepared,
        tree: &Tree,
        scope_id: LocalScopeId,
        value: destack_dir::LocalNodeId<Expression>,
        mut cache: Option<&mut ResolveDependencyItemCache>,
    ) -> ResolveResult<Option<ExportAssignmentTarget>> {
        // handle resolved references
        let expr = tree.get(value);
        if let Expression::LocalReference {
            path,
            target_symbol,
            ..
        }
        | Expression::ModuleReference {
            path,
            target_symbol,
            ..
        }
        | Expression::GlobalReference {
            path,
            target_symbol,
            ..
        } = expr
        {
            if let Some(name) = path.first_segment()
                && let Some(redirect) = self.find_import_redirect_for_name(
                    revision,
                    module_id,
                    profile,
                    tree,
                    scope_id,
                    name,
                    cache.as_deref_mut(),
                )?
            {
                return Ok(Some(ExportAssignmentTarget::Module(redirect)));
            }

            return Ok(Some(ExportAssignmentTarget::Namespace(*target_symbol)));
        }

        // handle unresolved paths
        if let Expression::UnresolvedPath { path, .. } = expr
            && let Some(name) = path.first_segment()
        {
            if let Some(redirect) = self.find_import_redirect_for_name(
                revision, module_id, profile, tree, scope_id, name, cache,
            )? {
                return Ok(Some(ExportAssignmentTarget::Module(redirect)));
            }

            let symbols = &dir.symbols;
            let key = StaticKey::Name(name);
            let symbol_id = self.find_namespace_symbol_in_scope(symbols, scope_id, key);
            if let Some(symbol_id) = symbol_id {
                return Ok(Some(ExportAssignmentTarget::Namespace(
                    symbol_id.into_global(module_id),
                )));
            }
        }

        Ok(None)
    }

    /// Find a namespace symbol in a scope, falling back to any matching symbol.
    fn find_namespace_symbol_in_scope(
        &self,
        symbols: &SymbolTable,
        scope_id: LocalScopeId,
        key: StaticKey,
    ) -> Option<destack_dir::LocalSymbolId> {
        // prefer namespace symbols for class or namespace merges
        let scope = symbols.get_scope_by_id(scope_id);
        let mut fallback = None;
        for (candidate_key, symbol_id) in symbols.active_named_symbols(scope) {
            if candidate_key != key {
                continue;
            }

            let symbol = symbols.get_symbol(symbol_id);
            if symbol.kind == destack_dir::SymbolKind::Namespace {
                return Some(symbol_id);
            }

            if fallback.is_none() {
                fallback = Some(symbol_id);
            }
        }

        fallback
    }

    /// Find an import redirect target for a name in a binding scope.
    /// Used to resolve `export = X` where X is an import alias.
    fn find_import_redirect_for_name(
        &self,
        revision: destack_workspace::Revision,
        module_id: ModuleId,
        profile: ProfileId,
        tree: &Tree,
        scope_id: LocalScopeId,
        name: StringId,
        cache: Option<&mut ResolveDependencyItemCache>,
    ) -> ResolveResult<Option<ModuleTarget>> {
        if let Some(cache) = cache {
            if !cache.import_redirects_by_scope.contains_key(&module_id) {
                let item_ids = cache.dependency_item_ids_for(module_id, tree);
                let redirects_by_scope = self.build_import_redirects_by_scope(
                    revision, module_id, profile, tree, &item_ids,
                )?;
                cache
                    .import_redirects_by_scope
                    .insert(module_id, redirects_by_scope);
            }

            if let Some(scope_redirects) = cache.import_redirects_by_scope.get(&module_id)
                && let Some(redirects) = scope_redirects.get(&scope_id)
                && let Some(&target) = redirects.get(&name)
            {
                return Ok(Some(target));
            }

            return Ok(None);
        }

        self.find_import_redirect_for_name_uncached(
            revision, module_id, profile, tree, scope_id, name,
        )
    }

    /// Build a lookup table of import redirect targets per binding scope.
    fn build_import_redirects_by_scope(
        &self,
        revision: destack_workspace::Revision,
        module_id: ModuleId,
        profile: ProfileId,
        tree: &Tree,
        item_ids: &[destack_dir::LocalNodeId<DependencyItem>],
    ) -> ResolveResult<FxHashMap<LocalScopeId, FxHashMap<StringId, ModuleTarget>>> {
        let mut redirects_by_scope: FxHashMap<LocalScopeId, FxHashMap<StringId, ModuleTarget>> =
            FxHashMap::default();

        for item_id in item_ids {
            // extract the item scope once
            let (item_scope_id, _) = tree.get_scope(*item_id);
            let item = tree.get(*item_id);
            match item {
                DependencyItem::UnresolvedRemote {
                    source: ImportSource::ImportEquals | ImportSource::RequireCall,
                    mode: DependencyMode::Namespace,
                    kind,
                    name,
                    alias,
                    target,
                    ..
                } => {
                    let name = name.map(|name| name.string());
                    let Some(name) = alias.or(name) else {
                        continue;
                    };

                    // resolve module bindings before falling back to module specifiers
                    if let Some(binding_target) =
                        self.resolve_module_binding_target(revision, module_id, profile, *target)?
                    {
                        redirects_by_scope
                            .entry(item_scope_id)
                            .or_default()
                            .insert(name, binding_target);
                        continue;
                    }

                    // resolve the specifier to a module target
                    if let Ok(targets) = self.resolve_specifier_to_module_resolution(
                        revision,
                        profile,
                        *target,
                        Some(module_id),
                        destack_artifact::ModuleEdgeRelation::Require,
                        None,
                    ) && let Some(target_module) = targets.for_kind(*kind)
                    {
                        redirects_by_scope
                            .entry(item_scope_id)
                            .or_default()
                            .insert(name, target_module);
                    }
                }
                DependencyItem::Remote {
                    mode: DependencyMode::Namespace,
                    kind,
                    name,
                    alias,
                    target_module,
                    ..
                } => {
                    let name = name.map(|name| name.string());
                    let Some(name) = alias.or(name) else {
                        continue;
                    };
                    if let Some(target_module) = target_module.for_kind(*kind) {
                        redirects_by_scope
                            .entry(item_scope_id)
                            .or_default()
                            .insert(name, target_module);
                    }
                }
                _ => {}
            }
        }

        Ok(redirects_by_scope)
    }

    /// Search dependency items for a redirect target without using the cache.
    fn find_import_redirect_for_name_uncached(
        &self,
        revision: destack_workspace::Revision,
        module_id: ModuleId,
        profile: ProfileId,
        tree: &Tree,
        scope_id: LocalScopeId,
        name: StringId,
    ) -> ResolveResult<Option<ModuleTarget>> {
        // scan dependency items in the scope
        for item_id in tree.iter_node_ids_of_type::<DependencyItem>() {
            // skip items outside the binding scope
            let (item_scope_id, _) = tree.get_scope(item_id);
            if item_scope_id != scope_id {
                continue;
            }

            // check if this is an import that declares the name we're looking for
            let item = tree.get(item_id);
            match item {
                DependencyItem::UnresolvedRemote {
                    source,
                    mode: DependencyMode::Namespace,
                    kind,
                    name: item_name,
                    alias,
                    target,
                    ..
                } if matches!(
                    source,
                    ImportSource::ImportEquals | ImportSource::RequireCall
                ) && alias.or(item_name.map(|name| name.string())) == Some(name) =>
                {
                    // found `import X = require("target")` where X is our name
                    // resolve module bindings before falling back to module specifiers
                    if let Some(binding_target) =
                        self.resolve_module_binding_target(revision, module_id, profile, *target)?
                    {
                        return Ok(Some(binding_target));
                    }

                    // resolve the specifier to a module target
                    if let Ok(targets) = self.resolve_specifier_to_module_resolution(
                        revision,
                        profile,
                        *target,
                        Some(module_id),
                        destack_artifact::ModuleEdgeRelation::Require,
                        None,
                    ) {
                        return Ok(targets.for_kind(*kind));
                    }
                    return Ok(None);
                }
                DependencyItem::Remote {
                    mode: DependencyMode::Namespace,
                    kind,
                    name: item_name,
                    alias,
                    target_module,
                    ..
                } if alias.or(item_name.map(|name| name.string())) == Some(name) => {
                    // already resolved import, use its target module
                    return Ok(target_module.for_kind(*kind));
                }
                _ => continue,
            }
        }

        Ok(None)
    }

    /// Resolve a symbol by looking inside a namespace symbol's scope.
    /// Used for `export = LocalNamespace` where we need to find members of the namespace.
    pub(crate) fn resolve_symbol_in_namespace(
        &self,
        revision: Revision,
        _node: GlobalNodeIdAny,
        namespace_symbol: GlobalSymbolId,
        profile: ProfileId,
        kind: DependencyKind,
        key: StaticKey,
        mut cache: Option<&mut ResolveDependencyItemCache>,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        let cache_key =
            cache.as_ref().map(
                |_| crate::resolve::dependency::cache::NamespaceSymbolCacheKey {
                    symbol: namespace_symbol,
                    kind,
                    key,
                },
            );
        if let (Some(cache), Some(cache_key)) = (cache.as_deref(), cache_key)
            && let Some(&cached) = cache.namespace_symbols.get(&cache_key)
        {
            return Ok(cached);
        }

        // load the namespace symbol's module and check if it's a namespace
        let module = self
            .cache_module_snapshot(revision, namespace_symbol.module_id)
            .map_err(|error| ResolveError::Internal {
                message: format!("failed to load module snapshot: {error}"),
            })?;
        let dir = self
            .require_artifact_dir_prepared(revision, namespace_symbol.module_id, profile)
            .map_err(ResolveError::from)?;
        let symbols = &dir.symbols;

        let symbol = symbols.get_symbol(namespace_symbol.local_id);

        // if this is a namespace, look in its scope
        if symbol.kind == destack_dir::SymbolKind::Namespace {
            let (scope_id, _) = symbol.scope;
            let scope = symbols.get_scope_by_id(scope_id);
            let space_order = self.export_spaces_for_kind(kind);

            let (preferred, fallback) =
                self.find_symbol_in_scope(scope, key, space_order, symbols, None);

            if let Some(symbol_id) = preferred.or(fallback) {
                let resolved = Some(symbol_id.into_global(namespace_symbol.module_id));
                if let (Some(cache), Some(cache_key)) = (cache.as_deref_mut(), cache_key) {
                    cache.namespace_symbols.insert(cache_key, resolved);
                }
                return Ok(resolved);
            }
        }

        // if not a namespace (or not found), check for merged namespace with same name
        // this handles `class Foo {} namespace Foo {}` where export = Foo points to the class
        if let Some(name) = symbol.name() {
            let symbol_key = StaticKey::Name(name);
            let (scope_id, _) = symbol.scope;
            let scope = symbols.get_scope_by_id(scope_id);

            // look for a namespace symbol with the same name in the same scope
            for (candidate_key, sym_id) in symbols.active_named_symbols(scope) {
                if candidate_key == symbol_key {
                    let other_symbol = symbols.get_symbol(sym_id);
                    if other_symbol.kind == destack_dir::SymbolKind::Namespace {
                        // found a merged namespace, look in its scope
                        let (ns_scope_id, _) = other_symbol.scope;
                        let ns_scope = symbols.get_scope_by_id(ns_scope_id);
                        let space_order = self.export_spaces_for_kind(kind);

                        let (preferred, fallback) =
                            self.find_symbol_in_scope(ns_scope, key, space_order, symbols, None);

                        if let Some(symbol_id) = preferred.or(fallback) {
                            let resolved = Some(symbol_id.into_global(namespace_symbol.module_id));
                            if let (Some(cache), Some(cache_key)) =
                                (cache.as_deref_mut(), cache_key)
                            {
                                cache.namespace_symbols.insert(cache_key, resolved);
                            }
                            return Ok(resolved);
                        }
                    }
                }
            }
        }

        // declaration export assignments may expose members through value types
        // this path only runs for `export =` namespace-target lookups
        if kind == DependencyKind::Value && module.language_type.is_declaration() {
            let resolved = Some(namespace_symbol);
            if let (Some(cache), Some(cache_key)) = (cache.as_deref_mut(), cache_key) {
                cache.namespace_symbols.insert(cache_key, resolved);
            }
            return Ok(resolved);
        }
        if let (Some(cache), Some(cache_key)) = (cache, cache_key) {
            cache.namespace_symbols.insert(cache_key, None);
        }

        Ok(None)
    }
}
