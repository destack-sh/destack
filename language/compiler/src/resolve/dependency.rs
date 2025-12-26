use destack_ast::StringId;
use destack_dir::{
    DependencyItem, DependencyMode, DependencySource, GlobalNodeIdAny, GlobalSymbolId, LocalNodeId,
    LocalScopeMark, NodeTree, StaticKey, SymbolTable,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ModuleDir, ProfileId};

use crate::{Compiler, ResolveError, ResolveResult, TaskDependencyError};

impl Compiler {
    /// Whether the target is a relative import.
    pub(super) fn is_import_relative(&self, target: StringId) -> bool {
        let target_str = self.program.strings.get(target);
        target_str.starts_with("./") || target_str.starts_with("../")
    }

    /// Try to resolve an import of some target specifier synchronously.
    pub(super) fn resolve_import(
        &self,
        module: &Module,
        dir: &ModuleDir,
        _profile: ProfileId,
        node: GlobalNodeIdAny,
        _source: DependencySource,
        target: StringId,
    ) -> ResolveResult<ModuleId> {
        let is_relative = self.is_import_relative(target);
        let relative_module = if is_relative { Some(module.id) } else { None };

        // check if already resolved locally
        if let Some(&remote_module_id) = dir.imported_modules.read().get(&(relative_module, target))
        {
            return Ok(remote_module_id);
        }

        // check if already resolved globally
        if !is_relative {
            let global_module = self.program.modules.get(self.program.root_module_id);
            let global_module = global_module.read();
            if let Some(global_dir) = global_module.dir_base.as_ref()
                && let Some(&remote_module_id) =
                    global_dir.imported_modules.read().get(&(None, target))
            {
                dir.imported_modules
                    .write()
                    .insert((None, target), remote_module_id);
                return Ok(remote_module_id);
            }
        }

        // resolve specifier to module id (synchronous)
        let remote_module_id = self
            .resolve_specifier_to_module(target, relative_module)
            .map_err(|_| ResolveError::UnresolvedModule { node, target })?;

        // ensure the target module is bound (may yield)
        self.require_bind_module_validate(remote_module_id)
            .map_err(|e| match e {
                TaskDependencyError::NotReady { dependency } => ResolveError::Yield { dependency },
                TaskDependencyError::Failed { .. } => {
                    ResolveError::UnresolvedModule { node, target }
                }
            })?;

        // record the resolved import
        dir.imported_modules
            .write()
            .insert((relative_module, target), remote_module_id);
        Ok(remote_module_id)
    }

    /// Resolve a DependencyItem.
    pub(super) fn resolve_dependency_item(
        &self,
        module: &Module,
        dir: &ModuleDir,
        profile: ProfileId,
        item_id: LocalNodeId<DependencyItem>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
    ) -> ResolveResult<()> {
        let item = tree.get(item_id);
        let resolved_item: DependencyItem = match item {
            DependencyItem::UnresolvedRemote {
                source,
                name,
                mode,
                kind,
                alias,
                target,
                target_module: _,
                symbol,
            } => {
                let remote_module_id = self.resolve_import(
                    module,
                    dir,
                    profile,
                    item_id.into_global_any(module.id),
                    *source,
                    *target,
                )?;

                // resolve target symbol based on mode, returning (target_module_id, target_symbol)
                let (target_module_id, target_symbol) = match mode {
                    DependencyMode::Item => {
                        let key = name.map(StaticKey::Name).ok_or(
                            ResolveError::UnsupportedConstruct {
                                node: item_id.into_global_any(module.id),
                            },
                        )?;
                        self.resolve_remote_item_symbol(
                            module,
                            item_id.into_global_any(module.id),
                            remote_module_id,
                            profile,
                            key,
                        )?
                    }
                    DependencyMode::Default => {
                        let remote_module = self.program.modules.get(remote_module_id);
                        let remote_module = remote_module.read();
                        let remote_dir = remote_module.dir_base();
                        (
                            remote_module_id,
                            remote_dir.default_symbol.into_global(remote_module_id),
                        )
                    }
                    DependencyMode::Namespace => {
                        // check if this is a namespace export (namespace re-export without alias)
                        if alias.is_none() && matches!(source, DependencySource::ExportStatement) {
                            // `export * from "..."` -> register as namespace export
                            dir.namespace_exports.write().push(remote_module_id);
                        }
                        let remote_module = self.program.modules.get(remote_module_id);
                        let remote_module = remote_module.read();
                        let remote_dir = remote_module.dir_base();
                        (
                            remote_module_id,
                            remote_dir.namespace_symbol.into_global(remote_module_id),
                        )
                    }
                };

                DependencyItem::Remote {
                    mode: *mode,
                    kind: *kind,
                    name: *name,
                    alias: *alias,
                    target: *target,
                    target_module: target_module_id,
                    symbol: *symbol,
                    target_symbol,
                }
            }
            DependencyItem::UnresolvedLocal {
                mode,
                kind,
                name,
                alias,
                symbol,
            } => {
                // nothing to resolve for unnamed local exports (internal edge case)
                let Some(name_id) = name else {
                    return Ok(());
                };
                let (scope_id, scope, mark) = symbols.get_scope(item_id, tree);
                let target_symbol_id = self.resolve_absolute_symbol(
                    module,
                    item_id.into_global_any(module.id),
                    (scope_id, scope, mark),
                    StaticKey::Name(*name_id),
                    symbols,
                )?;
                DependencyItem::Local {
                    mode: *mode,
                    kind: *kind,
                    name: *name,
                    alias: *alias,
                    symbol: *symbol,
                    target_symbol: target_symbol_id.into_global(module.id),
                }
            }

            _ => {
                // nothing to do
                return Ok(());
            }
        };

        // update the target symbol of our symbol
        if let Some(symbol_id) = resolved_item.symbol()
            && let Some(target_symbol) = resolved_item.target_symbol()
        {
            symbols.get_symbol_mut(symbol_id).resolve_to(target_symbol);
        }

        *tree.get_mut(item_id) = resolved_item;
        Ok(())
    }

    /// Resolve an item symbol in a remote module, searching through namespace exports if needed.
    fn resolve_remote_item_symbol(
        &self,
        module: &Module,
        node: GlobalNodeIdAny,
        remote_module_id: ModuleId,
        profile: ProfileId,
        key: StaticKey,
    ) -> ResolveResult<(ModuleId, GlobalSymbolId)> {
        let remote_module = self.program.modules.get(remote_module_id);
        let remote_module = remote_module.read();
        let remote_dir = remote_module.dir_base();
        let remote_symbols = remote_dir.symbols.read();
        let remote_scope_id = remote_dir.namespace_scope;
        let remote_scope = remote_symbols.get_scope_by_id(remote_scope_id);

        // first try to resolve in the direct namespace
        if let Ok(local_id) = self.resolve_absolute_symbol(
            module,
            node,
            (remote_scope_id, remote_scope, LocalScopeMark::end()),
            key,
            &remote_symbols,
        ) {
            return Ok((remote_module_id, local_id.into_global(remote_module_id)));
        }

        // must drop locks before calling resolve_symbol_via_namespace_exports
        drop(remote_symbols);
        drop(remote_module);

        // search through namespace exports
        let global_symbol = self
            .resolve_symbol_via_namespace_exports(module, node, remote_module_id, profile, key)
            .map_err(|e| {
                // propagate yields, convert other errors to MissingSymbol
                if matches!(e, ResolveError::Yield { .. }) {
                    e
                } else {
                    ResolveError::MissingSymbol {
                        node,
                        scope: remote_scope_id.into_global(remote_module_id),
                        via_module: Some(remote_module_id),
                        key,
                    }
                }
            })?;

        Ok((global_symbol.module_id, global_symbol))
    }

    /// Resolve a symbol through namespace exports of a module.
    /// This is used when a symbol isn't found in the direct namespace scope.
    fn resolve_symbol_via_namespace_exports(
        &self,
        module: &Module,
        node: GlobalNodeIdAny,
        via_module_id: ModuleId,
        profile: ProfileId,
        key: StaticKey,
    ) -> ResolveResult<GlobalSymbolId> {
        self.require_resolve_module_direct_if_other(module.id, via_module_id, profile)?;
        // get the namespace exports for the via module
        let via_module = self.program.modules.get(via_module_id);
        let via_module = via_module.read();
        let via_dir = via_module.dir(profile);
        let via_namespace_scope = via_dir.namespace_scope;
        let namespace_exports: Vec<ModuleId> = via_dir.namespace_exports.read().clone();
        drop(via_module);

        // search through namespace exports (breadth-first)
        let mut visited = vec![via_module_id];
        let mut queue = namespace_exports;

        while let Some(namespace_module_id) = queue.pop() {
            // skip if already visited (prevents cycles)
            if visited.contains(&namespace_module_id) {
                continue;
            }
            visited.push(namespace_module_id);

            self.require_resolve_module_direct_if_other(module.id, namespace_module_id, profile)?;
            let namespace_module = self.program.modules.get(namespace_module_id);
            let namespace_module = namespace_module.read();
            let namespace_dir = namespace_module.dir(profile);
            let namespace_symbols = namespace_dir.symbols.read();

            // try to find the symbol in this module's namespace
            let namespace_scope_id = namespace_dir.namespace_scope;
            let namespace_scope = namespace_symbols.get_scope_by_id(namespace_scope_id);

            // check if the symbol exists and is exported in this module
            if let Some(symbol_id) = namespace_scope.find(key) {
                let symbol = namespace_symbols.get_symbol(symbol_id);
                // only consider exported symbols
                if symbol.export.is_some() {
                    return Ok(symbol_id.into_global(namespace_module_id));
                }
            }

            // add this module's namespace exports to the queue
            let nested_namespace_exports: Vec<ModuleId> =
                namespace_dir.namespace_exports.read().clone();
            queue.extend(nested_namespace_exports);
        }

        // symbol not found
        Err(ResolveError::MissingSymbol {
            node,
            scope: via_namespace_scope.into_global(via_module_id),
            via_module: Some(via_module_id),
            key,
        })
    }
}
