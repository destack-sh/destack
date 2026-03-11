use crate::resolve::binding::cache::ResolveExpressionCache;
use crate::resolve::dependency::cache::ResolveDependencyItemCache;
use crate::timing::tags;
use crate::{BuildRequirementCollector, Compiler, ResolveError, ResolveResult};
use destack_dir::{
    Declaration, DependencyItem, DependencyKind, Expression, GlobalSymbolId, LocalNodeId,
    LocalScopeId, NodeTree, SymbolSpace,
};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::ProfileId;
use rustc_hash::FxHashMap;

/// Collect node ids needed for resolve passes.
struct ResolveModuleWorklist {
    /// Expressions to resolve after dependency items are applied.
    resolve_expression_ids: Vec<LocalNodeId<Expression>>,
    /// Expressions that define dependency items.
    dependency_expression_ids: Vec<LocalNodeId<Expression>>,
    /// Declarations that require resolve passes.
    declaration_ids: Vec<LocalNodeId<Declaration>>,
    /// Dependency items grouped by declaring scope.
    dependency_items_by_scope: FxHashMap<LocalScopeId, Vec<LocalNodeId<DependencyItem>>>,
}

impl ResolveModuleWorklist {
    /// Build resolve worklists from the module tree.
    fn from_tree(tree: &NodeTree) -> Self {
        // collect declarations that need resolve passes
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

        // collect expressions for dependency and resolution passes
        let mut resolve_expression_ids = Vec::new();
        let mut dependency_expression_ids = Vec::new();
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            let expression = tree.get(expression_id);
            if matches!(
                expression,
                Expression::UnresolvedImport { .. } | Expression::UnresolvedReExport { .. }
            ) {
                dependency_expression_ids.push(expression_id);
            }
            if matches!(
                expression,
                Expression::UnresolvedImport { .. }
                    | Expression::UnresolvedReExport { .. }
                    | Expression::UnresolvedPath { .. }
                    | Expression::UnresolvedBreak { .. }
                    | Expression::UnresolvedContinue { .. }
            ) {
                resolve_expression_ids.push(expression_id);
            }
        }

        // group dependency items by scope for export resolution
        let mut dependency_items_by_scope: FxHashMap<
            LocalScopeId,
            Vec<LocalNodeId<DependencyItem>>,
        > = FxHashMap::default();
        for item_id in tree.iter_node_ids_of_type::<DependencyItem>() {
            let (item_scope, _) = tree.get_scope(item_id);
            dependency_items_by_scope
                .entry(item_scope)
                .or_default()
                .push(item_id);
        }

        Self {
            resolve_expression_ids,
            dependency_expression_ids,
            declaration_ids,
            dependency_items_by_scope,
        }
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve expressions, dependencies, and declarations (phase 1).
    pub(crate) fn resolve_module_direct(
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

        self.require_dir_prepared(module_id, profile)?;
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        // load module data for read
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);
        let is_selected_lib_module = self.is_selected_lib_module(profile, module.id);
        let is_standard_lib_environment_module = self.is_standard_lib_environment_module(module.id);
        let skip_builtin_declaration_expressions = module.language_type.is_declaration()
            && module.is_builtin()
            && !self.options.validate_builtin_libs
            && (!is_selected_lib_module || !is_standard_lib_environment_module);
        let skip_builtin_global_symbol_table = module.language_type.is_declaration()
            && module.is_builtin()
            && !self.options.validate_builtin_libs;
        let worklist = {
            let tree = dir.tree.read();
            ResolveModuleWorklist::from_tree(&tree)
        };
        let mut expression_cache = ResolveExpressionCache::default();

        {
            let _timing = self.timing_scope(tags::RESOLVE_MODULE_DEPENDENCIES);

            // resolve module dependency expressions
            {
                if !skip_builtin_declaration_expressions {
                    let mut tree = dir.tree.write();
                    let symbols = dir.symbols.read();
                    let mut collector = BuildRequirementCollector::new();
                    for expression_id in &worklist.dependency_expression_ids {
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
                    if let Some(requirement) = collector.try_into_requirement() {
                        return Err(ResolveError::Yield { requirement });
                    }
                }
            }

            // resolve dependencies
            self.resolve_dependency_items(module_id, profile)?;

            // build the global symbol table
            if !skip_builtin_global_symbol_table {
                self.require_global_symbol_table(module.id, profile)?;
            }
        }

        {
            let _timing = self.timing_scope(tags::RESOLVE_MODULE_EXPRESSIONS);

            // resolve expressions
            if !skip_builtin_declaration_expressions {
                let mut tree = dir.tree.write();
                let symbols = dir.symbols.read();
                let mut collector = BuildRequirementCollector::new();
                for expression_id in &worklist.resolve_expression_ids {
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

                if let Some(requirement) = collector.try_into_requirement() {
                    return Err(ResolveError::Yield { requirement });
                }
            }
        }

        {
            let _timing = self.timing_scope(tags::RESOLVE_MODULE_DECLARATIONS);

            // resolve declarations
            let mut tree = dir.tree.write();
            let mut symbols = dir.symbols.write();
            let mut collector = BuildRequirementCollector::new();
            for declaration_id in &worklist.declaration_ids {
                if !self.is_node_active(&tree, &symbols, (*declaration_id).into_any()) {
                    continue;
                }

                self.collect(
                    &mut collector,
                    self.resolve_declaration(
                        &module,
                        dir,
                        profile,
                        *declaration_id,
                        &mut tree,
                        &mut symbols,
                    ),
                );
            }

            if let Some(requirement) = collector.try_into_requirement() {
                return Err(ResolveError::Yield { requirement });
            }
        }

        {
            let _timing = self.timing_scope(tags::RESOLVE_MODULE_EXPORTS);

            // finalize export targets (after dependency resolution)
            let tree = dir.tree.read();
            let mut symbols = dir.symbols.write();
            self.finalize_module_exports(&module, profile, dir, &tree, &mut symbols);
            self.finalize_module_binding_exports(
                &module,
                profile,
                dir,
                &tree,
                &mut symbols,
                &worklist.dependency_items_by_scope,
            );
        }

        Ok(())
    }

    /// Resolve dependency items (imports/reexports) for a module.
    pub(crate) fn resolve_dependency_items(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> ResolveResult<()> {
        let mut cache = ResolveDependencyItemCache::default();
        self.resolve_dependency_items_with_cache(module_id, profile, &mut cache)
    }

    /// Resolve dependency items using a shared cache across modules.
    pub(crate) fn resolve_dependency_items_with_cache(
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
        let mut collector = BuildRequirementCollector::new();
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
                Err(error @ ResolveError::UnresolvedModule { .. }) => {
                    self.handle_unresolved_module(error);
                    continue;
                }
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
        if let Some(requirement) = collector.try_into_requirement() {
            return Err(ResolveError::Yield { requirement });
        }

        Ok(())
    }
}
