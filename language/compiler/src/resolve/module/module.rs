use crate::resolve::binding::cache::{ResolveExpressionCache, ResolveScopeIndexCache};
use crate::resolve::dependency::cache::ResolveDependencyItemCache;
use crate::timing::tags;
use crate::{Compiler, CompilerContext, RequirementCollector, ResolveError, ResolveResult};
use destack_artifact::{
    DirPrepared, ExportedSymbolTable, ImportedModuleTable, ModuleBindingExportTable,
};
use destack_dir::{
    Declaration, DependencyItem, DependencyKind, Expression, GlobalSymbolId, LocalNodeId,
    LocalScopeId, NamespaceExport, NodeTree, SymbolSpace, SymbolTable, TypeExpression, TypeTable,
};
use destack_workspace::{Module, ProfileId};
use rustc_hash::FxHashMap;

/// Collect node ids needed for resolve passes.
pub(crate) struct ResolveModuleWorklist {
    /// Expressions to resolve after dependency items are applied.
    pub(crate) resolve_expression_ids: Vec<LocalNodeId<Expression>>,
    /// Expressions that define dependency items.
    pub(crate) dependency_expression_ids: Vec<LocalNodeId<Expression>>,
    /// Type expressions to resolve after value-space paths settle.
    pub(crate) resolve_type_expression_ids: Vec<LocalNodeId<TypeExpression>>,
    /// Declarations that require resolve passes.
    pub(crate) declaration_ids: Vec<LocalNodeId<Declaration>>,
    /// Dependency items grouped by declaring scope.
    pub(crate) dependency_items_by_scope: FxHashMap<LocalScopeId, Vec<LocalNodeId<DependencyItem>>>,
}

impl ResolveModuleWorklist {
    /// Build resolve worklists from the module tree.
    pub(crate) fn from_tree(tree: &NodeTree) -> Self {
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

        // collect type references for type-space resolution
        let mut resolve_type_expression_ids = Vec::new();
        for expression_id in tree.iter_node_ids_of_type::<TypeExpression>() {
            if matches!(tree.get(expression_id), TypeExpression::Reference { .. }) {
                resolve_type_expression_ids.push(expression_id);
            }
        }

        // group dependency items by scope for export resolution
        let mut dependency_items_by_scope = FxHashMap::default();
        for item_id in tree.iter_node_ids_of_type::<DependencyItem>() {
            let (item_scope, _) = tree.get_scope(item_id);
            dependency_items_by_scope
                .entry(item_scope)
                .or_insert_with(Vec::new)
                .push(item_id);
        }

        Self {
            resolve_expression_ids,
            dependency_expression_ids,
            resolve_type_expression_ids,
            declaration_ids,
            dependency_items_by_scope,
        }
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve one batch of active expressions for one module.
    fn resolve_expression_ids(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile: ProfileId,
        prepared: &DirPrepared,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
        imported_modules: &mut ImportedModuleTable,
        exported_symbols: &mut ExportedSymbolTable,
        expression_ids: &[LocalNodeId<Expression>],
        expression_cache: &mut ResolveExpressionCache,
    ) -> ResolveResult<()> {
        let mut collector = RequirementCollector::new();
        for expression_id in expression_ids {
            if !self.is_node_active(tree, symbols, (*expression_id).into_any()) {
                continue;
            }

            self.collect(
                &mut collector,
                self.resolve_expression(
                    revision,
                    module,
                    profile,
                    tree,
                    symbols,
                    types,
                    imported_modules,
                    prepared.namespace_symbol,
                    prepared.namespace_scope,
                    prepared.global_augmentation_scope,
                    exported_symbols,
                    *expression_id,
                    expression_cache,
                ),
            );
        }

        if let Some(requirement) = collector.try_into_requirement() {
            return Err(ResolveError::Yield { requirement });
        }

        Ok(())
    }

    /// Resolve one batch of active type expressions for one module.
    fn resolve_type_expression_ids(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile: ProfileId,
        prepared: &DirPrepared,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
        exported_symbols: &mut ExportedSymbolTable,
        expression_ids: &[LocalNodeId<TypeExpression>],
    ) -> ResolveResult<()> {
        let mut collector = RequirementCollector::new();
        let mut scope_cache = ResolveScopeIndexCache::default();
        for expression_id in expression_ids {
            if !self.is_node_active(tree, symbols, (*expression_id).into_any()) {
                continue;
            }

            self.collect(
                &mut collector,
                self.resolve_type_reference_expression(
                    revision,
                    module,
                    profile,
                    tree,
                    symbols,
                    types,
                    prepared.namespace_symbol,
                    prepared.namespace_scope,
                    prepared.global_augmentation_scope,
                    exported_symbols,
                    *expression_id,
                    &mut scope_cache,
                ),
            );
        }

        if let Some(requirement) = collector.try_into_requirement() {
            return Err(ResolveError::Yield { requirement });
        }

        Ok(())
    }

    /// Resolve expressions, dependencies, and declarations for one module.
    pub(crate) fn resolve_module_direct(
        &self,
        context: &CompilerContext<'_>,
        module: &Module,
        profile: ProfileId,
        prepared: &DirPrepared,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
        export_assignment: &mut Option<LocalNodeId<DependencyItem>>,
        namespace_exports: &mut Vec<NamespaceExport>,
        module_binding_exports: &mut ModuleBindingExportTable,
        imported_modules: &mut ImportedModuleTable,
        exported_symbols: &mut ExportedSymbolTable,
    ) -> ResolveResult<()> {
        let _timing = self.timing_scope(tags::RESOLVE_MODULE_DIRECT);
        let revision = context.revision();
        if !context.is_code_module(module.id) {
            return Ok(());
        }

        // builtin declarations can skip eager expression validation in some modes
        let is_selected_library_module = self.is_selected_library_module(profile, module.id);
        let is_standard_library_environment_module =
            self.is_standard_library_environment_module(module.id);
        let skip_builtin_declaration_expressions = module.language_type.is_declaration()
            && module.is_builtin()
            && !self.options.validate_builtin_libs
            && (!is_selected_library_module || !is_standard_library_environment_module);
        let worklist = ResolveModuleWorklist::from_tree(tree);
        let mut expression_cache = ResolveExpressionCache::default();

        // dependency expressions and dependency items
        {
            let _timing = self.timing_scope(tags::RESOLVE_MODULE_DEPENDENCIES);

            if !skip_builtin_declaration_expressions {
                self.resolve_expression_ids(
                    revision,
                    module,
                    profile,
                    prepared,
                    tree,
                    symbols,
                    types,
                    imported_modules,
                    exported_symbols,
                    &worklist.dependency_expression_ids,
                    &mut expression_cache,
                )?;
            }

            self.resolve_dependency_items(
                revision,
                module,
                prepared,
                tree,
                symbols,
                namespace_exports,
                imported_modules,
                exported_symbols,
                profile,
            )?;
        }

        // remaining expressions
        {
            let _timing = self.timing_scope(tags::RESOLVE_MODULE_EXPRESSIONS);

            if !skip_builtin_declaration_expressions {
                self.resolve_expression_ids(
                    revision,
                    module,
                    profile,
                    prepared,
                    tree,
                    symbols,
                    types,
                    imported_modules,
                    exported_symbols,
                    &worklist.resolve_expression_ids,
                    &mut expression_cache,
                )?;
            }
        }

        // type expressions
        {
            let _timing = self.timing_scope(tags::RESOLVE_MODULE_EXPRESSIONS);

            if !skip_builtin_declaration_expressions {
                self.resolve_type_expression_ids(
                    revision,
                    module,
                    profile,
                    prepared,
                    tree,
                    symbols,
                    types,
                    exported_symbols,
                    &worklist.resolve_type_expression_ids,
                )?;
            }
        }

        // declarations
        {
            let _timing = self.timing_scope(tags::RESOLVE_MODULE_DECLARATIONS);
            let mut collector = RequirementCollector::new();
            for declaration_id in &worklist.declaration_ids {
                if !self.is_node_active(tree, symbols, (*declaration_id).into_any()) {
                    continue;
                }

                self.collect(
                    &mut collector,
                    self.resolve_declaration(
                        revision,
                        module,
                        profile,
                        tree,
                        symbols,
                        prepared.namespace_symbol,
                        prepared.namespace_scope,
                        prepared.global_augmentation_scope,
                        exported_symbols,
                        *declaration_id,
                    ),
                );
            }

            if let Some(requirement) = collector.try_into_requirement() {
                return Err(ResolveError::Yield { requirement });
            }
        }

        // exports
        {
            let _timing = self.timing_scope(tags::RESOLVE_MODULE_EXPORTS);

            self.finalize_module_exports(
                context,
                module,
                profile,
                tree,
                symbols,
                prepared.default_symbol,
                prepared.export_assignment_symbol,
                *export_assignment,
                exported_symbols,
            );
            self.finalize_module_binding_exports(
                context,
                module,
                profile,
                tree,
                symbols,
                &prepared.module_bindings,
                module_binding_exports,
                &worklist.dependency_items_by_scope,
            );
        }

        Ok(())
    }

    /// Resolve dependency items for one module.
    pub(crate) fn resolve_dependency_items(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        prepared: &DirPrepared,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        namespace_exports: &mut Vec<NamespaceExport>,
        imported_modules: &mut ImportedModuleTable,
        exported_symbols: &mut ExportedSymbolTable,
        profile: ProfileId,
    ) -> ResolveResult<()> {
        let mut cache = ResolveDependencyItemCache::default();
        self.resolve_dependency_items_with_cache(
            revision,
            module,
            prepared,
            tree,
            symbols,
            namespace_exports,
            imported_modules,
            exported_symbols,
            profile,
            &mut cache,
        )
    }

    /// Resolve dependency items using one shared cache.
    pub(crate) fn resolve_dependency_items_with_cache(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        prepared: &DirPrepared,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        namespace_exports: &mut Vec<NamespaceExport>,
        imported_modules: &mut ImportedModuleTable,
        exported_symbols: &mut ExportedSymbolTable,
        profile: ProfileId,
        cache: &mut ResolveDependencyItemCache,
    ) -> ResolveResult<()> {
        // cache module exports for dependency resolution
        cache.ensure_module_exports(module.id, exported_symbols);

        // collect dependency item ids once
        let item_ids = cache.dependency_item_ids_for(module.id, tree);

        // resolve dependency items
        let mut collector = RequirementCollector::new();
        let mut resolved_items = Vec::new();
        for item_id in item_ids {
            let resolved_item = self.resolve_dependency_item(
                revision,
                module,
                tree,
                symbols,
                imported_modules,
                exported_symbols,
                prepared.namespace_symbol,
                prepared.namespace_scope,
                prepared.global_augmentation_scope,
                namespace_exports,
                profile,
                item_id,
                Some(cache),
            );

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
        for (item_id, resolved_item, target_info) in resolved_items {
            let mut resolved_item = resolved_item;
            if let Some((typed_target_symbol, target_type, target_space)) = target_info {
                resolved_item =
                    self.retype_dependency_item_target(resolved_item, typed_target_symbol);

                if let Some(symbol_id) = resolved_item.symbol() {
                    let typed_symbol = symbols.retype_symbol_id(symbol_id, target_type);
                    resolved_item = self.retype_dependency_item_symbol(resolved_item, typed_symbol);
                    symbols
                        .get_symbol_mut(typed_symbol)
                        .resolve_to(typed_target_symbol);

                    let dependency_kind = match resolved_item {
                        DependencyItem::Local { kind, .. }
                        | DependencyItem::Remote { kind, .. }
                        | DependencyItem::UnresolvedLocal { kind, .. }
                        | DependencyItem::UnresolvedRemote { kind, .. } => Some(kind),
                        DependencyItem::Value { .. } | DependencyItem::Error => None,
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

        // yield unresolved dependency items after updates
        if let Some(requirement) = collector.try_into_requirement() {
            return Err(ResolveError::Yield { requirement });
        }

        Ok(())
    }
}
