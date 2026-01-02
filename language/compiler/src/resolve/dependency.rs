use std::collections::VecDeque;

use destack_ast::StringId;
use destack_dir::{
    DependencyItem, DependencyKind, DependencyMode, DependencySource, Export, ExportKind,
    ExportSpaceOrder, Expression, GlobalNodeIdAny, GlobalSymbolId, LocalNodeId, LocalScopeMark,
    NodeTree, StaticKey, SymbolSpace, SymbolTable,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ModuleDir, ProfileId};
use indexmap::IndexMap;

use crate::{BindError, Compiler, ResolveError, ResolveResult, TaskDependencyError};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Select the export spaces to consider for a dependency kind.
    fn export_spaces_for_kind(&self, kind: DependencyKind) -> ExportSpaceOrder {
        // prefer type space for type lookups
        if kind == DependencyKind::Type {
            return ExportSpaceOrder::TypeThenValue;
        }

        // prefer value space for value lookups
        ExportSpaceOrder::ValueThenType
    }

    /// Resolve a symbol from a module export table.
    pub(super) fn resolve_exported_symbol(
        &self,
        module_id: ModuleId,
        exports: &IndexMap<(SymbolSpace, StaticKey), Export>,
        tree: &NodeTree,
        order: ExportSpaceOrder,
        key: StaticKey,
    ) -> Option<GlobalSymbolId> {
        // walk export spaces in priority order
        for space in order.spaces() {
            let Some(export) = exports.get(&(*space, key)) else {
                continue;
            };

            let symbol = match export.kind {
                ExportKind::Local => export.symbol.map(|symbol| symbol.into_global(module_id)),
                ExportKind::ReExport => export.item.and_then(|item| tree.get(item).target_symbol()),
            };

            // return the first matching symbol
            if let Some(symbol) = symbol {
                return Some(symbol);
            }
        }

        None
    }

    /// Get the import key for a reexported dependency item.
    fn reexport_import_key(
        &self,
        mode: DependencyMode,
        name: Option<StringId>,
        default_name: StringId,
    ) -> Option<StaticKey> {
        // map export modes to import keys
        match mode {
            DependencyMode::Item => name.map(StaticKey::Name),
            DependencyMode::Default => Some(StaticKey::Name(default_name)),
            DependencyMode::Namespace => None,
        }
    }

    /// Resolve an export entry by walking reexport chains.
    fn resolve_reexport_chain_symbol(
        &self,
        origin_module_id: ModuleId,
        origin_symbol: Option<GlobalSymbolId>,
        node: GlobalNodeIdAny,
        module_id: ModuleId,
        profile: ProfileId,
        kind: DependencyKind,
        key: StaticKey,
        default_name: StringId,
        visited: &mut Vec<(ModuleId, StaticKey, SymbolSpace)>,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        // check spaces in priority order
        let order = self.export_spaces_for_kind(kind);
        for space in order.spaces() {
            let symbol = self.resolve_reexport_chain_symbol_for_space(
                origin_module_id,
                origin_symbol,
                node,
                module_id,
                profile,
                *space,
                key,
                default_name,
                visited,
            )?;
            if let Some(symbol) = symbol {
                // ignore value only symbols for type lookups
                if kind == DependencyKind::Type {
                    let symbol_space = self.symbol_space_for_global(profile, symbol);
                    if symbol_space == SymbolSpace::Value {
                        continue;
                    }
                }
                return Ok(Some(symbol));
            }
        }

        Ok(None)
    }

    /// Resolve an export entry for a specific symbol space.
    fn resolve_reexport_chain_symbol_for_space(
        &self,
        origin_module_id: ModuleId,
        origin_symbol: Option<GlobalSymbolId>,
        node: GlobalNodeIdAny,
        module_id: ModuleId,
        profile: ProfileId,
        space: SymbolSpace,
        key: StaticKey,
        default_name: StringId,
        visited: &mut Vec<(ModuleId, StaticKey, SymbolSpace)>,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        // prepare is enough for export lookup
        self.require_resolve_module_prepare_if_needed(origin_module_id, module_id, profile)?;

        // read the module dir for this profile
        let module_ref = self.program.modules.get(module_id);
        let module_ref = module_ref.read();
        let dir = module_ref.dir(profile);

        // detect cycles in reexport chains
        if visited.contains(&(module_id, key, space)) {
            if let Some(symbol) = origin_symbol {
                return Err(ResolveError::CyclicSymbol { node, symbol });
            }
            return Err(ResolveError::MissingSymbol {
                node,
                scope: dir.namespace_scope.into_global(module_id),
                via_module: Some(module_id),
                key,
            });
        }

        // record this visit for cycle detection
        visited.push((module_id, key, space));

        // read the export entry for this space and key
        let export = {
            let exports = dir.exported_symbols.read();
            exports.get(&(space, key)).cloned()
        };

        // resolve the export entry
        let result = match export {
            None => Ok(None),
            Some(export) if export.kind == ExportKind::Local => {
                Ok(export.symbol.map(|symbol| symbol.into_global(module_id)))
            }
            Some(export) if export.kind == ExportKind::ReExport => {
                // copy the dependency item so we can drop the tree lock
                let Some(item_id) = export.item else {
                    return Ok(None);
                };
                let item_node = item_id.into_global_any(module_id);
                let item = {
                    let tree = dir.tree.read();
                    tree.get(item_id).clone()
                };
                self.resolve_reexport_item_target(
                    &module_ref,
                    dir,
                    origin_module_id,
                    origin_symbol,
                    node,
                    profile,
                    default_name,
                    item_node,
                    item,
                    visited,
                )
            }
            _ => Ok(None),
        };

        // drop the visit marker
        visited.pop();
        result
    }

    /// Resolve a reexport item to a concrete target symbol.
    fn resolve_reexport_item_target(
        &self,
        module: &Module,
        dir: &ModuleDir,
        origin_module_id: ModuleId,
        origin_symbol: Option<GlobalSymbolId>,
        node: GlobalNodeIdAny,
        profile: ProfileId,
        default_name: StringId,
        item_node: GlobalNodeIdAny,
        item: DependencyItem,
        visited: &mut Vec<(ModuleId, StaticKey, SymbolSpace)>,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        // resolve local or remote target symbols
        match item {
            DependencyItem::Local { target_symbol, .. }
            | DependencyItem::Remote { target_symbol, .. } => Ok(Some(target_symbol)),
            DependencyItem::UnresolvedLocal { name, .. } => {
                // resolve the name from the module namespace scope
                let Some(name_id) = name else {
                    return Ok(None);
                };
                let symbols = dir.symbols.read();
                let scope = symbols.get_scope_by_id(dir.namespace_scope);
                let symbol_id = self.resolve_absolute_symbol(
                    module,
                    item_node,
                    (dir.namespace_scope, scope, LocalScopeMark::end()),
                    StaticKey::Name(name_id),
                    &symbols,
                )?;
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
                // resolve the target module id
                let target_module_id = if let Some(target_module) = target_module {
                    target_module
                } else {
                    self.resolve_import(module, dir, profile, item_node, source, target)?
                };

                // namespace reexports produce a namespace symbol directly
                if mode == DependencyMode::Namespace {
                    self.require_resolve_module_prepare_if_needed(
                        origin_module_id,
                        target_module_id,
                        profile,
                    )?;
                    let target_module = self.program.modules.get(target_module_id);
                    let target_module = target_module.read();
                    let target_dir = target_module.dir(profile);
                    return Ok(Some(
                        target_dir.namespace_symbol.into_global(target_module_id),
                    ));
                }

                // resolve the import key and follow the reexport chain
                let Some(target_key) = self.reexport_import_key(mode, name, default_name) else {
                    return Ok(None);
                };
                self.resolve_reexport_chain_symbol(
                    origin_module_id,
                    origin_symbol,
                    node,
                    target_module_id,
                    profile,
                    kind,
                    target_key,
                    default_name,
                    visited,
                )
            }
            DependencyItem::Value { .. } => Ok(None),
        }
    }

    /// Collect namespace reexport targets for a module.
    fn collect_namespace_exports(
        &self,
        module: &Module,
        profile: ProfileId,
    ) -> ResolveResult<Vec<destack_dir::NamespaceExport>> {
        // load module data for export discovery
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let mut exports = Vec::new();

        for item_id in tree.iter_node_ids_of_type::<DependencyItem>() {
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
                _ => continue,
            };

            // namespace export statements must be export * from without alias
            if mode != DependencyMode::Namespace || alias.is_some() {
                continue;
            }
            if let Some(source) = source
                && source != DependencySource::ExportStatement
            {
                continue;
            }

            // ensure this dependency item belongs to an export expression
            let Some(parent_id) = tree.get_parent(item_id.id) else {
                continue;
            };
            let Ok(parent_id) = parent_id.try_into_typed::<Expression>() else {
                continue;
            };
            if !matches!(
                tree.get(parent_id),
                Expression::Export { .. }
                    | Expression::UnresolvedReExport { .. }
                    | Expression::ReExport { .. }
            ) {
                continue;
            }

            // resolve the target module id
            let target_module_id = if let Some(target_module) = target_module {
                target_module
            } else if let Some(target) = target {
                self.resolve_import(
                    module,
                    dir,
                    profile,
                    item_id.into_global_any(module.id),
                    source.unwrap_or(DependencySource::ExportStatement),
                    target,
                )?
            } else {
                continue;
            };

            // register the namespace export edge
            exports.push(destack_dir::NamespaceExport {
                module_id: target_module_id,
                kind,
                item: item_id,
            });
        }

        Ok(exports)
    }

    /// Whether the target is a relative import.
    pub(super) fn is_import_relative(&self, target: StringId) -> bool {
        // check relative prefix markers
        let target_str = self.program.strings.get(target);
        target_str.starts_with("./") || target_str.starts_with("../")
    }

    /// Try to resolve an import of some target specifier synchronously.
    pub(super) fn resolve_import(
        &self,
        module: &Module,
        dir: &ModuleDir,
        profile: ProfileId,
        node: GlobalNodeIdAny,
        _source: DependencySource,
        target: StringId,
    ) -> ResolveResult<ModuleId> {
        // derive the relative module context
        let is_relative = self.is_import_relative(target);
        let relative_module = if is_relative { Some(module.id) } else { None };

        // check if already resolved locally
        if let Some(&remote_module_id) = dir.imported_modules.read().get(&(relative_module, target))
        {
            return Ok(remote_module_id);
        }

        // check if already resolved globally
        if !is_relative {
            self.require_resolve_module_prepare_if_needed(
                module.id,
                self.program.root_module_id,
                profile,
            )?;
            let global_module = self.program.modules.get(self.program.root_module_id);
            let global_module = global_module.read();
            if let Some(&remote_module_id) = global_module
                .dir(profile)
                .imported_modules
                .read()
                .get(&(None, target))
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

    /// Resolve the namespace symbol for a target module.
    fn resolve_namespace_symbol(
        &self,
        origin_module_id: ModuleId,
        target_module_id: ModuleId,
        profile: ProfileId,
    ) -> ResolveResult<(ModuleId, GlobalSymbolId)> {
        // ensure the target module is prepared
        self.require_resolve_module_prepare_if_needed(origin_module_id, target_module_id, profile)?;
        let target_module = self.program.modules.get(target_module_id);
        let target_module = target_module.read();
        let target_dir = target_module.dir(profile);
        Ok((
            target_module_id,
            target_dir.namespace_symbol.into_global(target_module_id),
        ))
    }

    /// Resolve the export assignment symbol for a target module, if present.
    fn resolve_export_assignment_symbol(
        &self,
        origin_module_id: ModuleId,
        target_module_id: ModuleId,
        profile: ProfileId,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        // ensure the target module is prepared
        self.require_resolve_module_prepare_if_needed(origin_module_id, target_module_id, profile)?;
        let target_module = self.program.modules.get(target_module_id);
        let target_module = target_module.read();
        let target_dir = target_module.dir(profile);
        if target_dir.export_assignment.read().is_some() {
            return Ok(Some(
                target_dir
                    .export_assignment_symbol
                    .into_global(target_module_id),
            ));
        }
        Ok(None)
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
        // resolve the dependency item based on its mode
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
                let origin_symbol = symbol.map(|symbol| symbol.into_global(module.id));
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
                            *kind,
                            origin_symbol,
                            key,
                        )?
                    }
                    DependencyMode::Default => {
                        let default_name = self.program.strings.intern("default");
                        let key = StaticKey::Name(default_name);
                        self.resolve_remote_item_symbol(
                            module,
                            item_id.into_global_any(module.id),
                            remote_module_id,
                            profile,
                            *kind,
                            origin_symbol,
                            key,
                        )?
                    }
                    DependencyMode::Namespace => {
                        if *source == DependencySource::ImportEquals {
                            if let Some(symbol) = self.resolve_export_assignment_symbol(
                                module.id,
                                remote_module_id,
                                profile,
                            )? {
                                (remote_module_id, symbol)
                            } else {
                                self.resolve_namespace_symbol(module.id, remote_module_id, profile)?
                            }
                        } else {
                            // check if this is a namespace export, namespace reexport without alias
                            if alias.is_none()
                                && matches!(source, DependencySource::ExportStatement)
                            {
                                // `export * from "..."` so register as namespace export
                                dir.namespace_exports
                                    .write()
                                    .push(destack_dir::NamespaceExport {
                                        module_id: remote_module_id,
                                        kind: *kind,
                                        item: item_id,
                                    });
                            }
                            self.resolve_namespace_symbol(module.id, remote_module_id, profile)?
                        }
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
        kind: DependencyKind,
        origin_symbol: Option<GlobalSymbolId>,
        key: StaticKey,
    ) -> ResolveResult<(ModuleId, GlobalSymbolId)> {
        let default_name = self.program.strings.intern("default");

        // resolve explicit exports and reexport chains first
        let mut visited = Vec::new();
        let resolved_symbol = self.resolve_reexport_chain_symbol(
            module.id,
            origin_symbol,
            node,
            remote_module_id,
            profile,
            kind,
            key,
            default_name,
            &mut visited,
        )?;
        if let Some(symbol_id) = resolved_symbol {
            return Ok((symbol_id.module_id, symbol_id));
        }

        // fall back to namespace exports
        let global_symbol = self.resolve_symbol_via_namespace_exports(
            module,
            node,
            remote_module_id,
            profile,
            kind,
            key,
            origin_symbol,
            default_name,
        )?;

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
        kind: DependencyKind,
        key: StaticKey,
        origin_symbol: Option<GlobalSymbolId>,
        default_name: StringId,
    ) -> ResolveResult<GlobalSymbolId> {
        // ensure the module is prepared before reading exports
        self.require_resolve_module_prepare_if_needed(module.id, via_module_id, profile)?;

        // read the target module namespace scope
        let via_module = self.program.modules.get(via_module_id);
        let via_module = via_module.read();
        let via_dir = via_module.dir(profile);
        let via_namespace_scope = via_dir.namespace_scope;

        // collect namespace exports from export * statements
        let namespace_exports = self.collect_namespace_exports(&via_module, profile)?;
        if namespace_exports.is_empty() {
            return Err(ResolveError::MissingSymbol {
                node,
                scope: via_namespace_scope.into_global(via_module_id),
                via_module: Some(via_module_id),
                key,
            });
        }

        // seed the namespace export queue
        let mut found: Option<(GlobalSymbolId, LocalNodeId<DependencyItem>)> = None;
        let mut visited = Vec::new();
        let mut queue = VecDeque::new();

        // enqueue top level namespace exports
        for export in namespace_exports {
            if self.namespace_export_allows_kind(export.kind, kind) {
                queue.push_back((export.module_id, export.item));
            }
        }

        // walk the namespace export graph
        while let Some((namespace_module_id, origin_item)) = queue.pop_front() {
            // skip cycles on namespace exports
            if visited.contains(&(namespace_module_id, origin_item)) {
                continue;
            }
            visited.push((namespace_module_id, origin_item));

            // resolve explicit exports within the namespace target
            let mut visited_exports = Vec::new();
            if let Some(symbol_id) = self.resolve_reexport_chain_symbol(
                module.id,
                origin_symbol,
                node,
                namespace_module_id,
                profile,
                kind,
                key,
                default_name,
                &mut visited_exports,
            )? {
                match found {
                    None => found = Some((symbol_id, origin_item)),
                    Some((existing, other_item)) => {
                        if existing != symbol_id {
                            let other_node = other_item.into_global_any(via_module_id);
                            let node = origin_item.into_global_any(via_module_id);
                            self.error(BindError::ConflictingExport {
                                node,
                                other_node,
                                module: via_module_id,
                                name: Some(key),
                            });
                            return Ok(existing);
                        }
                    }
                }
            }

            // enqueue nested namespace exports
            let namespace_module = self.program.modules.get(namespace_module_id);
            let namespace_module = namespace_module.read();
            let nested_exports = self.collect_namespace_exports(&namespace_module, profile)?;
            for nested in nested_exports {
                if self.namespace_export_allows_kind(nested.kind, kind) {
                    queue.push_back((nested.module_id, origin_item));
                }
            }
        }

        // return the resolved symbol if any
        if let Some((symbol_id, _)) = found {
            return Ok(symbol_id);
        }

        // report a missing symbol for namespace lookup failures
        Err(ResolveError::MissingSymbol {
            node,
            scope: via_namespace_scope.into_global(via_module_id),
            via_module: Some(via_module_id),
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
        export_kind == DependencyKind::Value
    }
}
