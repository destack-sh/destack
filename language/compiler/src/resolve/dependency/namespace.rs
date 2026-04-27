use std::collections::VecDeque;

use destack_artifact::DirPrepared;
use destack_ast::StringId;
use destack_dir::{
    DependencyItem, DependencyKind, DependencyMode, Expression, GlobalNodeIdAny, GlobalSymbolId,
    ImportSource, LocalNodeId, LocalScopeId, ModuleTarget, StaticKey, Tree,
};
use destack_source::ModuleId;
use destack_workspace::workspace::{Module, ProfileId};
use rustc_hash::FxHashMap;

use crate::resolve::dependency::cache::{
    NamespaceExportSymbolCacheKey, ResolveDependencyItemCache,
};
use crate::resolve::dependency::dependency::{ReexportVisitStack, ResolvedExportSymbol};
use crate::resolve::dependency::loader::LoaderAttribute;
use crate::{Compiler, ResolveError, ResolveResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Collect namespace reexport targets for a specific scope.
    pub(super) fn collect_namespace_exports_for_scope(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile: ProfileId,
        scope_id: LocalScopeId,
        cache: Option<&mut ResolveDependencyItemCache>,
        dir: &DirPrepared,
    ) -> ResolveResult<Vec<destack_dir::NamespaceExport>> {
        // use cached exports when available
        if let Some(cache) = cache {
            if !cache.namespace_exports_by_scope.contains_key(&module.id) {
                let tree = &dir.tree;
                let item_ids = cache.dependency_item_ids_for(module.id, tree);
                let exports_by_scope = self.build_namespace_exports_by_scope(
                    revision, module, profile, dir, tree, &item_ids,
                )?;
                cache
                    .namespace_exports_by_scope
                    .insert(module.id, exports_by_scope);
            }

            if let Some(exports_by_scope) = cache.namespace_exports_by_scope.get(&module.id)
                && let Some(exports) = exports_by_scope.get(&scope_id)
            {
                return Ok(exports.clone());
            }

            return Ok(Vec::new());
        }

        // fall back to a local scan without caching
        let tree = &dir.tree;
        let item_ids = tree.iter_node_ids_of_type::<DependencyItem>();
        self.collect_namespace_exports_in_scope_direct(
            revision, module, profile, dir, tree, &item_ids, scope_id,
        )
    }

    /// Collect namespace exports in a scope without caching.
    fn collect_namespace_exports_in_scope_direct(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile: ProfileId,
        dir: &DirPrepared,
        tree: &Tree,
        item_ids: &[LocalNodeId<DependencyItem>],
        scope_id: LocalScopeId,
    ) -> ResolveResult<Vec<destack_dir::NamespaceExport>> {
        let mut exports = Vec::new();

        // walk dependency items for namespace exports
        for item_id in item_ids {
            let Some(export) = self.namespace_export_for_item(
                revision, module, profile, dir, tree, *item_id, scope_id,
            )?
            else {
                continue;
            };
            exports.push(export);
        }

        Ok(exports)
    }

    /// Build a scope grouped namespace export table for the module.
    fn build_namespace_exports_by_scope(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile: ProfileId,
        dir: &DirPrepared,
        tree: &Tree,
        item_ids: &[LocalNodeId<DependencyItem>],
    ) -> ResolveResult<FxHashMap<LocalScopeId, Vec<destack_dir::NamespaceExport>>> {
        let mut exports_by_scope: FxHashMap<LocalScopeId, Vec<destack_dir::NamespaceExport>> =
            FxHashMap::default();

        // walk dependency items once and group exports by scope
        for item_id in item_ids {
            let Some((scope_id, export)) = self.namespace_export_for_item_with_scope(
                revision, module, profile, dir, tree, *item_id,
            )?
            else {
                continue;
            };
            exports_by_scope.entry(scope_id).or_default().push(export);
        }

        Ok(exports_by_scope)
    }

    /// Resolve a namespace export for a dependency item in the given scope.
    fn namespace_export_for_item(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile: ProfileId,
        dir: &DirPrepared,
        tree: &Tree,
        item_id: LocalNodeId<DependencyItem>,
        scope_id: LocalScopeId,
    ) -> ResolveResult<Option<destack_dir::NamespaceExport>> {
        // ensure the dependency item belongs to the requested scope
        let Some((item_scope_id, export)) = self
            .namespace_export_for_item_with_scope(revision, module, profile, dir, tree, item_id)?
        else {
            return Ok(None);
        };
        if item_scope_id != scope_id {
            return Ok(None);
        }

        Ok(Some(export))
    }

    /// Resolve a namespace export and return its owning scope.
    fn namespace_export_for_item_with_scope(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile: ProfileId,
        dir: &DirPrepared,
        tree: &Tree,
        item_id: LocalNodeId<DependencyItem>,
    ) -> ResolveResult<Option<(LocalScopeId, destack_dir::NamespaceExport)>> {
        // extract namespace export metadata
        let item = tree.get(item_id);
        let (mode, kind, alias, source, target, target_module) = match item {
            DependencyItem::UnresolvedRemote {
                mode,
                kind,
                alias,
                source,
                target,
                target_module,
                ..
            } => (
                *mode,
                *kind,
                *alias,
                Some(*source),
                Some(*target),
                *target_module,
            ),
            DependencyItem::Remote {
                mode,
                kind,
                alias,
                target_module,
                ..
            } => (*mode, *kind, *alias, None, None, Some(*target_module)),
            _ => return Ok(None),
        };

        // namespace export statements must be export * from without alias
        if mode != DependencyMode::Namespace || alias.is_some() {
            return Ok(None);
        }
        if let Some(source) = source
            && source != ImportSource::ExportStatement
        {
            return Ok(None);
        }

        // ensure this dependency item belongs to an export expression
        let Some(parent_id) = tree.get_parent(item_id.id) else {
            return Ok(None);
        };
        let Ok(parent_id) = parent_id.try_into_typed::<Expression>() else {
            return Ok(None);
        };
        if !matches!(
            tree.get(parent_id),
            Expression::Export { .. }
                | Expression::UnresolvedReExport { .. }
                | Expression::ReExport { .. }
        ) {
            return Ok(None);
        }

        let expression_target_module =
            self.parent_expression_target_module_for_dependency_item(tree, item_id);
        let loader_override = self.parent_expression_loader_override_for_dependency_item(
            module, profile, tree, item_id,
        )?;

        // resolve the target module
        let target_module = if let Some(target_module) = target_module {
            let Some(target_module) = target_module.for_kind(kind) else {
                return Ok(None);
            };
            target_module
        } else if let Some(target_module) = expression_target_module {
            target_module
        } else if let Some(target) = target {
            let Some(target_module) = self.resolve_import_maybe_from_artifact(
                revision,
                module,
                dir,
                profile,
                item_id.into_global_any(module.id),
                source.unwrap_or(ImportSource::ExportStatement),
                target,
                kind,
                loader_override,
            )?
            else {
                return Ok(None);
            };
            target_module
        } else {
            return Ok(None);
        };

        // register the namespace export edge
        let (parent_scope_id, _) = tree.get_scope(parent_id);
        Ok(Some((
            parent_scope_id,
            destack_dir::NamespaceExport {
                module_id: target_module,
                kind,
                item: item_id,
            },
        )))
    }

    /// Collect namespace exports for a module or module binding target.
    pub(super) fn collect_namespace_exports_for_target(
        &self,
        revision: destack_workspace::Revision,
        origin_module_id: ModuleId,
        target: ModuleTarget,
        profile: ProfileId,
        mut cache: Option<&mut ResolveDependencyItemCache>,
    ) -> ResolveResult<Vec<(ModuleId, destack_dir::NamespaceExport)>> {
        let cache_key = cache
            .as_ref()
            .map(|_| self.cache_target_key(origin_module_id, target));
        if let (Some(cache), Some(cache_key)) = (cache.as_deref(), cache_key)
            && let Some(cached) = cache.namespace_exports.get(&cache_key)
        {
            return Ok(cached.clone());
        }

        match target {
            ModuleTarget::Module(module_id) => {
                // ensure the target module is prepared
                self.require_dir_prepared_if_other(revision, origin_module_id, module_id, profile)?;

                // load namespace exports from the module scope
                let module = self
                    .cache_module_snapshot(revision, module_id)
                    .map_err(|error| ResolveError::Internal {
                        message: format!("failed to load module snapshot: {error}"),
                    })?;
                let dir = self
                    .require_artifact_dir_prepared(revision, module_id, profile)
                    .map_err(ResolveError::from)?;
                let exports = self.collect_namespace_exports_for_scope(
                    revision,
                    &module,
                    profile,
                    dir.namespace_scope,
                    cache.as_deref_mut(),
                    &dir,
                )?;

                // attach the module id to each namespace export
                let exports: Vec<(ModuleId, destack_dir::NamespaceExport)> = exports
                    .into_iter()
                    .map(|export| (module_id, export))
                    .collect();
                if let (Some(cache), Some(cache_key)) = (cache.as_deref_mut(), cache_key) {
                    cache.namespace_exports.insert(cache_key, exports.clone());
                }
                Ok(exports)
            }
            ModuleTarget::External(_) => Ok(Vec::new()),
            ModuleTarget::Binding(specifier) => {
                // load bindings for the specifier
                let bindings = self.module_bindings_for_specifier(
                    revision,
                    origin_module_id,
                    profile,
                    specifier,
                )?;
                let Some(bindings) = bindings else {
                    return Ok(Vec::new());
                };

                // collect namespace exports from each binding scope
                let mut exports = Vec::new();
                for binding_ref in bindings {
                    // ensure the binding module is prepared
                    self.require_dir_prepared_if_other(
                        revision,
                        origin_module_id,
                        binding_ref.module_id,
                        profile,
                    )?;

                    // load the binding scope
                    let module = self
                        .cache_module_snapshot(revision, binding_ref.module_id)
                        .map_err(|error| ResolveError::Internal {
                            message: format!("failed to load module snapshot: {error}"),
                        })?;
                    let dir = self
                        .require_artifact_dir_prepared(revision, binding_ref.module_id, profile)
                        .map_err(ResolveError::from)?;
                    let Some((scope_id, _, _)) =
                        self.binding_info_for_declaration(&dir, binding_ref.declaration)
                    else {
                        continue;
                    };

                    // collect namespace exports from the binding scope
                    let nested = self.collect_namespace_exports_for_scope(
                        revision,
                        &module,
                        profile,
                        scope_id,
                        cache.as_deref_mut(),
                        &dir,
                    )?;
                    exports.extend(
                        nested
                            .into_iter()
                            .map(|export| (binding_ref.module_id, export)),
                    );
                }

                if let (Some(cache), Some(cache_key)) = (cache, cache_key) {
                    cache.namespace_exports.insert(cache_key, exports.clone());
                }
                Ok(exports)
            }
        }
    }

    /// Resolve a dependency item's parent expression target module when available.
    pub(super) fn parent_expression_target_module_for_dependency_item(
        &self,
        tree: &Tree,
        item_id: LocalNodeId<DependencyItem>,
    ) -> Option<ModuleTarget> {
        // locate the parent expression node
        let parent_id = tree.get_parent(item_id.id)?;
        let parent_id = parent_id.try_into_typed::<Expression>().ok()?;

        // read the resolved target from import or re-export expressions
        match tree.get(parent_id) {
            Expression::Import { target_module, .. }
            | Expression::ReExport { target_module, .. } => Some(*target_module),
            _ => None,
        }
    }

    /// Resolve a loader override from the dependency item's parent expression.
    pub(super) fn parent_expression_loader_override_for_dependency_item(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &Tree,
        item_id: LocalNodeId<DependencyItem>,
    ) -> ResolveResult<Option<destack_artifact::Loader>> {
        // locate the parent expression node
        let Some(parent_id) = tree.get_parent(item_id.id) else {
            return Ok(None);
        };
        let Ok(parent_id) = parent_id.try_into_typed::<Expression>() else {
            return Ok(None);
        };

        // read import attributes from the parent import or re-export expression
        let arguments = match tree.get(parent_id) {
            Expression::UnresolvedImport { attributes, .. }
            | Expression::UnresolvedReExport { attributes, .. }
            | Expression::Import { attributes, .. }
            | Expression::ReExport { attributes, .. } => {
                attributes.as_ref().map(|attributes| &attributes.attributes)
            }
            _ => return Ok(None),
        };

        // parse the optional loader override
        match self.loader_from_import_attributes(arguments.map(|arguments| arguments.as_slice())) {
            LoaderAttribute::None => Ok(None),
            LoaderAttribute::Loader(loader) => Ok(Some(loader)),
            LoaderAttribute::InvalidType { value } => {
                Err(ResolveError::InvalidImportAttributeType {
                    node: item_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                    value,
                })
            }
        }
    }

    /// Resolve a symbol through namespace exports of a module.
    /// This is used when a symbol isn't found in the direct namespace scope.
    pub(super) fn resolve_symbol_via_namespace_exports(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        node: GlobalNodeIdAny,
        via_target: ModuleTarget,
        profile: ProfileId,
        kind: DependencyKind,
        key: StaticKey,
        origin_symbol: Option<GlobalSymbolId>,
        default_name: StringId,
        mut cache: Option<&mut ResolveDependencyItemCache>,
    ) -> ResolveResult<ResolvedExportSymbol> {
        let cache_key = cache.as_ref().map(|_| NamespaceExportSymbolCacheKey {
            target: via_target,
            origin_module_id: self.cache_origin_module_id(module.id, via_target),
            kind,
            key,
        });
        if let (Some(cache), Some(cache_key)) = (cache.as_deref(), cache_key)
            && let Some(&(symbol, export_space)) = cache.namespace_export_symbols.get(&cache_key)
        {
            return Ok(ResolvedExportSymbol {
                symbol,
                export_space,
            });
        }

        // resolve the scope for missing symbol errors
        let (via_scope, via_module_id) =
            self.export_chain_scope_for_target(revision, module.id, via_target, profile)?;

        // collect namespace exports from export * statements
        let namespace_exports = self.collect_namespace_exports_for_target(
            revision,
            module.id,
            via_target,
            profile,
            cache.as_deref_mut(),
        )?;
        if namespace_exports.is_empty() {
            return Err(ResolveError::MissingSymbol {
                node: node.into_anchored(Some(profile)),
                scope: via_scope,
                via_module: via_module_id,
                key,
            });
        }

        // seed the namespace export queue
        let mut found: Option<(ResolvedExportSymbol, GlobalNodeIdAny)> = None;
        let mut visited = Vec::new();
        let mut queue = VecDeque::new();

        // enqueue top level namespace exports
        for (source_module_id, export) in namespace_exports {
            if self.namespace_export_allows_kind(export.kind, kind) {
                queue.push_back((source_module_id, export.module_id, export.item));
            }
        }

        // walk the namespace export graph
        while let Some((source_module_id, namespace_target, origin_item)) = queue.pop_front() {
            // skip cycles on namespace exports
            if visited.contains(&(source_module_id, namespace_target, origin_item)) {
                continue;
            }
            visited.push((source_module_id, namespace_target, origin_item));

            // resolve explicit exports within the namespace target
            let mut visited_exports = ReexportVisitStack::default();
            if let Some(resolved) = self.resolve_reexport_chain_symbol(
                revision,
                module.id,
                origin_symbol,
                node,
                namespace_target,
                profile,
                kind,
                key,
                default_name,
                &mut visited_exports,
                cache.as_deref_mut(),
            )? {
                match found {
                    None => {
                        found = Some((resolved, origin_item.into_global_any(source_module_id)));
                    }
                    Some((existing, other_node)) => {
                        if existing.symbol != resolved.symbol {
                            let node = origin_item.into_global_any(source_module_id);
                            self.check_can_merge_declarations(
                                revision,
                                existing.symbol,
                                resolved.symbol,
                                node,
                                other_node,
                                source_module_id,
                                Some(key),
                            );
                            return Ok(existing);
                        }
                    }
                }
            }

            // enqueue nested namespace exports
            let nested_exports = self.collect_namespace_exports_for_target(
                revision,
                module.id,
                namespace_target,
                profile,
                cache.as_deref_mut(),
            )?;
            for (nested_source_module_id, nested) in nested_exports {
                if self.namespace_export_allows_kind(nested.kind, kind) {
                    queue.push_back((nested_source_module_id, nested.module_id, nested.item));
                }
            }
        }

        // return the resolved symbol if any
        if let Some((resolved, _)) = found {
            if let (Some(cache), Some(cache_key)) = (cache, cache_key) {
                cache
                    .namespace_export_symbols
                    .insert(cache_key, (resolved.symbol, resolved.export_space));
            }
            return Ok(resolved);
        }

        // report a missing symbol for namespace lookup failures
        Err(ResolveError::MissingSymbol {
            node: node.into_anchored(Some(profile)),
            scope: via_scope,
            via_module: via_module_id,
            key,
        })
    }

    /// Check if a namespace export participates in a lookup kind.
    fn namespace_export_allows_kind(
        &self,
        export_kind: DependencyKind,
        lookup_kind: DependencyKind,
    ) -> bool {
        // type lookups accept both type and value exports
        if lookup_kind == DependencyKind::Type {
            return true;
        }

        // value lookups can still resolve type-only exports as type symbols
        matches!(export_kind, DependencyKind::Type | DependencyKind::Value)
    }
}
