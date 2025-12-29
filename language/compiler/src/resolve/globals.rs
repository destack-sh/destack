use std::collections::{HashSet, VecDeque};

use destack_base::StringId;
use destack_dir::{
    Argument, Declaration, Expression, GlobalNodeIdAny, GlobalSymbolId, LocalNodeId, NodeTree,
    Path, StaticKey, SymbolKind, SymbolTable,
};
use destack_source::{ModuleId, ModuleVersion, PackageId};
use destack_workspace::{Module, ProfileId, Target, TargetDiscovery, TargetId};
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
    /// Root modules used to seed discovery.
    pub roots: Vec<ModuleId>,
    /// Versions for modules included in the index.
    pub module_versions: IndexMap<ModuleId, ModuleVersion>,
    /// First symbol observed for each global key.
    pub symbols: IndexMap<StaticKey, GlobalSymbolId>,
    /// All symbols observed for each global key.
    pub sources: IndexMap<StaticKey, Vec<GlobalSymbolId>>,
}

impl GlobalSymbolCache {
    /// Create an empty table seeded with roots.
    fn new(roots: Vec<ModuleId>) -> Self {
        Self {
            roots,
            module_versions: IndexMap::new(),
            symbols: IndexMap::new(),
            sources: IndexMap::new(),
        }
    }

    /// Insert a global symbol and preserve the first binding for the key.
    fn insert_symbol(&mut self, key: StaticKey, symbol: GlobalSymbolId) {
        self.sources.entry(key).or_default().push(symbol);
        self.symbols.entry(key).or_insert(symbol);
    }
}

impl Compiler {
    /// Prepare the global symbol table for a module and profile.
    pub(super) fn prepare_global_symbol_table(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ResolveResult<GlobalSymbolCacheKey> {
        // select the root set for this module
        let (key, roots) = self.select_global_symbol_table(module_id, profile_id)?;

        // reuse cached table when roots and module versions match
        let should_rebuild = self
            .global_symbol_caches
            .get(&key)
            .map(|index| self.is_global_symbol_table_stale(&index, &roots))
            .unwrap_or(true);

        // rebuild the table when it is stale
        if should_rebuild {
            let index = self.build_global_symbol_table(&roots)?;
            self.global_symbol_caches.insert(key.clone(), index);
        }
        Ok(key)
    }

    /// Resolve a path against the global symbol table.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_global_path(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        node: GlobalNodeIdAny,
        profile_id: ProfileId,
        path: &Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        tree: &mut NodeTree,
    ) -> ResolveResult<Option<Expression>> {
        // load the cached table for this module
        let key = self.build_global_symbol_table_key(module.id, profile_id)?;
        let Some(index) = self.global_symbol_caches.get(&key) else {
            return Ok(None);
        };

        // resolve the first path segment against global symbols
        let first_segment = path.first_segment().expect("path is empty");
        let key = StaticKey::Name(first_segment);
        let Some(target_symbol) = index.symbols.get(&key).copied() else {
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
                node,
                local_symbol_id,
                &remaining_path,
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

    /// Check whether a cached symbol table is stale.
    fn is_global_symbol_table_stale(&self, index: &GlobalSymbolCache, roots: &[ModuleId]) -> bool {
        // roots must match the current selection
        if index.roots != roots {
            return true;
        }

        // module versions must match the current program view
        for (module_id, version) in &index.module_versions {
            let module = self.program.modules.get(*module_id);
            let module = module.read();
            if module.version != *version {
                return true;
            }
        }
        false
    }

    /// Build the cache key for a module and profile.
    fn build_global_symbol_table_key(
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

    /// Select the global symbol table roots for a module.
    /// Returns the cache key and root module list.
    fn select_global_symbol_table(
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

    /// Build the global symbol table for a root module set.
    fn build_global_symbol_table(&self, roots: &[ModuleId]) -> ResolveResult<GlobalSymbolCache> {
        // initialize the traversal state
        let mut index = GlobalSymbolCache::new(roots.to_vec());
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.extend(roots.iter().copied());

        // walk the module graph starting from the roots
        while let Some(module_id) = queue.pop_front() {
            if !visited.insert(module_id) {
                continue;
            }

            // ensure bind validation before reading dir data
            self.require_bind_module_validate(module_id)
                .map_err(|error| match error {
                    TaskDependencyError::NotReady { dependency } => {
                        ResolveError::Yield { dependency }
                    }
                    TaskDependencyError::Failed { dependency } => {
                        ResolveError::UnsatisfiedDependency { dependency }
                    }
                })?;

            // load the module tree and symbols
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let base_dir = module.dir_base();
            let tree = base_dir.tree.read();
            let symbols = base_dir.symbols.read();
            index.module_versions.insert(module_id, module.version);

            // collect global declarations from this module
            self.collect_global_symbols(module_id, &tree, &symbols, &mut index);

            // enqueue dependency targets for further discovery
            let dependency_targets = self.collect_dependency_targets(module_id, &tree);
            for (target, node) in dependency_targets {
                let remote_module_id = self
                    .resolve_specifier_to_module(target, Some(module_id))
                    .map_err(|_| ResolveError::UnresolvedModule { node, target })?;
                queue.push_back(remote_module_id);
            }
        }

        Ok(index)
    }

    /// Collect global symbols from declare global blocks in a module.
    fn collect_global_symbols(
        &self,
        module_id: ModuleId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        index: &mut GlobalSymbolCache,
    ) {
        // scan for declare global blocks
        for declaration_id in tree.iter_node_ids_of_type::<Declaration>() {
            let Declaration::Global { expressions, .. } = tree.get(declaration_id) else {
                continue;
            };

            // collect expressions inside the global block
            for expression_id in expressions {
                self.collect_global_expression(module_id, tree, symbols, index, *expression_id);
            }
        }
    }

    /// Collect global symbols from an expression inside a global block.
    fn collect_global_expression(
        &self,
        module_id: ModuleId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        index: &mut GlobalSymbolCache,
        expression_id: LocalNodeId<Expression>,
    ) {
        // walk statements and declarations inside a global block
        match tree.get(expression_id) {
            Expression::Statement { statement } => {
                self.collect_global_expression(module_id, tree, symbols, index, *statement);
            }
            Expression::Declaration { declaration } => {
                // register the symbol for the global key
                let declaration = tree.get(*declaration);
                let symbol_id = declaration.descriptor().symbol;
                let symbol = symbols.get_symbol(symbol_id);
                let Some(key) = symbol.key else {
                    return;
                };
                index.insert_symbol(key, symbol_id.into_global(module_id));
            }
            _ => {}
        }
    }

    /// Collect module specifiers referenced by imports and reexports.
    fn collect_dependency_targets(
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
