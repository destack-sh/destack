use std::collections::VecDeque;

use destack_base::StringId;
use destack_dir::{
    Argument, Expression, GlobalNodeIdAny, GlobalSymbolId, LocalNodeId, NodeTree, Path, StaticKey,
    SymbolKind, SymbolSpace, SymbolSpaceOrder, SymbolTable,
};
use destack_source::{ModuleId, ModuleVersion, PackageId};
use destack_workspace::{Module, ModuleDir, ProfileId, Target, TargetDiscovery, TargetId};
use indexmap::IndexMap;

use crate::{Compiler, ResolveError, ResolveResult, TargetDiscoveryIssue, TaskDependencyError};

/// Identify a cached global symbol table view for a target and profile.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct GlobalSymbolCacheKey {
    /// Target id for the module selection.
    pub target_id: TargetId,
    /// Profile id for the compilation.
    pub profile_id: ProfileId,
    /// Entry module id when target discovery is implicit.
    pub entry_module: Option<ModuleId>,
}

/// Track global symbols from declare global blocks reachable from a root set.
#[derive(Debug, Clone)]
pub(crate) struct GlobalSymbolCache {
    /// Versions for modules included in the cache.
    pub module_versions: IndexMap<ModuleId, ModuleVersion>,
    /// First symbol observed for each global key.
    pub symbols: IndexMap<StaticKey, GlobalSymbolId>,
    /// First symbol observed for each global key and space.
    pub symbols_by_space: IndexMap<GlobalSymbolGroupKey, GlobalSymbolId>,
    /// All symbols observed for each global key.
    pub sources: IndexMap<StaticKey, Vec<GlobalSymbolId>>,
    /// All symbols observed for each global key and space.
    pub sources_by_space: IndexMap<GlobalSymbolGroupKey, Vec<GlobalSymbolId>>,
    /// Modules remaining to process (empty once complete).
    pub pending: VecDeque<ModuleId>,
}

impl GlobalSymbolCache {
    /// Create an empty table.
    fn new() -> Self {
        Self {
            module_versions: IndexMap::new(),
            symbols: IndexMap::new(),
            symbols_by_space: IndexMap::new(),
            sources: IndexMap::new(),
            sources_by_space: IndexMap::new(),
            pending: VecDeque::new(),
        }
    }

    /// Check if the cache is complete (no pending modules).
    fn is_complete(&self) -> bool {
        self.pending.is_empty()
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

/// Key for grouping global symbols by name and space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct GlobalSymbolGroupKey {
    /// The symbol key.
    pub key: StaticKey,
    /// The symbol space.
    pub space: SymbolSpace,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Prepare the global symbol table for a module and profile.
    pub(super) fn require_global_symbol_cache(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ResolveResult<GlobalSymbolCacheKey> {
        let (key, roots) = self.select_global_symbol_table(module_id, profile_id)?;

        // build the cache when incomplete
        let needs_build = self
            .global_symbol_caches
            .get(&key)
            .map(|c| !c.is_complete())
            .unwrap_or(true);
        if needs_build {
            let cache = self.build_global_symbol_cache_resumable(&key, &roots, profile_id)?;
            self.global_symbol_caches.insert(key.clone(), cache);
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
        tree: &mut NodeTree,
    ) -> ResolveResult<Option<Expression>> {
        // load the cached table for this module
        let key = self.build_global_symbol_table_key(module.id, profile_id)?;
        let Some(cache) = self.global_symbol_caches.get(&key) else {
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
            match self.resolve_relative_symbol(
                &target_module,
                profile_id,
                node,
                local_symbol_id,
                &remaining_path,
                space_order,
                &symbols,
            ) {
                Ok((resolved_id, None)) => {
                    return Ok(Some(Expression::GlobalReference {
                        path: path.clone(),
                        static_arguments,
                        target_symbol: resolved_id.into_global(target_symbol.module_id),
                    }));
                }
                Ok((resolved_id, Some(remaining))) => {
                    let resolved_path =
                        path.slice(0..path.segments.len() - remaining.segments.len());
                    let root_expr = Expression::GlobalReference {
                        path: resolved_path,
                        static_arguments: None,
                        target_symbol: resolved_id.into_global(target_symbol.module_id),
                    };
                    return Ok(Some(self.build_member_chain(
                        expression_id,
                        root_expr,
                        &remaining,
                        static_arguments,
                        tree,
                    )));
                }
                Err(e) => return Err(e),
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

    /// Build the cache key for a module and profile.
    pub(crate) fn build_global_symbol_table_key(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ResolveResult<GlobalSymbolCacheKey> {
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
        Ok(GlobalSymbolCacheKey {
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
        let cache = self.global_symbol_caches.get(&cache_key)?;
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
    ) -> ResolveResult<(GlobalSymbolCacheKey, Vec<ModuleId>)> {
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
            let key = GlobalSymbolCacheKey {
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
        let key = GlobalSymbolCacheKey {
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

        // fall back to the first declared target
        let Some((target_id, target)) = package.targets.iter().next() else {
            return Err(ResolveError::InvalidTargetConfig {
                package: package_id,
                target: TargetId::new(package_id, "default"),
                message: "package has no targets".to_string(),
            });
        };
        Ok((target_id.clone(), target.clone()))
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
    pub(super) fn build_global_symbol_cache_freestanding(
        &self,
        modules: &[ModuleId],
        profile_id: ProfileId,
    ) -> ResolveResult<GlobalSymbolCache> {
        let mut cache = GlobalSymbolCache::new();
        cache.pending.extend(modules.iter().copied());

        // process all lib modules
        while let Some(module_id) = cache.pending.pop_front() {
            // skip already processed modules
            if cache.module_versions.contains_key(&module_id) {
                continue;
            }

            // load the module tree and symbols
            self.require_import_module_validate(module_id)?;
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let base_dir = module.dir(profile_id);
            let tree = base_dir.tree.read();
            let symbols = base_dir.symbols.read();
            cache.module_versions.insert(module_id, module.version);

            // collect global declarations from this module
            self.collect_global_augmentation_symbols(&module, base_dir, &symbols, &mut cache);
            if module.language_type.is_declaration() {
                self.collect_export_namespace_globals(&module, &tree, &mut cache);
            }
            if self.module_is_ambient_lib(&module) {
                self.collect_namespace_scope_globals(&module, base_dir, &symbols, &mut cache);
            }
        }

        Ok(cache)
    }

    /// Build the global symbol table for a root module set.
    /// Resumes from partial state if available.
    fn build_global_symbol_cache_resumable(
        &self,
        key: &GlobalSymbolCacheKey,
        roots: &[ModuleId],
        profile_id: ProfileId,
    ) -> ResolveResult<GlobalSymbolCache> {
        // resume from partial cache or start fresh
        let mut cache = self
            .global_symbol_caches
            .remove(key)
            .map(|(_, c)| c)
            .unwrap_or_else(|| {
                let mut c = GlobalSymbolCache::new();
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
                self.global_symbol_caches.insert(key.clone(), cache);
                return Err(error.into());
            }

            // load the module tree and symbols
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let base_dir = module.dir_base();
            let tree = base_dir.tree.read();
            let symbols = base_dir.symbols.read();
            cache.module_versions.insert(module_id, module.version);

            // collect global declarations from this module
            self.collect_global_augmentation_symbols(&module, base_dir, &symbols, &mut cache);
            if module.language_type.is_declaration() {
                self.collect_export_namespace_globals(&module, &tree, &mut cache);
            }
            if self.module_is_ambient_lib(&module) {
                self.collect_namespace_scope_globals(&module, base_dir, &symbols, &mut cache);
            }

            // enqueue dependency targets for further discovery
            let dependency_targets = self.collect_dependency_targets(module_id, &tree);
            for (target, node) in dependency_targets {
                // skip module bindings before resolving file targets
                if self
                    .resolve_module_binding_target(module_id, profile_id, target)?
                    .is_some()
                {
                    continue;
                }

                // resolve specifiers to modules for traversal
                let remote_module_id = self
                    .resolve_specifier_to_module(target, Some(module_id))
                    .map_err(|_| ResolveError::UnresolvedModule {
                        node: node.into_anchored(Some(profile_id)),
                        target,
                    })?;
                cache.pending.push_back(remote_module_id);
            }
        }

        Ok(cache)
    }

    /// Collect global symbols from the global augmentation scope.
    /// Symbols inside `declare global { }` blocks are bound into this scope by the binder.
    fn collect_global_augmentation_symbols(
        &self,
        module: &Module,
        dir: &ModuleDir,
        symbols: &SymbolTable,
        cache: &mut GlobalSymbolCache,
    ) {
        let scope = symbols.get_scope_by_id(dir.global_augmentation_scope);
        for (key, symbol_id) in &scope.named_symbols {
            let symbol = symbols.get_symbol(*symbol_id);
            cache.insert_symbol(*key, symbol.space, symbol_id.into_global(module.id));
        }
    }

    /// Collect global symbols from `export as namespace` declarations.
    /// Only call this for declaration modules (`.d.ts`).
    fn collect_export_namespace_globals(
        &self,
        module: &Module,
        tree: &NodeTree,
        cache: &mut GlobalSymbolCache,
    ) {
        let symbol_id = module.dir_base().namespace_symbol.into_global(module.id);
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            let Expression::ExportNamespace { name } = tree.get(expression_id) else {
                continue;
            };
            cache.insert_symbol(StaticKey::Name(*name), SymbolSpace::Value, symbol_id);
        }
    }

    /// Collect top-level declarations from a module's namespace scope as globals.
    /// Intended only for ambient/global script modules where top-level symbols are globals.
    fn collect_namespace_scope_globals(
        &self,
        module: &Module,
        dir: &ModuleDir,
        symbols: &SymbolTable,
        cache: &mut GlobalSymbolCache,
    ) {
        let scope = symbols.get_scope_by_id(dir.namespace_scope);
        for (key, symbol_id) in &scope.named_symbols {
            let symbol = symbols.get_symbol(*symbol_id);
            cache.insert_symbol(*key, symbol.space, symbol_id.into_global(module.id));
        }
    }

    /// Collect module specifiers referenced by imports and reexports.
    pub(super) fn collect_dependency_targets(
        &self,
        module_id: ModuleId,
        tree: &NodeTree,
    ) -> Vec<(StringId, GlobalNodeIdAny)> {
        // collect import and reexport targets
        let mut targets = Vec::new();
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            match tree.get(expression_id) {
                Expression::UnresolvedImport { target, .. }
                | Expression::UnresolvedReExport { target, .. }
                | Expression::Import { target, .. }
                | Expression::ReExport { target, .. } => {
                    targets.push((*target, expression_id.into_global_any(module_id)));
                }
                Expression::TypeImport { target, .. } => {
                    targets.push((*target, expression_id.into_global_any(module_id)));
                }
                _ => {}
            }
        }
        targets
    }
}

#[cfg(test)]
mod tests {
    use destack_dir::StaticKey;

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
            .build_global_symbol_cache_freestanding(&[module_id], profile)
            .unwrap();

        // check that the global symbols were collected
        let test_global_key = StaticKey::Name(test.program.strings.intern("TestGlobal"));
        assert!(cache.symbols.contains_key(&test_global_key),);
        let test_fn_key = StaticKey::Name(test.program.strings.intern("testGlobalFn"));
        assert!(cache.symbols.contains_key(&test_fn_key),);
        let test_iface_key = StaticKey::Name(test.program.strings.intern("TestGlobalInterface"));
        assert!(cache.symbols.contains_key(&test_iface_key),);
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
            .build_global_symbol_cache_freestanding(&[module_id], profile)
            .unwrap();

        let buffer_key = StaticKey::Name(test.program.strings.intern("Buffer"));
        assert!(cache.symbols.contains_key(&buffer_key),);
    }
}
