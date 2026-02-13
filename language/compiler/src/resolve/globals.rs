use destack_base::StringId;
use destack_dir::{
    Argument, DependencyKind, DependencySource, Expression, GlobalNodeIdAny, GlobalSymbolId,
    LocalNodeId, ModuleResolution, ModuleTarget, NodeTree, Path, ScalarLiteral, StaticKey,
    SymbolKind, SymbolSpace, SymbolSpaceOrder, SymbolTable,
};
use destack_source::{ModuleId, PackageId};
use destack_workspace::{
    GlobalSymbolGroupKey, GlobalSymbolTable, GlobalSymbolTableKey, ImportEdgeKind, Module,
    ModuleDir, ProfileId, Target, TargetDiscovery, TargetId,
};

use crate::resolve::cache::ResolveScopeIndexCache;
use crate::{Compiler, ResolveError, ResolveResult, TargetDiscoveryIssue, TaskDependencyError};

/// Track dependency targets while scanning module trees.
#[derive(Debug, Clone, Copy)]
struct DependencyTarget {
    /// The dependency source syntax.
    source: DependencySource,
    /// The module specifier.
    target: StringId,
    /// The node that referenced the module.
    node: GlobalNodeIdAny,
    /// The dependency kind (type vs value).
    kind: DependencyKind,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Prepare the global symbol table for a module and profile.
    pub(super) fn require_global_symbol_table(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ResolveResult<GlobalSymbolTableKey> {
        let (key, roots) = self.select_global_symbol_table(module_id, profile_id)?;

        // build the cache when incomplete
        let needs_build = self
            .program
            .index
            .global_symbol_tables
            .get(&key)
            .map(|c| !c.is_complete())
            .unwrap_or(true);
        if needs_build {
            let cache = self.build_global_symbol_table_resumable(&key, &roots, profile_id)?;
            self.program
                .index
                .global_symbol_tables
                .insert(key.clone(), cache);
        }

        Ok(key)
    }

    /// Resolve a path against the global symbol table.
    pub(super) fn resolve_global_path(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        node: GlobalNodeIdAny,
        profile_id: ProfileId,
        path: &Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        space_order: SymbolSpaceOrder,
        scope_cache: Option<&mut ResolveScopeIndexCache>,
        tree: &mut NodeTree,
    ) -> ResolveResult<Option<Expression>> {
        // load the cached table for this module
        let key = self.build_global_symbol_table_key(module.id, profile_id)?;
        let Some(cache) = self.program.index.global_symbol_tables.get(&key) else {
            return Ok(None);
        };

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

        // fall back to ambient lib symbol cache when global cache misses
        let target_symbol = target_symbol.or_else(|| {
            let builtins = self.program.builtins.as_ref()?;
            let profile = self.program.profile(profile_id);
            builtins.get_ambient_lib_symbol_for_space_order(
                &profile.key,
                first_segment,
                space_order,
            )
        });

        let Some(target_symbol) = target_symbol else {
            return Ok(None);
        };

        // ensure the target module is prepared before reading its symbols
        self.require_resolve_module_prepare_if_needed(
            module.id,
            target_symbol.module_id,
            profile_id,
        )
        .map_err(|error| match error {
            TaskDependencyError::NotReady { dependency } => ResolveError::Yield { dependency },
            TaskDependencyError::Failed { dependency } => {
                ResolveError::UnsatisfiedDependency { dependency }
            }
        })?;

        // return the global reference when the path is a single segment
        if path.segments.len() == 1 {
            return Ok(Some(Expression::GlobalReference {
                path: path.clone(),
                static_arguments,
                target_symbol,
            }));
        }

        // load the target module symbols for namespace resolution
        let target_module = self.program.modules.get(target_symbol.module_id);
        let target_module = target_module.read();
        let base_dir = target_module.dir_base();
        let symbols = base_dir.symbols.read();
        let local_symbol_id = target_symbol.local_id;
        let symbol = symbols.get_symbol(local_symbol_id);

        // resolve namespace members when the root is a namespace
        if symbol.kind == SymbolKind::Namespace {
            let remaining_path = path.slice(1..);
            match self.resolve_relative_symbol_with_ambient_merge(
                &target_module,
                profile_id,
                node,
                local_symbol_id,
                &remaining_path,
                space_order,
                &symbols,
                scope_cache,
            ) {
                // resolve namespace members directly when the local scope has the full path
                Ok((resolved_id, None)) => {
                    return Ok(Some(Expression::GlobalReference {
                        path: path.clone(),
                        static_arguments,
                        target_symbol: resolved_id,
                    }));
                }
                // build a member chain when only a path prefix resolved
                Ok((resolved_id, Some(remaining))) => {
                    // if local namespace lookup consumed no member segments, retry via module exports
                    if remaining.segments.len() == remaining_path.segments.len()
                        && let Some((export_symbol, export_remaining)) = self
                            .resolve_global_namespace_member_via_exports(
                                &target_module,
                                profile_id,
                                &remaining_path,
                                space_order,
                            )?
                    {
                        let resolved_path =
                            path.slice(0..path.segments.len() - export_remaining.segments.len());
                        if export_remaining.segments.is_empty() {
                            return Ok(Some(Expression::GlobalReference {
                                path: resolved_path,
                                static_arguments,
                                target_symbol: export_symbol,
                            }));
                        }

                        let root_expr = Expression::GlobalReference {
                            path: resolved_path,
                            static_arguments: None,
                            target_symbol: export_symbol,
                        };
                        return Ok(Some(self.build_member_chain(
                            expression_id,
                            root_expr,
                            &export_remaining,
                            static_arguments,
                            tree,
                        )));
                    }
                    // build regular global reference member chain
                    else {
                        let resolved_path =
                            path.slice(0..path.segments.len() - remaining.segments.len());
                        let root_expr = Expression::GlobalReference {
                            path: resolved_path,
                            static_arguments: None,
                            target_symbol: resolved_id,
                        };
                        return Ok(Some(self.build_member_chain(
                            expression_id,
                            root_expr,
                            &remaining,
                            static_arguments,
                            tree,
                        )));
                    }
                }
                // fall back to module exports for export-as-namespace alias members
                Err(error @ ResolveError::MissingSymbol { .. }) => {
                    if let Some((resolved_id, remaining)) = self
                        .resolve_global_namespace_member_via_exports(
                            &target_module,
                            profile_id,
                            &remaining_path,
                            space_order,
                        )?
                    {
                        let resolved_path =
                            path.slice(0..path.segments.len() - remaining.segments.len());
                        if remaining.segments.is_empty() {
                            return Ok(Some(Expression::GlobalReference {
                                path: resolved_path,
                                static_arguments,
                                target_symbol: resolved_id,
                            }));
                        }

                        let root_expr = Expression::GlobalReference {
                            path: resolved_path,
                            static_arguments: None,
                            target_symbol: resolved_id,
                        };
                        return Ok(Some(self.build_member_chain(
                            expression_id,
                            root_expr,
                            &remaining,
                            static_arguments,
                            tree,
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
            static_arguments: None,
            target_symbol,
        };
        Ok(Some(self.build_member_chain(
            expression_id,
            root_expr,
            &path.slice(1..),
            static_arguments,
            tree,
        )))
    }

    /// Resolve a global namespace member through module exports.
    fn resolve_global_namespace_member_via_exports(
        &self,
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
        let anchor = module.dir_base().anchor_node.into_global(module.id);
        let Some(resolved_symbol) = self.resolve_export_symbol_for_target(
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

    /// Build the cache key for a module and profile.
    pub(crate) fn build_global_symbol_table_key(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ResolveResult<GlobalSymbolTableKey> {
        // load module and package metadata
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let package_id = module.package_id;
        let package = self.program.packages.get(package_id);
        let package = package.read();
        let has_targets = !package.targets.is_empty();
        drop(package);

        // select the entry module and target id for the key
        let entry_module = (!has_targets).then_some(module_id);
        let target_id = if has_targets {
            self.select_default_target_for_package(package_id)?.0
        } else {
            TargetId::new(package_id, "default")
        };
        Ok(GlobalSymbolTableKey {
            target_id,
            profile_id,
            entry_module,
        })
    }

    /// Resolve a global symbol group by key and space.
    pub(crate) fn get_global_symbol_group(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        key: StaticKey,
        space: SymbolSpace,
    ) -> Option<Vec<GlobalSymbolId>> {
        let cache_key = self
            .build_global_symbol_table_key(module_id, profile_id)
            .ok()?;
        let cache = self.program.index.global_symbol_tables.get(&cache_key)?;
        cache
            .sources_by_space
            .get(&GlobalSymbolGroupKey { key, space })
            .cloned()
    }

    /// Select the global symbol table roots for a module.
    /// Returns the cache key and root module list.
    pub(super) fn select_global_symbol_table(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ResolveResult<(GlobalSymbolTableKey, Vec<ModuleId>)> {
        // load module and package metadata
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let package_id = module.package_id;
        let package = self.program.packages.get(package_id);
        let package = package.read();
        let package_path = package.path.clone();
        let has_targets = !package.targets.is_empty();
        drop(package);

        // fall back to the current module when no targets exist
        if !has_targets {
            let key = GlobalSymbolTableKey {
                target_id: TargetId::new(package_id, "default"),
                profile_id,
                entry_module: Some(module_id),
            };
            return Ok((key, vec![module_id]));
        }

        // prefer roots based on the package target discovery rules
        let (target_id, target) = self.select_default_target_for_package(package_id)?;
        let roots = match target.discovery {
            TargetDiscovery::Entry => self
                .discover_entry_modules(package_id, &package_path, &target, &target_id)
                .map_err(|issue| self.map_target_discovery_issue(issue))?,
            TargetDiscovery::Include => self
                .discover_include_modules(package_id, &package_path, &target)
                .map_err(|issue| self.map_target_discovery_issue(issue))?,
        };
        let key = GlobalSymbolTableKey {
            target_id,
            profile_id,
            entry_module: None,
        };
        Ok((key, roots))
    }

    /// Select the default target for a package.
    fn select_default_target_for_package(
        &self,
        package_id: PackageId,
    ) -> ResolveResult<(TargetId, Target)> {
        // load package metadata
        let package = self.program.packages.get(package_id);
        let package = package.read();

        // honor an explicit default target from dsconfig
        if let Some(dsconfig) = package.dsconfig.as_ref()
            && let Some(default_target) = dsconfig.options.default_target.as_ref()
        {
            let target_id = TargetId::new(package_id, default_target);
            let Some(target) = package.targets.get(&target_id) else {
                return Err(ResolveError::InvalidTargetConfig {
                    package: package_id,
                    target: target_id.clone(),
                    message: "default target not found".to_string(),
                });
            };
            return Ok((target_id, target.clone()));
        }

        // reject packages with no targets configured
        if package.targets.is_empty() {
            return Err(ResolveError::InvalidTargetConfig {
                package: package_id,
                target: TargetId::new(package_id, "default"),
                message: "package has no targets".to_string(),
            });
        }

        // select the only configured target when there is exactly one
        if package.targets.len() == 1 {
            let (target_id, target) = package.targets.iter().next().expect("checked len");
            return Ok((target_id.clone(), target.clone()));
        }

        // require an explicit default target when multiple targets exist
        let mut target_names: Vec<String> = package
            .targets
            .keys()
            .map(|target_id| target_id.name.clone())
            .collect();
        target_names.sort();
        let available = target_names.join(", ");
        Err(ResolveError::InvalidTargetConfig {
            package: package_id,
            target: TargetId::new(package_id, "default"),
            message: format!("default target not specified; available targets: {available}"),
        })
    }

    /// Map a target discovery issue into a resolve error.
    fn map_target_discovery_issue(&self, issue: TargetDiscoveryIssue) -> ResolveError {
        // normalize issues into invalid target configuration errors
        match issue {
            TargetDiscoveryIssue::MissingPackagePath { package, target } => {
                ResolveError::InvalidTargetConfig {
                    package,
                    target,
                    message: "entry based discovery requires package path".to_string(),
                }
            }
            TargetDiscoveryIssue::MissingEntry {
                package,
                target,
                path,
            } => ResolveError::InvalidTargetConfig {
                package,
                target,
                message: format!("entry point not found: {}", path.display()),
            },
        }
    }

    /// Build a global symbol cache for freestanding modules.
    /// This collects all global symbols from the given modules in one go.
    pub(super) fn build_global_symbol_table_freestanding(
        &self,
        modules: &[ModuleId],
        profile_id: ProfileId,
    ) -> ResolveResult<GlobalSymbolTable> {
        let mut cache = GlobalSymbolTable::new();
        cache.pending.extend(modules.iter().copied());

        // process all lib modules
        while let Some(module_id) = cache.pending.pop_front() {
            // skip already processed modules
            if cache.module_versions.contains_key(&module_id) {
                continue;
            }

            // load the module tree and symbols
            self.require_import_module_validate(module_id)?;
            self.require_resolve_module_prepare(module_id, profile_id)?;
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let dir = module.dir(profile_id);
            let tree = dir.tree.read();
            let symbols = dir.symbols.read();
            cache.module_versions.insert(module_id, module.version);

            // collect global declarations from this module
            self.collect_global_augmentation_symbols(&module, dir, &symbols, &mut cache);
            if module.language_type.is_declaration() {
                self.collect_export_namespace_globals(&module, &tree, &symbols, &mut cache);
            }
            if self.module_exposes_namespace_scope_globals(&module) {
                self.collect_namespace_scope_globals(&module, dir, &symbols, &mut cache);
            }
        }

        Ok(cache)
    }

    /// Build the global symbol table for a root module set.
    /// Resumes from partial state if available.
    fn build_global_symbol_table_resumable(
        &self,
        key: &GlobalSymbolTableKey,
        roots: &[ModuleId],
        profile_id: ProfileId,
    ) -> ResolveResult<GlobalSymbolTable> {
        // resume from partial cache or start fresh
        let mut cache = self
            .program
            .index
            .global_symbol_tables
            .remove(key)
            .map(|(_, c)| c)
            .unwrap_or_else(|| {
                let mut c = GlobalSymbolTable::new();
                c.pending.extend(roots.iter().copied());
                c
            });

        // walk the module graph
        while let Some(module_id) = cache.pending.pop_front() {
            // skip already processed modules
            if cache.module_versions.contains_key(&module_id) {
                continue;
            }

            // ensure bind validation before reading dir data
            if let Err(error) = self.require_import_module_validate(module_id) {
                // put current module back for retry
                cache.pending.push_front(module_id);
                self.program
                    .index
                    .global_symbol_tables
                    .insert(key.clone(), cache);
                return Err(error.into());
            }

            // load the module tree and symbols
            self.require_resolve_module_prepare(module_id, profile_id)?;
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let dir = module.dir(profile_id);
            let tree = dir.tree.read();
            let symbols = dir.symbols.read();
            cache.module_versions.insert(module_id, module.version);

            // collect global declarations from this module
            self.collect_global_augmentation_symbols(&module, dir, &symbols, &mut cache);
            if module.language_type.is_declaration() {
                self.collect_export_namespace_globals(&module, &tree, &symbols, &mut cache);
            }
            if self.module_exposes_namespace_scope_globals(&module) {
                self.collect_namespace_scope_globals(&module, dir, &symbols, &mut cache);
            }

            // enqueue dependency targets for further discovery
            let dependency_targets = self.collect_dependency_targets(module_id, &tree, &symbols);
            for dependency in dependency_targets {
                // skip module bindings before resolving file targets
                if self
                    .resolve_module_binding_target(module_id, profile_id, dependency.target)?
                    .is_some()
                {
                    continue;
                }

                // resolve specifiers to modules for traversal
                let resolved_targets = match self.resolve_dependency_targets_for_global_traversal(
                    module_id, profile_id, dependency,
                ) {
                    Ok(targets) => targets,
                    Err(_) if module.is_builtin() && dependency.kind == DependencyKind::Type => {
                        continue;
                    }
                    Err(_) => {
                        self.handle_unresolved_module(ResolveError::UnresolvedModule {
                            node: dependency.node.into_anchored(Some(profile_id)),
                            target: dependency.target,
                        });
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
        if self.module_is_ambient_lib(module) {
            return true;
        }

        // declaration scripts also contribute top-level global declarations
        module.language_type.is_declaration() && module.source_type.is_script()
    }

    /// Resolve one dependency target for global symbol table traversal.
    fn resolve_dependency_targets_for_global_traversal(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        dependency: DependencyTarget,
    ) -> ResolveResult<ModuleResolution> {
        // resolve triple slash reference lib directives through builtin library loading
        if dependency.source == DependencySource::ReferenceLibDirective {
            let target_text = self.program.strings.get(dependency.target);
            let module_id = self
                .resolve_reference_lib_to_module(profile_id, target_text.as_ref())
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

        self.resolve_specifier_to_module_resolution(
            target,
            Some(module_id),
            ImportEdgeKind::Import,
            None,
        )
        .map_err(|_| ResolveError::UnresolvedModule {
            node: dependency.node.into_anchored(Some(profile_id)),
            target: dependency.target,
        })
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
        dir: &ModuleDir,
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
        let symbol_id = module.dir_base().namespace_symbol.into_global(module.id);
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
        dir: &ModuleDir,
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
                        source: DependencySource::ExportStatement,
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
                Expression::TypeImport { target, .. } => {
                    if let Expression::ScalarLiteral {
                        value: ScalarLiteral::String(target),
                    } = tree.get(*target)
                    {
                        targets.push(DependencyTarget {
                            source: DependencySource::ImportStatement,
                            target: *target,
                            node: expression_id.into_global_any(module_id),
                            kind: DependencyKind::Type,
                        });
                    }
                }
                _ => {}
            }
        }
        targets
    }
}

#[cfg(test)]
mod tests {
    use destack_dir::{DependencySource, StaticKey};

    use crate::TestProgram;

    /// Test that symbols inside `declare global { }` blocks are collected as globals.
    #[test]
    fn test_collect_global_symbols_declare_global() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.d.ts",
            r#"
declare global {
    var TestGlobal: string;
    function testGlobalFn(): void;
    interface TestGlobalInterface {}
}
export {};
"#,
        );
        test.resolve_module(module_id);
        test.compile_check_clean();

        let profile = test.default_profile_id(module_id);
        let cache = test
            .compiler
            .build_global_symbol_table_freestanding(&[module_id], profile)
            .unwrap();

        // check that the global symbols were collected
        let test_global_key = StaticKey::Name(test.program.strings.intern("TestGlobal"));
        assert!(cache.symbols.contains_key(&test_global_key),);
        let test_fn_key = StaticKey::Name(test.program.strings.intern("testGlobalFn"));
        assert!(cache.symbols.contains_key(&test_fn_key),);
        let test_iface_key = StaticKey::Name(test.program.strings.intern("TestGlobalInterface"));
        assert!(cache.symbols.contains_key(&test_iface_key),);
    }

    /// Resolve `export as namespace` globals in both value and type spaces.
    #[test]
    fn test_collect_export_namespace_globals_in_type_space() {
        let test = TestProgram::memory_sequential();
        test.add_module(
            "a.d.ts",
            r#"
export as namespace babel;

export namespace types {
    export interface Expression {
        kind: string;
    }
}
"#,
        );
        let module_id = test.add_module(
            "main.ts",
            r#"
import { types } from "./a";

type Expression = babel.types.Expression;
const expression: types.Expression = { kind: "ok" };
"#,
        );
        test.resolve_module(module_id);
        test.compile_check_clean();
    }

    /// Resolve triple slash path directives to same-directory declaration modules.
    #[test]
    fn test_collect_global_symbols_from_triple_slash_declaration_script() {
        let test = TestProgram::memory_sequential();

        // normalize one bare reference path target to same-directory relative form
        let bare_target = test.program.strings.intern("global.d.ts");
        let normalized_target = test.compiler.resolve_target_for_dependency_source(
            DependencySource::ReferencePathDirective,
            bare_target,
        );
        let normalized_text = test.program.strings.get(normalized_target);
        assert_eq!(normalized_text.as_ref(), "./global.d.ts");

        // keep explicit relative path targets unchanged
        let explicit_target = test.program.strings.intern("./global.d.ts");
        let explicit_result = test.compiler.resolve_target_for_dependency_source(
            DependencySource::ReferencePathDirective,
            explicit_target,
        );
        assert_eq!(explicit_result, explicit_target);
    }

    /// Resolve triple slash type package directives through @types package lookup.

    #[test]
    fn test_collect_global_symbols_from_triple_slash_types_package() {
        let test = TestProgram::memory_sequential();
        test.add_module(
            "node_modules/@types/runner-types/package.json",
            r#"{
  "name": "@types/runner-types",
  "types": "./index.d.ts"
}"#,
        );
        test.add_module(
            "node_modules/@types/runner-types/index.d.ts",
            r#"
interface RunnerGlobal {
    id: string;
}
"#,
        );
        test.add_module(
            "ambient.d.ts",
            r#"
/// <reference types="runner-types" />

export interface Markup {
    value: RunnerGlobal;
}
"#,
        );
        let module_id = test.add_module(
            "main.ts",
            r#"
import type { Markup } from "./ambient";

const markup: Markup = {
    value: {
        id: "ok",
    },
};
"#,
        );
        test.resolve_module(module_id);
        test.compile_check_clean();
    }

    /// Resolve triple slash lib directives through builtin library loading.
    #[test]
    fn test_collect_global_symbols_from_triple_slash_lib_directive() {
        let test = TestProgram::memory_sequential();
        test.add_module(
            "ambient.d.ts",
            r#"
/// <reference lib="esnext.disposable" />

export interface ResourceHolder {
    resource: Disposable;
}
"#,
        );
        let module_id = test.add_module(
            "main.ts",
            r#"
import type { ResourceHolder } from "./ambient";

declare const holder: ResourceHolder;
holder.resource;
"#,
        );
        test.resolve_module(module_id);
        test.compile_check_clean();
    }

    /// Resolve export-namespace globals when value imports use declaration companions.
    #[test]
    fn test_collect_export_namespace_globals_from_companion_type_target() {
        let test = TestProgram::memory_sequential();
        test.add_module(
            "babel.js",
            r#"
export const types = {};
"#,
        );
        test.add_module(
            "babel.d.ts",
            r#"
export as namespace babel;

export namespace types {
    export interface Expression {
        kind: string;
    }

    export interface V8IntrinsicIdentifier {
        intrinsic: string;
    }
}
"#,
        );
        let module_id = test.add_module(
            "main.ts",
            r#"
import "./babel.js";

type BabelType = babel.types.Expression | babel.types.V8IntrinsicIdentifier;
const value: BabelType | null = null;
"#,
        );
        test.resolve_module(module_id);
        test.compile_check_clean();
    }

    /// Resolve export-namespace globals when members are export aliases.
    #[test]
    fn test_collect_export_namespace_globals_from_export_alias() {
        let test = TestProgram::memory_sequential();
        test.add_module(
            "types.d.ts",
            r#"
export interface Expression {
    kind: string;
}

export interface V8IntrinsicIdentifier {
    intrinsic: string;
}
"#,
        );
        test.add_module(
            "babel.js",
            r#"
export {};
"#,
        );
        test.add_module(
            "babel.d.ts",
            r#"
import * as t from "./types";

export { t as types };
export as namespace babel;
"#,
        );
        let module_id = test.add_module(
            "main.ts",
            r#"
import "./babel.js";

type BabelType = babel.types.Expression | babel.types.V8IntrinsicIdentifier;
const value: BabelType | null = null;
"#,
        );
        test.resolve_module(module_id);
        test.compile_check_clean();
    }

    /// Test that symbols inside nested `declare module "x" { global { } }` are collected.
    #[test]
    fn test_collect_global_symbols_nested_in_module() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.d.ts",
            r#"
declare module "buffer" {
    global {
        var Buffer: string;
    }
}
"#,
        );
        test.resolve_module(module_id);
        test.compile_check_clean();

        let profile = test.default_profile_id(module_id);
        let cache = test
            .compiler
            .build_global_symbol_table_freestanding(&[module_id], profile)
            .unwrap();

        let buffer_key = StaticKey::Name(test.program.strings.intern("Buffer"));
        assert!(cache.symbols.contains_key(&buffer_key),);
    }
}
