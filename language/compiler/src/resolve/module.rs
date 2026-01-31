use std::collections::HashSet;

use crate::resolve::cache::{ResolveDependencyItemCache, ResolveExpressionCache};
use crate::timing::tags;
use crate::{Compiler, ImportError, ResolveError, ResolveResult, TaskResultCollector};
use destack_builtin::builtin_lib;
use destack_dir::{
    Declaration, DependencyItem, DependencyKind, DependencyMode, Export, ExportKind, Expression,
    GlobalNodeIdAny, GlobalSymbolId, LocalNodeId, LocalScopeId, LocalSymbolId, ModuleBinding,
    ModuleBindingExports, ModuleTarget, NodeTree, StaticKey, SymbolSpace, SymbolSpaceOrder,
    SymbolTable, SymbolType,
};

use destack_source::{CacheKind, ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{
    ImportMeta, Module, ModuleContent, ModuleDir, ModuleGraph, ModuleGraphKey, ProfileId,
};
use indexmap::IndexMap;
use rustc_hash::FxHashMap;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Prepare the per profile DIR by cloning from the base DIR.
    pub(super) fn resolve_module_prepare(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
    ) -> ResolveResult<()> {
        // skip stale tasks
        self.ensure_module_profile_matches::<ResolveError>(
            module_id,
            module_version,
            profile_id,
            profile_version,
        )?;
        let _timing = self.timing_scope(tags::RESOLVE_MODULE_PREPARE);

        self.require_import_module_validate(module_id)?;

        // data/text/binary modules have simpler preparation
        if !self.is_code_module(module_id) {
            return self.resolve_data_module_prepare(
                module_id,
                profile_id,
                module_version,
                profile_version,
            );
        }

        // resolve libs if needed
        if self.options.load_libs {
            let module = self.program.modules.get(module_id);
            let module = module.read();
            if module.is_user() {
                self.require_resolve_libs(profile_id)?;
            }
        }

        // resolve cache handle
        let cache_handle =
            self.cache_handle_for_module(module_id, Some(profile_id), None, CacheKind::DirResolved);

        // load the module and skip when the profile dir already exists
        self.ensure_module_profile_matches::<ResolveError>(
            module_id,
            module_version,
            profile_id,
            profile_version,
        )?;
        let module = self.program.modules.get(module_id);
        let mut module = module.write();
        self.ensure_module_profile_matches_guard::<ResolveError>(
            &module,
            module_version,
            profile_id,
            profile_version,
        )?;
        if module
            .code()
            .dirs
            .iter()
            .any(|dir| dir.profile_id == Some(profile_id))
        {
            return Ok(());
        }

        // try to load profile DIR from cache
        if let Some(cache) = cache_handle.as_ref()
            && let Ok(Some(entry)) = cache.read_dir_resolved()
        {
            self.ensure_module_profile_matches::<ResolveError>(
                module_id,
                module_version,
                profile_id,
                profile_version,
            )?;
            let dir = ModuleDir::from_data(entry.payload);
            module.code_mut().dirs.push(dir);
            tracing::trace!(?module_id, ?profile_id, "resolve.module.prepare.cache");
            return Ok(());
        }

        // load the base dir and profile
        let base = module.dir_base();
        let profile = self.program.profile(profile_id);
        let path = module.path.clone();
        let dir = module
            .path
            .as_ref()
            .and_then(|path| path.parent().map(|parent| parent.to_path_buf()));

        // build import meta
        let import_meta = ImportMeta {
            url: module.uri.clone(),
            path: path.clone(),
            file: path.clone(),
            filename: path.clone(),
            dir: dir.clone(),
            dirname: dir.clone(),
            output: profile.key.output,
            platform: profile.key.platform,
            runtime: profile.key.runtime,
            debug: profile.key.debug,
            test: profile.key.test,
            env: profile.env.clone(),
        };

        // create the profile dir and attach import meta
        let mut dir = ModuleDir::from_base(base, profile_id);
        dir.import_meta = Some(import_meta);
        module.code_mut().dirs.push(dir);

        // apply static if decorators before exports are built
        {
            let _timing = self.timing_scope(tags::RESOLVE_MODULE_PREPARE_STATIC_IF);
            let dir = module.dir_mut(profile_id);
            self.apply_static_if_decorators(module_id, profile_id, dir)?;
        }

        // build export table from bound declarations
        let dir = module.dir(profile_id);
        let tree = dir.tree.read();
        let mut symbols = dir.symbols.write();
        let dependency_items_by_scope = self.dependency_items_by_scope(&tree);
        {
            let _timing = self.timing_scope(tags::RESOLVE_MODULE_PREPARE_EXPORTS);
            self.build_module_exports(
                &module,
                dir,
                &tree,
                &mut symbols,
                &dependency_items_by_scope,
            );
        }
        {
            let _timing = self.timing_scope(tags::RESOLVE_MODULE_PREPARE_BINDING_EXPORTS);
            self.build_module_binding_exports(
                &module,
                dir,
                &tree,
                &mut symbols,
                &dependency_items_by_scope,
            );
        }

        // write profile DIR to cache
        if let Some(cache) = cache_handle.as_ref() {
            let _timing = self.timing_scope(tags::RESOLVE_MODULE_PREPARE_CACHE_WRITE);
            let payload = module.dir(profile_id).to_data();
            if let Err(error) = cache.write_dir_resolved(payload) {
                tracing::debug!(
                    ?module_id,
                    ?profile_id,
                    ?error,
                    "resolve.module.cache.write"
                );
            }
        }

        Ok(())
    }

    /// Update the symbol id stored in a dependency item.
    fn retype_dependency_item_symbol(
        &self,
        item: DependencyItem,
        symbol_id: LocalSymbolId,
    ) -> DependencyItem {
        // rewrite the stored symbol id when present
        match item {
            DependencyItem::UnresolvedRemote {
                source,
                mode,
                kind,
                name,
                alias,
                target,
                target_module,
                symbol: _,
            } => DependencyItem::UnresolvedRemote {
                source,
                mode,
                kind,
                name,
                alias,
                target,
                target_module,
                symbol: Some(symbol_id),
            },
            DependencyItem::UnresolvedLocal {
                mode,
                kind,
                name,
                alias,
                symbol: _,
            } => DependencyItem::UnresolvedLocal {
                mode,
                kind,
                name,
                alias,
                symbol: Some(symbol_id),
            },
            DependencyItem::Local {
                mode,
                kind,
                name,
                alias,
                symbol: _,
                target_symbol,
            } => DependencyItem::Local {
                mode,
                kind,
                name,
                alias,
                symbol: Some(symbol_id),
                target_symbol,
            },
            DependencyItem::Remote {
                mode,
                kind,
                name,
                alias,
                target,
                target_module,
                symbol: _,
                target_symbol,
            } => DependencyItem::Remote {
                mode,
                kind,
                name,
                alias,
                target,
                target_module,
                symbol: Some(symbol_id),
                target_symbol,
            },
            item => item,
        }
    }

    /// Update the target symbol stored in a dependency item.
    fn retype_dependency_item_target(
        &self,
        item: DependencyItem,
        target_symbol: GlobalSymbolId,
    ) -> DependencyItem {
        // rewrite the stored target symbol when present
        match item {
            DependencyItem::Local {
                mode,
                kind,
                name,
                alias,
                symbol,
                target_symbol: _,
            } => DependencyItem::Local {
                mode,
                kind,
                name,
                alias,
                symbol,
                target_symbol,
            },
            DependencyItem::Remote {
                mode,
                kind,
                name,
                alias,
                target,
                target_module,
                symbol,
                target_symbol: _,
            } => DependencyItem::Remote {
                mode,
                kind,
                name,
                alias,
                target,
                target_module,
                symbol,
                target_symbol,
            },
            item => item,
        }
    }

    /// Prepare profile DIR for data/text/binary modules.
    fn resolve_data_module_prepare(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
    ) -> ResolveResult<()> {
        // resolve cache handle
        let cache_handle =
            self.cache_handle_for_module(module_id, Some(profile_id), None, CacheKind::DirResolved);

        let module = self.program.modules.get(module_id);

        // skip stale tasks
        self.ensure_module_profile_matches::<ResolveError>(
            module_id,
            module_version,
            profile_id,
            profile_version,
        )?;
        let mut module = module.write();
        self.ensure_module_profile_matches_guard::<ResolveError>(
            &module,
            module_version,
            profile_id,
            profile_version,
        )?;

        // skip if profile DIR already exists
        if module.dir_maybe(profile_id).is_some() {
            return Ok(());
        }

        // try to load profile DIR from cache
        if let Some(cache) = cache_handle.as_ref()
            && let Ok(Some(entry)) = cache.read_dir_resolved()
        {
            self.ensure_module_profile_matches::<ResolveError>(
                module_id,
                module_version,
                profile_id,
                profile_version,
            )?;
            let dir = ModuleDir::from_data(entry.payload);
            match &mut module.content {
                ModuleContent::Data { dirs, .. } => dirs.push(dir),
                ModuleContent::Text { dirs, .. } => dirs.push(dir),
                ModuleContent::Binary { dirs, .. } => dirs.push(dir),
                ModuleContent::Code(_) | ModuleContent::Unloaded => {}
            }
            tracing::trace!(?module_id, ?profile_id, "resolve.module.prepare.cache");
            return Ok(());
        }

        // get base DIR (must exist for parsed data modules)
        let base = module
            .dir_base_maybe()
            .expect("data module missing base DIR");

        // create profile DIR from base
        let dir = ModuleDir::from_base(base, profile_id);

        // populate default export in exported_symbols
        let default_key_id = self.program.strings.intern("default");
        let default_key = destack_dir::StaticKey::Name(default_key_id);
        let default_export = destack_dir::Export::local(
            module_id,
            default_key,
            destack_dir::SymbolSpace::Value,
            dir.default_symbol,
        );
        dir.exported_symbols.write().insert(
            (destack_dir::SymbolSpace::Value, default_key),
            default_export,
        );

        // add DIR to module
        match &mut module.content {
            ModuleContent::Data { dirs, .. } => dirs.push(dir),
            ModuleContent::Text { dirs, .. } => dirs.push(dir),
            ModuleContent::Binary { dirs, .. } => dirs.push(dir),
            _ => {}
        }

        // write profile DIR to cache
        if let Some(cache) = cache_handle.as_ref() {
            let payload = module.dir(profile_id).to_data();
            if let Err(error) = cache.write_dir_resolved(payload) {
                tracing::debug!(?module_id, ?profile_id, ?error, "resolve.data.cache.write");
            }
        }

        Ok(())
    }

    /// Resolve expressions, dependencies, and declarations (phase 1).
    pub(super) fn resolve_module_direct(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
    ) -> ResolveResult<()> {
        // skip stale tasks
        self.ensure_module_profile_matches::<ResolveError>(
            module_id,
            module_version,
            profile,
            profile_version,
        )?;
        let _timing = self.timing_scope(tags::RESOLVE_MODULE_DIRECT);

        self.require_resolve_module_prepare(module_id, profile)?;
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        // load module data for read
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);
        let skip_declaration_expressions = module.language_type.is_declaration()
            && module.is_builtin()
            && !self.options.validate_builtin_libs;
        let (resolve_expression_ids, dependency_expression_ids, declaration_ids) = {
            let tree = dir.tree.read();
            // snapshot expression ids for resolve passes
            let expression_ids = tree.iter_node_ids_of_type::<Expression>();
            let mut declaration_ids = Vec::new();
            for declaration_id in tree.iter_node_ids_of_type::<Declaration>() {
                if matches!(
                    tree.get(declaration_id),
                    Declaration::Extension { .. }
                        | Declaration::Type { .. }
                        | Declaration::ImportAlias { .. }
                ) {
                    declaration_ids.push(declaration_id);
                }
            }
            let mut dependency_expression_ids = Vec::new();
            let mut resolve_expression_ids = Vec::new();
            for expression_id in &expression_ids {
                if matches!(
                    tree.get(*expression_id),
                    Expression::UnresolvedImport { .. } | Expression::UnresolvedReExport { .. }
                ) {
                    dependency_expression_ids.push(*expression_id);
                }
                if matches!(
                    tree.get(*expression_id),
                    Expression::UnresolvedImport { .. }
                        | Expression::UnresolvedReExport { .. }
                        | Expression::UnresolvedPath { .. }
                        | Expression::UnresolvedBreak { .. }
                        | Expression::UnresolvedContinue { .. }
                ) {
                    resolve_expression_ids.push(*expression_id);
                }
            }
            (
                resolve_expression_ids,
                dependency_expression_ids,
                declaration_ids,
            )
        };
        let mut expression_cache = ResolveExpressionCache::default();

        {
            let _timing = self.timing_scope(tags::RESOLVE_MODULE_DEPENDENCIES);

            // resolve module dependency expressions
            {
                if !skip_declaration_expressions {
                    let mut tree = dir.tree.write();
                    let symbols = dir.symbols.read();
                    let mut collector = TaskResultCollector::new();
                    for expression_id in &dependency_expression_ids {
                        if !self.is_node_active(&tree, &symbols, (*expression_id).into_any()) {
                            continue;
                        }
                        self.collect(
                            &mut collector,
                            self.resolve_expression(
                                &module,
                                dir,
                                profile,
                                *expression_id,
                                &mut tree,
                                &symbols,
                                &mut expression_cache,
                            ),
                        );
                    }
                    if let Some(dependency) = collector.try_into_yield_any() {
                        return Err(ResolveError::Yield { dependency });
                    }
                }
            }

            // resolve dependencies
            self.resolve_dependency_items(module_id, profile)?;

            // build the global symbol table (after dependency resolution)
            self.require_global_symbol_table(module.id, profile)?;
        }

        {
            let _timing = self.timing_scope(tags::RESOLVE_MODULE_EXPRESSIONS);

            // resolve expressions
            let mut tree = dir.tree.write();
            let symbols = dir.symbols.read();
            let mut collector = TaskResultCollector::new();
            for expression_id in &resolve_expression_ids {
                if !self.is_node_active(&tree, &symbols, (*expression_id).into_any()) {
                    continue;
                }
                self.collect(
                    &mut collector,
                    self.resolve_expression(
                        &module,
                        dir,
                        profile,
                        *expression_id,
                        &mut tree,
                        &symbols,
                        &mut expression_cache,
                    ),
                );
            }
            if let Some(dependency) = collector.try_into_yield_any() {
                return Err(ResolveError::Yield { dependency });
            }
        }

        {
            let _timing = self.timing_scope(tags::RESOLVE_MODULE_DECLARATIONS);

            // resolve declarations (e.g., extensions, types/aliases)
            let mut tree = dir.tree.write();
            let mut symbols = dir.symbols.write();
            let mut collector = TaskResultCollector::new();
            for declaration_id in &declaration_ids {
                if !self.is_node_active(&tree, &symbols, (*declaration_id).into_any()) {
                    continue;
                }
                self.collect(
                    &mut collector,
                    self.resolve_declaration(
                        &module,
                        dir,
                        *declaration_id,
                        &mut tree,
                        &mut symbols,
                    ),
                );
            }
            if let Some(dependency) = collector.try_into_yield_any() {
                return Err(ResolveError::Yield { dependency });
            }
        }

        let dependency_items_by_scope = {
            let tree = dir.tree.read();
            self.dependency_items_by_scope(&tree)
        };

        {
            let _timing = self.timing_scope(tags::RESOLVE_MODULE_EXPORTS);

            // finalize export targets (after dependency resolution)
            let tree = dir.tree.read();
            let mut symbols = dir.symbols.write();
            self.finalize_module_exports(dir, &tree, &mut symbols);
            self.finalize_module_binding_exports(
                dir,
                &tree,
                &mut symbols,
                &dependency_items_by_scope,
            );
        }

        Ok(())
    }

    /// Resolve dependency items (imports/reexports) for a module.
    pub(super) fn resolve_dependency_items(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> ResolveResult<()> {
        let mut cache = ResolveDependencyItemCache::default();
        self.resolve_dependency_items_with_cache(module_id, profile, &mut cache)
    }

    /// Resolve dependency items using a shared cache across modules.
    pub(super) fn resolve_dependency_items_with_cache(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        cache: &mut ResolveDependencyItemCache,
    ) -> ResolveResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);

        // cache module exports for dependency resolution
        cache.ensure_module_exports(module_id, dir);

        // collect dependency item ids once
        let item_ids = {
            let tree = dir.tree.read();
            cache.dependency_item_ids_for(module_id, &tree)
        };

        // resolve dependency items using read locks
        let mut collector = TaskResultCollector::new();
        let mut resolved_items = Vec::new();
        for item_id in item_ids {
            let resolved_item = {
                let tree = dir.tree.read();
                let symbols = dir.symbols.read();
                self.resolve_dependency_item(
                    &module,
                    dir,
                    profile,
                    item_id,
                    &tree,
                    &symbols,
                    Some(cache),
                )
            };

            // collect yields and return on non yield errors
            let resolved_item = match resolved_item {
                Ok(resolved_item) => resolved_item,
                Err(error) => {
                    if let Some(error) = collector.try_collect::<(), _>(Err(error)) {
                        return Err(error);
                    }
                    continue;
                }
            };

            if let Some(resolved_item) = resolved_item {
                let target_info = resolved_item.target_symbol().map(|target_symbol| {
                    let target_type = self.symbol_type_for_global(profile, target_symbol);
                    let typed_target_symbol = GlobalSymbolId::new(
                        target_symbol.module_id,
                        target_symbol.local_id.with_type(target_type),
                    );
                    let target_space = self.symbol_space_for_global(profile, typed_target_symbol);
                    (typed_target_symbol, target_type, target_space)
                });
                resolved_items.push((item_id, resolved_item, target_info));
            }
        }

        // apply resolved dependency updates
        if !resolved_items.is_empty() {
            let mut tree = dir.tree.write();
            let mut symbols = dir.symbols.write();
            for (item_id, resolved_item, target_info) in resolved_items {
                // align resolved target symbols with their declared types
                let mut resolved_item = resolved_item;
                if let Some((typed_target_symbol, target_type, target_space)) = target_info {
                    resolved_item =
                        self.retype_dependency_item_target(resolved_item, typed_target_symbol);

                    // align local bindings with resolved target types
                    if let Some(symbol_id) = resolved_item.symbol() {
                        let typed_symbol = symbols.retype_symbol_id(symbol_id, target_type);
                        resolved_item =
                            self.retype_dependency_item_symbol(resolved_item, typed_symbol);
                        symbols
                            .get_symbol_mut(typed_symbol)
                            .resolve_to(typed_target_symbol);

                        // adjust dependency kind and space for the resolved binding
                        let dependency_kind = match resolved_item {
                            DependencyItem::Local { kind, .. }
                            | DependencyItem::Remote { kind, .. }
                            | DependencyItem::UnresolvedLocal { kind, .. }
                            | DependencyItem::UnresolvedRemote { kind, .. } => Some(kind),
                            DependencyItem::Value { .. } => None,
                        };

                        if dependency_kind == Some(DependencyKind::Type)
                            || (dependency_kind == Some(DependencyKind::Value)
                                && target_space == SymbolSpace::Type)
                        {
                            symbols.get_symbol_mut(typed_symbol).space = SymbolSpace::Type;
                        }
                    }
                }

                *tree.get_mut(item_id) = resolved_item;
            }
        }

        // yield unresolved dependency items after updates
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(ResolveError::Yield { dependency });
        }

        Ok(())
    }

    /// Build export tables for module bindings in this module.
    fn build_module_binding_exports(
        &self,
        module: &Module,
        dir: &ModuleDir,
        tree: &NodeTree,
        symbols: &mut SymbolTable,
        dependency_items_by_scope: &FxHashMap<LocalScopeId, Vec<LocalNodeId<DependencyItem>>>,
    ) {
        // cache the default export name
        let default_name = self.program.strings.intern("default");

        // build the binding export table
        let bindings = dir.module_bindings.read().clone();
        let mut binding_exports = IndexMap::new();
        for binding in bindings {
            // collect the export assignment if present
            let dependency_items = dependency_items_by_scope
                .get(&binding.scope)
                .map(|items| items.as_slice())
                .unwrap_or_default();
            let export_assignment_item =
                self.collect_binding_export_assignment(module.id, &binding, tree, dependency_items);

            // insert exports declared by symbols
            let mut exports = IndexMap::new();
            self.insert_binding_symbol_exports(
                module.id,
                &binding,
                symbols,
                &mut exports,
                export_assignment_item,
                default_name,
            );

            // insert exports declared by dependency items
            self.insert_binding_dependency_exports(
                module.id,
                &binding,
                tree,
                symbols,
                &mut exports,
                export_assignment_item,
                default_name,
                dependency_items,
            );

            // insert ambient exports for remaining names
            self.insert_binding_ambient_exports(module.id, &binding, symbols, &mut exports);

            // record the binding exports
            binding_exports.insert(
                binding.declaration.into_any(),
                ModuleBindingExports {
                    exports,
                    export_assignment: export_assignment_item,
                },
            );
        }

        // store the binding export table
        *dir.module_binding_exports.write() = binding_exports;
    }

    /// Collect the export assignment item for a module binding, if present.
    fn collect_binding_export_assignment(
        &self,
        module_id: ModuleId,
        _binding: &ModuleBinding,
        tree: &NodeTree,
        dependency_items: &[LocalNodeId<DependencyItem>],
    ) -> Option<LocalNodeId<DependencyItem>> {
        // scan export statements in the binding scope
        let mut export_assignment_item: Option<LocalNodeId<DependencyItem>> = None;
        for item_id in dependency_items {
            // skip nonexport statements
            if self.export_statement_parent(tree, *item_id).is_none() {
                continue;
            }

            // skip nonassignment values
            let DependencyItem::Value { mode, .. } = tree.get(*item_id) else {
                continue;
            };
            if *mode != DependencyMode::Namespace {
                continue;
            }

            // report conflicts and keep the first assignment
            if let Some(existing) = export_assignment_item {
                self.error(ImportError::ConflictingExport {
                    node: (*item_id).into_global_any(module_id).into(),
                    other_node: existing.into_global_any(module_id).into(),
                    module: module_id,
                    name: None,
                });
                continue;
            }

            export_assignment_item = Some(*item_id);
        }

        export_assignment_item
    }

    /// Insert symbol exports for a module binding.
    fn insert_binding_symbol_exports(
        &self,
        module_id: ModuleId,
        binding: &ModuleBinding,
        symbols: &mut SymbolTable,
        exports: &mut IndexMap<(SymbolSpace, StaticKey), Export>,
        export_assignment_item: Option<LocalNodeId<DependencyItem>>,
        default_name: destack_base::StringId,
    ) {
        let mut seen_exports = HashSet::new();
        let scope = symbols.get_scope_by_id(binding.scope);
        let named_symbol_ids: Vec<LocalSymbolId> =
            scope.named_symbols.iter().map(|(_, id)| *id).collect();
        let anonymous_symbol_ids: Vec<LocalSymbolId> = scope.anonymous_symbols.to_vec();

        for symbol_id in named_symbol_ids {
            let symbol = symbols.get_symbol(symbol_id);
            if !symbol.is_active() {
                continue;
            }
            let Some(export_mode) = symbol.export else {
                continue;
            };

            // reject exports when export assignment is present
            if let Some(export_assignment_item) = export_assignment_item {
                let Some(other_node) = symbol.primary_declaration else {
                    continue;
                };
                self.error(ImportError::ConflictingExport {
                    node: export_assignment_item.into_global_any(module_id).into(),
                    other_node: other_node.into(),
                    module: module_id,
                    name: None,
                });
                continue;
            }

            // resolve the export key
            let key = match export_mode {
                DependencyMode::Default => StaticKey::Name(default_name),
                _ => {
                    let Some(name) = symbol.name() else {
                        continue;
                    };
                    StaticKey::Name(name)
                }
            };

            // insert export entries for each symbol space
            let spaces = match symbol.space {
                SymbolSpace::TypeValue => [Some(SymbolSpace::Type), Some(SymbolSpace::Value)],
                SymbolSpace::Type | SymbolSpace::Value => [Some(symbol.space), None],
                SymbolSpace::Label => [None, None],
            };
            for space in spaces.into_iter().flatten() {
                if !seen_exports.insert((space, key)) {
                    continue;
                }
                let export = Export::local(module_id, key, space, symbol_id);
                self.insert_exports(
                    module_id,
                    symbols,
                    exports,
                    export,
                    symbol.primary_declaration,
                );
            }

            // align the binding default symbol with default export declarations
            if export_mode == DependencyMode::Default
                && symbols
                    .get_symbol(binding.default_symbol)
                    .target_symbol
                    .is_none()
            {
                symbols
                    .get_symbol_mut(binding.default_symbol)
                    .resolve_to(symbol_id.into_global(module_id));
            }
        }

        for symbol_id in anonymous_symbol_ids {
            if symbol_id == binding.default_symbol {
                continue;
            }
            let symbol = symbols.get_symbol(symbol_id);
            if !symbol.is_active() {
                continue;
            }
            let Some(export_mode) = symbol.export else {
                continue;
            };

            // reject exports when export assignment is present
            if let Some(export_assignment_item) = export_assignment_item {
                let Some(other_node) = symbol.primary_declaration else {
                    continue;
                };
                self.error(ImportError::ConflictingExport {
                    node: export_assignment_item.into_global_any(module_id).into(),
                    other_node: other_node.into(),
                    module: module_id,
                    name: None,
                });
                continue;
            }

            // resolve the export key
            let key = match export_mode {
                DependencyMode::Default => StaticKey::Name(default_name),
                _ => {
                    let Some(name) = symbol.name() else {
                        continue;
                    };
                    StaticKey::Name(name)
                }
            };

            // insert export entries for each symbol space
            let spaces = match symbol.space {
                SymbolSpace::TypeValue => [Some(SymbolSpace::Type), Some(SymbolSpace::Value)],
                SymbolSpace::Type | SymbolSpace::Value => [Some(symbol.space), None],
                SymbolSpace::Label => [None, None],
            };
            for space in spaces.into_iter().flatten() {
                if !seen_exports.insert((space, key)) {
                    continue;
                }
                let export = Export::local(module_id, key, space, symbol_id);
                self.insert_exports(
                    module_id,
                    symbols,
                    exports,
                    export,
                    symbol.primary_declaration,
                );
            }

            // align the binding default symbol with default export declarations
            if export_mode == DependencyMode::Default
                && symbols
                    .get_symbol(binding.default_symbol)
                    .target_symbol
                    .is_none()
            {
                symbols
                    .get_symbol_mut(binding.default_symbol)
                    .resolve_to(symbol_id.into_global(module_id));
            }
        }
    }

    /// Insert dependency exports for a module binding.
    fn insert_binding_dependency_exports(
        &self,
        module_id: ModuleId,
        binding: &ModuleBinding,
        tree: &NodeTree,
        symbols: &SymbolTable,
        exports: &mut IndexMap<(SymbolSpace, StaticKey), Export>,
        export_assignment_item: Option<LocalNodeId<DependencyItem>>,
        default_name: destack_base::StringId,
        dependency_items: &[LocalNodeId<DependencyItem>],
    ) {
        // walk dependency items under export expressions in the binding scope
        for item_id in dependency_items {
            // skip nonexport dependency items
            if self.export_item_parent(tree, *item_id).is_none() {
                continue;
            }

            // reject exports when export assignment is present
            if let Some(export_assignment_item) = export_assignment_item
                && export_assignment_item != *item_id
            {
                self.error(ImportError::ConflictingExport {
                    node: (*item_id).into_global_any(module_id).into(),
                    other_node: export_assignment_item.into_global_any(module_id).into(),
                    module: module_id,
                    name: None,
                });
                continue;
            }

            // extract export metadata from the dependency item
            let item = tree.get(*item_id);
            let (mode, kind, name, alias) = match item {
                DependencyItem::Local {
                    mode,
                    kind,
                    name,
                    alias,
                    ..
                }
                | DependencyItem::Remote {
                    mode,
                    kind,
                    name,
                    alias,
                    ..
                }
                | DependencyItem::UnresolvedLocal {
                    mode,
                    kind,
                    name,
                    alias,
                    ..
                }
                | DependencyItem::UnresolvedRemote {
                    mode,
                    kind,
                    name,
                    alias,
                    ..
                } => (*mode, *kind, *name, *alias),
                DependencyItem::Value { mode, .. } => {
                    // emit default value exports
                    if *mode == DependencyMode::Default {
                        let key = StaticKey::Name(default_name);
                        let export = Export::local(
                            module_id,
                            key,
                            SymbolSpace::Value,
                            binding.default_symbol,
                        );
                        self.insert_exports(
                            module_id,
                            symbols,
                            exports,
                            export,
                            Some((*item_id).into_global_any(module_id)),
                        );
                    }
                    continue;
                }
            };

            // resolve the export key
            let export_name = self.export_name_for_dependency(mode, name, alias, default_name);
            let Some(export_name) = export_name else {
                continue;
            };
            let key = StaticKey::Name(export_name);

            // insert reexport entries
            let spaces = match kind {
                DependencyKind::Type => SymbolSpaceOrder::TypeOnly,
                DependencyKind::Value => SymbolSpaceOrder::ValueOnly,
            };
            for space in spaces.spaces() {
                let export = Export::reexport(key, *space, *item_id);
                self.insert_exports(
                    module_id,
                    symbols,
                    exports,
                    export,
                    Some((*item_id).into_global_any(module_id)),
                );
            }
        }
    }

    /// Insert ambient exports for a module binding scope.
    fn insert_binding_ambient_exports(
        &self,
        module_id: ModuleId,
        binding: &ModuleBinding,
        symbols: &SymbolTable,
        exports: &mut IndexMap<(SymbolSpace, StaticKey), Export>,
    ) {
        // collect namespace symbols
        let scope = symbols.get_scope_by_id(binding.scope);

        let symbol_ids: Vec<LocalSymbolId> = symbols
            .active_named_symbols(scope)
            .map(|(_, symbol_id)| symbol_id)
            .chain(
                symbols
                    .active_anonymous_symbols(scope)
                    .filter(|symbol_id| *symbol_id != binding.default_symbol),
            )
            .collect();

        // insert exports without overriding explicit entries
        for symbol_id in symbol_ids.iter().copied() {
            // skip synthetic export assignment symbol
            if symbol_id == binding.export_assignment_symbol {
                continue;
            }

            // skip anonymous symbols
            let symbol = symbols.get_symbol(symbol_id);
            let Some(name) = symbol.name() else {
                continue;
            };

            // determine export spaces (TypeValue expands to both Type and Value)
            let key = StaticKey::Name(name);
            let spaces: [Option<SymbolSpace>; 2] = match symbol.space {
                SymbolSpace::TypeValue => [Some(SymbolSpace::Type), Some(SymbolSpace::Value)],
                SymbolSpace::Type | SymbolSpace::Value => [Some(symbol.space), None],
                SymbolSpace::Label => [None, None],
            };

            // insert exports for each space if not already present
            for space in spaces.into_iter().flatten() {
                if !exports.contains_key(&(space, key)) {
                    exports.insert(
                        (space, key),
                        Export::local(module_id, key, space, symbol_id),
                    );
                }
            }
        }

        // export members from namespace scopes
        for symbol_id in symbol_ids.iter().copied() {
            // collect namespace symbols, including those in merge groups
            let mut namespace_symbols = Vec::new();
            let symbol = symbols.get_symbol(symbol_id);
            if symbol.kind == destack_dir::SymbolKind::Namespace {
                namespace_symbols.push(symbol_id);
            }

            // check merge group for additional namespace symbols (e.g., function+namespace merge)
            if let Some(merge_group) = symbol.merge_group {
                for &group_symbol_id in symbols.merge_group_symbols(merge_group) {
                    let group_symbol = symbols.get_symbol(group_symbol_id);
                    if group_symbol.kind == destack_dir::SymbolKind::Namespace {
                        namespace_symbols.push(group_symbol_id);
                    }
                }
            }

            // export members from each namespace scope
            for ns_symbol_id in namespace_symbols {
                let ns_symbol = symbols.get_symbol(ns_symbol_id);
                let namespace_scope = symbols.get_scope_by_id(ns_symbol.scope.0);

                for (key, symbol_id) in symbols.active_named_symbols(namespace_scope) {
                    let symbol = symbols.get_symbol(symbol_id);

                    // determine export spaces (TypeValue expands to both Type and Value)
                    let spaces: [Option<SymbolSpace>; 2] = match symbol.space {
                        SymbolSpace::TypeValue => {
                            [Some(SymbolSpace::Type), Some(SymbolSpace::Value)]
                        }
                        SymbolSpace::Type | SymbolSpace::Value => [Some(symbol.space), None],
                        SymbolSpace::Label => [None, None],
                    };

                    // insert exports for each space if not already present
                    for space in spaces.into_iter().flatten() {
                        if !exports.contains_key(&(space, key)) {
                            exports.insert(
                                (space, key),
                                Export::local(module_id, key, space, symbol_id),
                            );
                        }
                    }
                }
            }
        }
    }

    /// Group dependency items by their declaring scope.
    fn dependency_items_by_scope(
        &self,
        tree: &NodeTree,
    ) -> FxHashMap<LocalScopeId, Vec<LocalNodeId<DependencyItem>>> {
        // group dependency items by their declaring scope
        let mut items_by_scope: FxHashMap<LocalScopeId, Vec<LocalNodeId<DependencyItem>>> =
            FxHashMap::default();
        for item_id in tree.iter_node_ids_of_type::<DependencyItem>() {
            let (item_scope, _) = tree.get_scope(item_id);
            items_by_scope.entry(item_scope).or_default().push(item_id);
        }

        items_by_scope
    }

    /// Build the export table for a module.
    fn build_module_exports(
        &self,
        module: &Module,
        dir: &ModuleDir,
        tree: &NodeTree,
        symbols: &mut SymbolTable,
        dependency_items_by_scope: &FxHashMap<LocalScopeId, Vec<LocalNodeId<DependencyItem>>>,
    ) {
        // cache the default export name
        let default_name = self.program.strings.intern("default");

        // collect the export assignment if present
        let dependency_items = dependency_items_by_scope
            .get(&dir.namespace_scope)
            .map(|items| items.as_slice())
            .unwrap_or_default();
        let export_assignment_item =
            self.collect_export_assignment(module, dir, tree, dependency_items);

        // insert exports declared by symbols
        let mut exports = dir.exported_symbols.write();
        self.insert_symbol_exports(
            module.id,
            dir,
            symbols,
            &mut exports,
            export_assignment_item,
            default_name,
        );

        // insert exports declared by dependency items
        self.insert_dependency_exports(
            module.id,
            dir,
            tree,
            symbols,
            &mut exports,
            export_assignment_item,
            default_name,
            dependency_items,
        );

        // add ambient exports when a builtin lib is global
        if self.module_is_ambient_lib(module) {
            self.insert_ambient_exports(module.id, dir, symbols, &mut exports);
        }
    }

    /// Collect the export assignment item if present.
    fn collect_export_assignment(
        &self,
        module: &Module,
        dir: &ModuleDir,
        tree: &NodeTree,
        dependency_items: &[LocalNodeId<DependencyItem>],
    ) -> Option<LocalNodeId<DependencyItem>> {
        // scan export statements for export assignments
        let mut export_assignment_item: Option<LocalNodeId<DependencyItem>> = None;
        for item_id in dependency_items {
            // skip nonexport statements
            if self.export_statement_parent(tree, *item_id).is_none() {
                continue;
            }

            // skip nonassignment values
            let DependencyItem::Value { mode, .. } = tree.get(*item_id) else {
                continue;
            };
            if *mode != DependencyMode::Namespace {
                continue;
            }

            // report conflicts and keep the first assignment
            if let Some(existing) = export_assignment_item {
                self.error(ImportError::ConflictingExport {
                    node: (*item_id).into_global_any(module.id).into(),
                    other_node: existing.into_global_any(module.id).into(),
                    module: module.id,
                    name: None,
                });
                continue;
            }

            export_assignment_item = Some(*item_id);
        }

        // record the export assignment on the module dir
        *dir.export_assignment.write() = export_assignment_item;

        export_assignment_item
    }

    /// Insert exports declared by symbols.
    fn insert_symbol_exports(
        &self,
        module_id: ModuleId,
        dir: &ModuleDir,
        symbols: &mut SymbolTable,
        exports: &mut IndexMap<(SymbolSpace, StaticKey), Export>,
        export_assignment_item: Option<LocalNodeId<DependencyItem>>,
        default_name: destack_base::StringId,
    ) {
        // collect namespace symbols
        let namespace_scope = symbols.get_scope_by_id(dir.namespace_scope);
        let named_symbol_ids: Vec<LocalSymbolId> = namespace_scope
            .named_symbols
            .iter()
            .map(|(_, id)| *id)
            .collect();
        let anonymous_symbol_ids: Vec<LocalSymbolId> = namespace_scope.anonymous_symbols.to_vec();

        for symbol_id in named_symbol_ids {
            let symbol = symbols.get_symbol(symbol_id);
            if !symbol.is_active() {
                continue;
            }
            let Some(export_mode) = symbol.export else {
                continue;
            };

            // reject exports when export assignment is present
            if let Some(export_assignment_item) = export_assignment_item {
                let Some(other_node) = symbol.primary_declaration else {
                    continue;
                };
                self.error(ImportError::ConflictingExport {
                    node: export_assignment_item.into_global_any(module_id).into(),
                    other_node: other_node.into(),
                    module: module_id,
                    name: None,
                });
                continue;
            }

            // resolve the export key
            let key = match export_mode {
                DependencyMode::Default => StaticKey::Name(default_name),
                _ => {
                    let Some(name) = symbol.name() else {
                        continue;
                    };
                    StaticKey::Name(name)
                }
            };

            // insert the export entry
            let export = Export::local(module_id, key, symbol.space, symbol_id);
            self.insert_exports(
                module_id,
                symbols,
                exports,
                export,
                symbol.primary_declaration,
            );

            // align the module default symbol with default export declarations
            if export_mode == DependencyMode::Default
                && symbols
                    .get_symbol(dir.default_symbol)
                    .target_symbol
                    .is_none()
            {
                symbols
                    .get_symbol_mut(dir.default_symbol)
                    .resolve_to(symbol_id.into_global(module_id));
            }
        }

        for symbol_id in anonymous_symbol_ids {
            if symbol_id == dir.default_symbol {
                continue;
            }
            let symbol = symbols.get_symbol(symbol_id);
            if !symbol.is_active() {
                continue;
            }
            let Some(export_mode) = symbol.export else {
                continue;
            };

            // reject exports when export assignment is present
            if let Some(export_assignment_item) = export_assignment_item {
                let Some(other_node) = symbol.primary_declaration else {
                    continue;
                };
                self.error(ImportError::ConflictingExport {
                    node: export_assignment_item.into_global_any(module_id).into(),
                    other_node: other_node.into(),
                    module: module_id,
                    name: None,
                });
                continue;
            }

            // resolve the export key
            let key = match export_mode {
                DependencyMode::Default => StaticKey::Name(default_name),
                _ => {
                    let Some(name) = symbol.name() else {
                        continue;
                    };
                    StaticKey::Name(name)
                }
            };

            // insert the export entry
            let export = Export::local(module_id, key, symbol.space, symbol_id);
            self.insert_exports(
                module_id,
                symbols,
                exports,
                export,
                symbol.primary_declaration,
            );

            // align the module default symbol with default export declarations
            if export_mode == DependencyMode::Default
                && symbols
                    .get_symbol(dir.default_symbol)
                    .target_symbol
                    .is_none()
            {
                symbols
                    .get_symbol_mut(dir.default_symbol)
                    .resolve_to(symbol_id.into_global(module_id));
            }
        }
    }

    /// Insert exports declared by dependency items.
    fn insert_dependency_exports(
        &self,
        module_id: ModuleId,
        dir: &ModuleDir,
        tree: &NodeTree,
        symbols: &SymbolTable,
        exports: &mut IndexMap<(SymbolSpace, StaticKey), Export>,
        export_assignment_item: Option<LocalNodeId<DependencyItem>>,
        default_name: destack_base::StringId,
        dependency_items: &[LocalNodeId<DependencyItem>],
    ) {
        // walk dependency items under export expressions
        for item_id in dependency_items {
            // skip nonexport dependency items
            if self.export_item_parent(tree, *item_id).is_none() {
                continue;
            }

            // reject exports when export assignment is present
            if let Some(export_assignment_item) = export_assignment_item
                && export_assignment_item != *item_id
            {
                self.error(ImportError::ConflictingExport {
                    node: (*item_id).into_global_any(module_id).into(),
                    other_node: export_assignment_item.into_global_any(module_id).into(),
                    module: module_id,
                    name: None,
                });
                continue;
            }

            // extract export metadata from the dependency item
            let item = tree.get(*item_id);
            let (mode, kind, name, alias) = match item {
                DependencyItem::Local {
                    mode,
                    kind,
                    name,
                    alias,
                    ..
                }
                | DependencyItem::Remote {
                    mode,
                    kind,
                    name,
                    alias,
                    ..
                }
                | DependencyItem::UnresolvedLocal {
                    mode,
                    kind,
                    name,
                    alias,
                    ..
                }
                | DependencyItem::UnresolvedRemote {
                    mode,
                    kind,
                    name,
                    alias,
                    ..
                } => (*mode, *kind, *name, *alias),
                DependencyItem::Value { mode, .. } => {
                    // emit default value exports
                    if *mode == DependencyMode::Default {
                        let key = StaticKey::Name(default_name);
                        let export =
                            Export::local(module_id, key, SymbolSpace::Value, dir.default_symbol);
                        self.insert_exports(
                            module_id,
                            symbols,
                            exports,
                            export,
                            Some((*item_id).into_global_any(module_id)),
                        );
                    }
                    continue;
                }
            };

            // resolve the export key
            let export_name = self.export_name_for_dependency(mode, name, alias, default_name);
            let Some(export_name) = export_name else {
                continue;
            };
            let key = StaticKey::Name(export_name);

            // insert reexport entries
            let spaces = match kind {
                DependencyKind::Type => SymbolSpaceOrder::TypeOnly,
                DependencyKind::Value => SymbolSpaceOrder::ValueOnly,
            };
            for space in spaces.spaces() {
                let export = Export::reexport(key, *space, *item_id);
                self.insert_exports(
                    module_id,
                    symbols,
                    exports,
                    export,
                    Some((*item_id).into_global_any(module_id)),
                );
            }
        }
    }

    /// Finalize export targets after dependency resolution.
    fn finalize_module_exports(&self, dir: &ModuleDir, tree: &NodeTree, symbols: &mut SymbolTable) {
        // resolve export assignment target symbols
        if let Some(item_id) = *dir.export_assignment.read() {
            let DependencyItem::Value { mode, value } = tree.get(item_id) else {
                return;
            };
            if *mode == DependencyMode::Namespace {
                let value_expression = tree.get(*value);
                if let Some(target_symbol) = value_expression.target_symbol() {
                    symbols
                        .get_symbol_mut(dir.export_assignment_symbol)
                        .resolve_to(target_symbol);
                }
            }
        }

        // resolve the default export target from value expressions
        if symbols
            .get_symbol(dir.default_symbol)
            .target_symbol
            .is_none()
        {
            for item_id in tree.iter_node_ids_of_type::<DependencyItem>() {
                // skip nonexport statements
                if self.export_statement_parent(tree, item_id).is_none() {
                    continue;
                }

                // skip nondefault value exports
                let DependencyItem::Value { mode, value } = tree.get(item_id) else {
                    continue;
                };
                if *mode != DependencyMode::Default {
                    continue;
                }

                // resolve the target symbol for the default export
                let value_expression = tree.get(*value);
                if let Some(target_symbol) = value_expression.target_symbol() {
                    symbols
                        .get_symbol_mut(dir.default_symbol)
                        .resolve_to(target_symbol);
                    break;
                }
            }
        }

        // resolve export targets for reexports
        let mut exports = dir.exported_symbols.write();
        self.finalize_export_targets(tree, &mut exports);
    }

    /// Finalize export targets for module bindings after dependency resolution.
    fn finalize_module_binding_exports(
        &self,
        dir: &ModuleDir,
        tree: &NodeTree,
        symbols: &mut SymbolTable,
        dependency_items_by_scope: &FxHashMap<LocalScopeId, Vec<LocalNodeId<DependencyItem>>>,
    ) {
        // snapshot module bindings for export resolution
        let bindings = dir.module_bindings.read().clone();

        // load the binding exports table for updates
        let mut binding_exports = dir.module_binding_exports.write();

        // finalize exports for each module binding
        for binding in bindings {
            let binding_key = binding.declaration.into_any();

            // resolve export assignment target symbols
            if let Some(item_id) = binding_exports
                .get(&binding_key)
                .and_then(|binding_exports| binding_exports.export_assignment)
            {
                let DependencyItem::Value { mode, value } = tree.get(item_id) else {
                    continue;
                };
                if *mode == DependencyMode::Namespace {
                    let value_expression = tree.get(*value);
                    if let Some(target_symbol) = value_expression.target_symbol() {
                        symbols
                            .get_symbol_mut(binding.export_assignment_symbol)
                            .resolve_to(target_symbol);
                    }
                }
            }

            // resolve the default export target from value expressions
            if symbols
                .get_symbol(binding.default_symbol)
                .target_symbol
                .is_none()
            {
                let dependency_items = dependency_items_by_scope
                    .get(&binding.scope)
                    .map(|items| items.as_slice())
                    .unwrap_or_default();
                for item_id in dependency_items {
                    // skip nonexport statements
                    if self.export_statement_parent(tree, *item_id).is_none() {
                        continue;
                    }

                    // skip nondefault value exports
                    let DependencyItem::Value { mode, value } = tree.get(*item_id) else {
                        continue;
                    };
                    if *mode != DependencyMode::Default {
                        continue;
                    }

                    // resolve the target symbol for the default export
                    let value_expression = tree.get(*value);
                    if let Some(target_symbol) = value_expression.target_symbol() {
                        symbols
                            .get_symbol_mut(binding.default_symbol)
                            .resolve_to(target_symbol);
                        break;
                    }
                }
            }

            // resolve export targets for reexports
            if let Some(binding_exports) = binding_exports.get_mut(&binding_key) {
                self.finalize_export_targets(tree, &mut binding_exports.exports);
            }
        }
    }

    /// Resolve reexport targets after dependency resolution.
    fn finalize_export_targets(
        &self,
        tree: &NodeTree,
        exports: &mut IndexMap<(SymbolSpace, StaticKey), Export>,
    ) {
        // update unresolved reexports with resolved targets
        for export in exports.values_mut() {
            if export.target.resolved().is_some() {
                continue;
            }
            if export.kind != ExportKind::ReExport {
                continue;
            }
            let Some(item_id) = export.item else {
                continue;
            };
            let Some(target_symbol) = tree.get(item_id).target_symbol() else {
                continue;
            };
            export.resolve_target(target_symbol);
        }
    }

    /// Get the export statement parent for an item, if any.
    fn export_statement_parent(
        &self,
        tree: &NodeTree,
        item_id: LocalNodeId<DependencyItem>,
    ) -> Option<LocalNodeId<Expression>> {
        // resolve the parent expression
        let parent_id = tree.get_parent(item_id.id)?;
        let parent_id = parent_id.try_into_typed::<Expression>().ok()?;
        match tree.get(parent_id) {
            Expression::Export { .. } => Some(parent_id),
            _ => None,
        }
    }

    /// Get any export parent expression for an item, if any.
    pub(super) fn export_item_parent(
        &self,
        tree: &NodeTree,
        item_id: LocalNodeId<DependencyItem>,
    ) -> Option<LocalNodeId<Expression>> {
        // resolve the parent expression
        let parent_id = tree.get_parent(item_id.id)?;
        let parent_id = parent_id.try_into_typed::<Expression>().ok()?;
        match tree.get(parent_id) {
            Expression::Export { .. }
            | Expression::ReExport { .. }
            | Expression::UnresolvedReExport { .. } => Some(parent_id),
            _ => None,
        }
    }

    /// Check if this module should export all symbols from its namespace.
    pub(crate) fn module_is_ambient_lib(&self, module: &Module) -> bool {
        let Some(builtins) = self.program.builtins.as_ref() else {
            return false;
        };
        let Some(lib_name) = builtins.lib_name_for_module(module.id) else {
            return false;
        };
        let Some(lib) = builtin_lib(lib_name) else {
            return false;
        };
        lib.is_ambient
    }

    /// Insert ambient exports without overriding explicit export entries.
    fn insert_ambient_exports(
        &self,
        module_id: ModuleId,
        dir: &ModuleDir,
        symbols: &SymbolTable,
        exports: &mut IndexMap<(SymbolSpace, StaticKey), Export>,
    ) {
        // collect namespace symbols
        let scope = symbols.get_scope_by_id(dir.namespace_scope);
        let symbol_ids: Vec<LocalSymbolId> = symbols
            .active_named_symbols(scope)
            .map(|(_, symbol_id)| symbol_id)
            .chain(
                symbols
                    .active_anonymous_symbols(scope)
                    .filter(|symbol_id| *symbol_id != dir.default_symbol),
            )
            .collect();

        // insert exports without overriding explicit entries
        for symbol_id in symbol_ids {
            // skip synthetic export assignment symbol
            if symbol_id == dir.export_assignment_symbol {
                continue;
            }

            // skip anonymous symbols
            let symbol = symbols.get_symbol(symbol_id);
            let Some(name) = symbol.name() else {
                continue;
            };

            // insert an export if the slot is empty
            let key = StaticKey::Name(name);
            let mut insert_if_missing = |space: SymbolSpace| {
                if exports.contains_key(&(space, key)) {
                    return;
                }
                exports.insert(
                    (space, key),
                    Export::local(module_id, key, space, symbol_id),
                );
            };

            // expand type value entries into type and value exports
            match symbol.space {
                SymbolSpace::TypeValue => {
                    insert_if_missing(SymbolSpace::Type);
                    insert_if_missing(SymbolSpace::Value);
                }
                SymbolSpace::Type | SymbolSpace::Value => {
                    insert_if_missing(symbol.space);
                }
                SymbolSpace::Label => {}
            }
        }
    }

    /// Get the export name for a dependency item.
    fn export_name_for_dependency(
        &self,
        mode: DependencyMode,
        name: Option<destack_dir::Name>,
        alias: Option<destack_base::StringId>,
        default_name: destack_base::StringId,
    ) -> Option<destack_base::StringId> {
        // prefer explicit aliases
        if alias.is_some() {
            return alias;
        }

        // use mode defaults for unnamed exports
        match mode {
            DependencyMode::Item => name.map(|name| name.string()),
            DependencyMode::Default => name.map(|name| name.string()).or(Some(default_name)),
            DependencyMode::Namespace => None,
        }
    }

    /// Get the symbol space for a global symbol.
    pub(crate) fn symbol_space_for_global(
        &self,
        profile: ProfileId,
        symbol: GlobalSymbolId,
    ) -> SymbolSpace {
        let module = self.program.modules.get(symbol.module_id);
        let module = module.read();
        let symbols = module.dir(profile).symbols.read();
        symbols.get_symbol(symbol.local_id).space
    }

    /// Get the symbol type for a global symbol.
    pub(crate) fn symbol_type_for_global(
        &self,
        profile: ProfileId,
        symbol: GlobalSymbolId,
    ) -> SymbolType {
        let module = self.program.modules.get(symbol.module_id);
        let module = module.read();
        let symbols = module.dir(profile).symbols.read();
        symbols.get_symbol(symbol.local_id).ty
    }

    /// Check whether a symbol can be used as a value.
    ///
    /// This centralizes the type/value decision so Resolve can honor declaration-module reexports
    /// without silently treating nominal value symbols as type-only.
    pub(crate) fn symbol_is_value_capable(
        &self,
        profile: ProfileId,
        symbol: GlobalSymbolId,
    ) -> bool {
        // load the symbol entry
        let module = self.program.modules.get(symbol.module_id);
        let module = module.read();
        let symbols = module.dir(profile).symbols.read();
        let entry = symbols.get_symbol(symbol.local_id);

        // value-space symbols are always value-capable
        if entry.space == SymbolSpace::Value {
            return true;
        }

        // nominal declarations always introduce a value-capable symbol
        if matches!(
            entry.ty,
            SymbolType::Struct
                | SymbolType::Class
                | SymbolType::Enum
                | SymbolType::Newtype
                | SymbolType::Function
        ) {
            return true;
        }

        // type-value symbols only exclude type-only declarations
        if entry.space == SymbolSpace::TypeValue {
            return !matches!(
                entry.ty,
                SymbolType::Interface | SymbolType::TypeAlias | SymbolType::Extension
            );
        }

        // fall back to not value-capable
        false
    }

    /// Insert exports into the table and report conflicts.
    fn insert_exports(
        &self,
        module_id: ModuleId,
        symbols: &SymbolTable,
        exports: &mut IndexMap<(SymbolSpace, StaticKey), Export>,
        export: Export,
        node: Option<GlobalNodeIdAny>,
    ) {
        let key = export.key;
        let space = export.space;
        debug_assert!(space != SymbolSpace::Label);

        // expand type value exports into both type and value spaces
        if export.space == SymbolSpace::TypeValue {
            let mut type_export = export.clone();
            type_export.space = SymbolSpace::Type;
            self.insert_exports(module_id, symbols, exports, type_export, node);

            let mut value_export = export;
            value_export.space = SymbolSpace::Value;
            self.insert_exports(module_id, symbols, exports, value_export, node);
            return;
        }

        // insert when the export slot is empty
        let Some(existing) = exports.get(&(space, key)) else {
            exports.insert((space, key), export);
            return;
        };

        // resolve targets for conflict checks
        let existing_target = existing.target.resolved();
        let next_target = export.target.resolved();

        // keep resolved exports when the new target is unresolved
        if existing_target.is_some() && next_target.is_none() {
            return;
        }

        // replace unresolved exports with resolved ones
        if existing_target.is_none() && next_target.is_some() {
            exports.insert((space, key), export);
            return;
        }

        // keep the first unresolved export
        if existing_target.is_none() && next_target.is_none() {
            return;
        }

        // skip duplicates that resolve to the same target
        if existing_target == next_target {
            return;
        }

        // skip duplicates that are part of the same merge group
        if existing.kind == ExportKind::Local
            && export.kind == ExportKind::Local
            && let (Some(existing_symbol), Some(next_symbol)) = (existing.symbol, export.symbol)
        {
            let existing_merge = symbols.get_symbol(existing_symbol).merge_group;
            let next_merge = symbols.get_symbol(next_symbol).merge_group;
            if existing_merge.is_some() && existing_merge == next_merge {
                return;
            }
        }

        // report conflicts when the targets differ
        let other_node = match existing.kind {
            ExportKind::Local => existing
                .symbol
                .and_then(|symbol| symbols.get_symbol(symbol).primary_declaration),
            ExportKind::ReExport => existing.item.map(|item| item.into_global_any(module_id)),
        };
        if let (Some(node), Some(other_node)) = (node, other_node) {
            self.error(ImportError::ConflictingExport {
                node: node.into(),
                other_node: other_node.into(),
                module: module_id,
                name: Some(key),
            });
        }

        // insert the export entry
        exports.insert((space, key), export);
    }

    /// Compute canonical_symbol for all symbols (phase 2).
    pub(super) fn resolve_module_canonical(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
    ) -> ResolveResult<()> {
        // skip stale tasks
        self.ensure_module_profile_matches::<ResolveError>(
            module_id,
            module_version,
            profile,
            profile_version,
        )?;
        let _timing = self.timing_scope(tags::RESOLVE_MODULE_CANONICAL);

        self.require_resolve_module_direct(module_id, profile)?;
        if !self.is_code_module(module_id) {
            self.update_module_graph(module_id, profile, module_version, profile_version)?;
            return Ok(());
        }

        // load module data for canonical resolution
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();

        // collect symbols that have target_symbol but no canonical_symbol
        // (only include symbols with primary_declaration, others are internal or incomplete)
        let symbols_to_resolve: Vec<_> = symbols
            .active_symbol_ids()
            .filter_map(|id| {
                let symbol = symbols.get_symbol(id);
                if symbol.target_symbol.is_some()
                    && symbol.canonical_symbol.is_none()
                    && symbol.primary_declaration.is_some()
                {
                    Some((
                        id.into_global(module_id),
                        symbol.primary_declaration.unwrap(),
                    ))
                } else {
                    None
                }
            })
            .collect();

        // drop locks before resolving canonical symbols (may need to access other modules)
        drop(symbols);
        drop(tree);
        drop(module);

        // resolve canonical symbols
        // (this may yield for cross module resolution)
        let mut collector = TaskResultCollector::new();
        for (symbol_id, node) in symbols_to_resolve {
            self.collect(
                &mut collector,
                self.resolve_canonical_symbol(node, symbol_id, profile),
            );
        }

        // yield on any yield
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(ResolveError::Yield { dependency });
        }

        self.update_module_graph(module_id, profile, module_version, profile_version)?;
        Ok(())
    }

    /// Update the module graph for a resolved module.
    fn update_module_graph(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
    ) -> ResolveResult<()> {
        // skip stale tasks
        self.ensure_module_profile_matches::<ResolveError>(
            module_id,
            module_version,
            profile_id,
            profile_version,
        )?;

        // load module data for dependency discovery
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let Some(dir) = module.dir_maybe(profile_id) else {
            return Ok(());
        };

        // collect module dependency targets from imports and namespace exports
        let mut targets = Vec::new();
        for target in dir.imported_modules.read().values() {
            targets.push(*target);
        }
        for export in dir.namespace_exports.read().iter() {
            targets.push(export.module_id);
        }

        // drop module guard before resolving binding dependencies
        drop(module);

        // resolve module binding targets into module ids
        let mut dependencies = Vec::new();
        for target in targets {
            match target {
                ModuleTarget::Module(target_id) => {
                    dependencies.push(target_id);
                }
                ModuleTarget::Binding(specifier) => {
                    let bindings =
                        self.module_bindings_for_specifier(module_id, profile_id, specifier)?;
                    let Some(bindings) = bindings else {
                        continue;
                    };
                    for binding in bindings {
                        dependencies.push(binding.module_id);
                    }
                }
            }
        }

        // update the graph for this profile
        let key = ModuleGraphKey::new(profile_id);
        let mut entry = self
            .program
            .index
            .module_graphs
            .entry(key)
            .or_insert_with(|| ModuleGraph::new(profile_id));
        entry.update_module(module_id, module_version, dependencies);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::TestProgram;
    use destack_workspace::ModuleGraphKey;

    /// Build module graph edges for import dependencies.
    #[test]
    fn test_module_graph_import_dependency() {
        let test = TestProgram::memory_sequential();
        let dep_source = r#"
export const value = 1;
"#;
        let main_source = r#"
import { value } from "./dep.ts";

value;
"#;

        let dep_module_id = test.add_module("dep.ts", dep_source);
        let main_module_id = test.add_module("main.ts", main_source);

        test.resolve_module(main_module_id);
        test.compile_check_clean();

        let profile = test.default_profile_id(main_module_id);
        let key = ModuleGraphKey::new(profile);
        let graph = test
            .program
            .index
            .module_graphs
            .get(&key)
            .unwrap_or_else(|| panic!("missing module graph for profile {profile:?}"));
        let dependencies = graph.dependencies_for(main_module_id);

        // assert dependency edges
        assert!(
            dependencies.contains(&dep_module_id),
            "expected module graph to include dep.ts"
        );
    }

    /// Build module graph edges for namespace exports.
    #[test]
    fn test_module_graph_namespace_export_dependency() {
        let test = TestProgram::memory_sequential();
        let dep_source = r#"
export const value = 1;
"#;
        let export_source = r#"
export * from "./dep.ts";
"#;

        let dep_module_id = test.add_module("dep.ts", dep_source);
        let export_module_id = test.add_module("reexport.ts", export_source);

        test.resolve_module(export_module_id);
        test.compile_check_clean();

        let profile = test.default_profile_id(export_module_id);
        let key = ModuleGraphKey::new(profile);
        let graph = test
            .program
            .index
            .module_graphs
            .get(&key)
            .unwrap_or_else(|| panic!("missing module graph for profile {profile:?}"));
        let dependencies = graph.dependencies_for(export_module_id);

        // assert dependency edges
        assert!(
            dependencies.contains(&dep_module_id),
            "expected module graph to include dep.ts"
        );
    }

    /// Build module graph edges for module binding imports.
    #[test]
    fn test_module_graph_binding_dependency() {
        let test = TestProgram::memory_sequential();
        let decl_source = r#"
declare module "foo" {
    export const value: number;
}
"#;
        let main_source = r#"
import { value } from "foo";

value;
"#;

        let decl_module_id = test.add_module("decl.d.ts", decl_source);
        let main_module_id = test.add_module("main.ts", main_source);

        test.import_module(decl_module_id);
        test.compile_check_clean();

        test.resolve_module(main_module_id);
        test.compile_check_clean();

        let profile = test.default_profile_id(main_module_id);
        let key = ModuleGraphKey::new(profile);
        let graph = test
            .program
            .index
            .module_graphs
            .get(&key)
            .unwrap_or_else(|| panic!("missing module graph for profile {profile:?}"));
        let dependencies = graph.dependencies_for(main_module_id);

        // assert dependency edges
        assert!(
            dependencies.contains(&decl_module_id),
            "expected module graph to include module binding module"
        );
    }
}
