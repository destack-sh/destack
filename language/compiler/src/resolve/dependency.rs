use std::collections::VecDeque;

use destack_ast::StringId;
use destack_dir::{
    Declaration, DependencyItem, DependencyKind, DependencyMode, DependencySource, Export,
    ExportKind, Expression, GlobalNodeIdAny, GlobalSymbolId, LocalNodeId, LocalScopeId,
    LocalScopeMark, LocalSymbolId, ModuleTarget, NodeTree, StaticKey, SymbolSpace,
    SymbolSpaceOrder, SymbolTable,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ModuleDir, ProfileId};
use indexmap::IndexMap;

use crate::{
    Compiler, ImportError, ResolveError, ResolveResult, SymbolDescriptor, can_merge_declarations,
};

/// Target of an export assignment resolution.
/// Used when resolving named imports from modules with `export = X`.
#[derive(Debug, Clone, Copy)]
enum ExportAssignmentTarget {
    /// Redirect to another module's exports (e.g., `export = importedModule`)
    Module(ModuleTarget),
    /// Look in a namespace symbol's members (e.g., `export = LocalNamespace`)
    Namespace(GlobalSymbolId),
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Select the export spaces to consider for a dependency kind.
    fn export_spaces_for_kind(&self, kind: DependencyKind) -> SymbolSpaceOrder {
        // prefer type space for type lookups
        if kind == DependencyKind::Type {
            return SymbolSpaceOrder::TypeThenValue;
        }

        // prefer value space for value lookups
        SymbolSpaceOrder::ValueThenType
    }

    /// Resolve a symbol from a module export table.
    pub(super) fn resolve_exported_symbol(
        &self,
        module_id: ModuleId,
        exports: &IndexMap<(SymbolSpace, StaticKey), Export>,
        tree: &NodeTree,
        order: SymbolSpaceOrder,
        key: StaticKey,
    ) -> Option<GlobalSymbolId> {
        // walk export spaces in priority order
        for space in order.spaces() {
            let Some(export) = exports.get(&(*space, key)) else {
                continue;
            };

            let symbol = export.target.resolved().or_else(|| match export.kind {
                ExportKind::Local => export.symbol.map(|symbol| symbol.into_global(module_id)),
                ExportKind::ReExport => export.item.and_then(|item| tree.get(item).target_symbol()),
            });

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
        target: ModuleTarget,
        profile: ProfileId,
        kind: DependencyKind,
        key: StaticKey,
        default_name: StringId,
        visited: &mut Vec<(ModuleTarget, StaticKey, SymbolSpace)>,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        // check symbol spaces in priority order
        let order = self.export_spaces_for_kind(kind);
        for space in order.spaces() {
            let symbol = self.resolve_reexport_chain_symbol_for_space(
                origin_module_id,
                origin_symbol,
                node,
                target,
                profile,
                *space,
                key,
                default_name,
                visited,
            )?;
            if let Some(symbol) = symbol {
                // ignore value symbols for type lookups
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
        target: ModuleTarget,
        profile: ProfileId,
        space: SymbolSpace,
        key: StaticKey,
        default_name: StringId,
        visited: &mut Vec<(ModuleTarget, StaticKey, SymbolSpace)>,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        // detect cycles in reexport chains
        if visited.contains(&(target, key, space)) {
            // report cyclic symbols when an origin symbol is known
            if let Some(symbol) = origin_symbol {
                return Err(ResolveError::CyclicSymbol {
                    node: node.into_anchored(Some(profile)),
                    symbol,
                });
            }
            // report missing symbols when a chain loops without an origin symbol
            let (scope, via_module) =
                self.export_chain_scope_for_target(origin_module_id, target, profile)?;
            return Err(ResolveError::MissingSymbol {
                node: node.into_anchored(Some(profile)),
                scope,
                via_module,
                key,
            });
        }

        // record this visit for cycle detection
        visited.push((target, key, space));

        // resolve the export entry for this space and key
        let result = self.resolve_export_entry_for_target(
            origin_module_id,
            origin_symbol,
            node,
            target,
            profile,
            space,
            key,
            default_name,
            visited,
        );

        // drop the visit marker
        visited.pop();

        result
    }

    /// Get a scope for errors while resolving export chains.
    fn export_chain_scope_for_target(
        &self,
        origin_module_id: ModuleId,
        target: ModuleTarget,
        profile: ProfileId,
    ) -> ResolveResult<(destack_dir::GlobalScopeId, Option<ModuleId>)> {
        match target {
            ModuleTarget::Module(module_id) => {
                // ensure the target module is prepared
                self.require_resolve_module_prepare_if_needed(
                    origin_module_id,
                    module_id,
                    profile,
                )?;

                // use the module namespace scope for error reporting
                let module = self.program.modules.get(module_id);
                let module = module.read();
                let dir = module.dir(profile);
                Ok((dir.namespace_scope.into_global(module_id), Some(module_id)))
            }
            ModuleTarget::Binding(specifier) => {
                // load the module bindings for this specifier
                let bindings =
                    self.module_bindings_for_specifier(origin_module_id, profile, specifier)?;

                // fall back to the origin scope when bindings are missing
                let Some(bindings) = bindings else {
                    let module = self.program.modules.get(origin_module_id);
                    let module = module.read();
                    let dir = module.dir(profile);
                    return Ok((
                        dir.namespace_scope.into_global(origin_module_id),
                        Some(origin_module_id),
                    ));
                };

                // fall back to the origin scope when the list is empty
                let Some(binding_ref) = bindings.first() else {
                    let module = self.program.modules.get(origin_module_id);
                    let module = module.read();
                    let dir = module.dir(profile);
                    return Ok((
                        dir.namespace_scope.into_global(origin_module_id),
                        Some(origin_module_id),
                    ));
                };

                // ensure the binding module is prepared
                self.require_resolve_module_prepare_if_needed(
                    origin_module_id,
                    binding_ref.module_id,
                    profile,
                )?;

                // load the binding scope from the owning module
                let module = self.program.modules.get(binding_ref.module_id);
                let module = module.read();
                let dir = module.dir(profile);
                let Some((scope_id, _, _)) =
                    self.binding_info_for_declaration(dir, binding_ref.declaration)
                else {
                    return Ok((
                        dir.namespace_scope.into_global(binding_ref.module_id),
                        Some(binding_ref.module_id),
                    ));
                };

                Ok((
                    scope_id.into_global(binding_ref.module_id),
                    Some(binding_ref.module_id),
                ))
            }
        }
    }

    /// Resolve an export entry for a target to a concrete symbol.
    fn resolve_export_entry_for_target(
        &self,
        origin_module_id: ModuleId,
        origin_symbol: Option<GlobalSymbolId>,
        node: GlobalNodeIdAny,
        target: ModuleTarget,
        profile: ProfileId,
        space: SymbolSpace,
        key: StaticKey,
        default_name: StringId,
        visited: &mut Vec<(ModuleTarget, StaticKey, SymbolSpace)>,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        match target {
            ModuleTarget::Module(module_id) => {
                // ensure the target module is prepared
                self.require_resolve_module_prepare_if_needed(
                    origin_module_id,
                    module_id,
                    profile,
                )?;

                // load the module dir for this profile
                let module_ref = self.program.modules.get(module_id);
                let module_ref = module_ref.read();
                let dir = module_ref.dir(profile);

                // read the export entry for the requested key
                let export = {
                    let exports = dir.exported_symbols.read();
                    exports.get(&(space, key)).cloned()
                };
                let Some(export) = export else {
                    return Ok(None);
                };

                // resolve the export entry for this module
                self.resolve_export_entry_symbol(
                    &module_ref,
                    dir,
                    dir.namespace_scope,
                    origin_module_id,
                    origin_symbol,
                    node,
                    profile,
                    default_name,
                    export,
                    visited,
                )
            }
            ModuleTarget::Binding(specifier) => {
                // load bindings for the specifier
                let bindings =
                    self.module_bindings_for_specifier(origin_module_id, profile, specifier)?;
                let Some(bindings) = bindings else {
                    return Ok(None);
                };

                // track the first resolved symbol for conflict checks
                let mut resolved: Option<(GlobalSymbolId, Option<GlobalNodeIdAny>)> = None;
                for binding_ref in bindings {
                    // ensure the binding module is prepared
                    self.require_resolve_module_prepare_if_needed(
                        origin_module_id,
                        binding_ref.module_id,
                        profile,
                    )?;

                    // load the binding export table
                    let module_ref = self.program.modules.get(binding_ref.module_id);
                    let module_ref = module_ref.read();
                    let dir = module_ref.dir(profile);
                    let binding_exports = dir
                        .module_binding_exports
                        .read()
                        .get(&binding_ref.declaration.into_any())
                        .cloned();
                    let Some(binding_exports) = binding_exports else {
                        continue;
                    };

                    // load the binding scope and export entry
                    let Some((scope_id, _, _)) =
                        self.binding_info_for_declaration(dir, binding_ref.declaration)
                    else {
                        continue;
                    };
                    let Some(export) = binding_exports.exports.get(&(space, key)).cloned() else {
                        continue;
                    };

                    // resolve the export entry for this binding
                    let symbol = self.resolve_export_entry_symbol(
                        &module_ref,
                        dir,
                        scope_id,
                        origin_module_id,
                        origin_symbol,
                        node,
                        profile,
                        default_name,
                        export.clone(),
                        visited,
                    )?;
                    let Some(symbol) = symbol else {
                        continue;
                    };

                    // report conflicts when bindings disagree
                    if let Some((existing, other_node)) = resolved {
                        if existing != symbol {
                            let symbols = dir.symbols.read();
                            let node =
                                self.export_entry_node(binding_ref.module_id, &export, &symbols);
                            if let (Some(node), Some(other_node)) = (node, other_node) {
                                self.check_can_merge_declarations(
                                    existing,
                                    symbol,
                                    node,
                                    other_node,
                                    binding_ref.module_id,
                                    Some(key),
                                );
                            }
                            return Ok(Some(existing));
                        }
                        resolved = Some((existing, other_node));
                        continue;
                    }

                    // record the first resolved symbol and its node
                    let symbols = dir.symbols.read();
                    let node = self.export_entry_node(binding_ref.module_id, &export, &symbols);
                    resolved = Some((symbol, node));
                }

                Ok(resolved.map(|(symbol, _)| symbol))
            }
        }
    }

    /// Resolve a single export entry into a concrete symbol.
    fn resolve_export_entry_symbol(
        &self,
        module: &Module,
        dir: &ModuleDir,
        scope_id: LocalScopeId,
        origin_module_id: ModuleId,
        origin_symbol: Option<GlobalSymbolId>,
        node: GlobalNodeIdAny,
        profile: ProfileId,
        default_name: StringId,
        export: Export,
        visited: &mut Vec<(ModuleTarget, StaticKey, SymbolSpace)>,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        match export.kind {
            ExportKind::Local => {
                // return the local symbol when present
                Ok(export.symbol.map(|symbol| symbol.into_global(module.id)))
            }
            ExportKind::ReExport => {
                // load the dependency item for this export
                let Some(item_id) = export.item else {
                    return Ok(None);
                };
                let item_node = item_id.into_global_any(module.id);
                let item = {
                    let tree = dir.tree.read();
                    tree.get(item_id).clone()
                };

                // resolve the reexport target
                self.resolve_reexport_item_target(
                    module,
                    dir,
                    scope_id,
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
        }
    }

    /// Locate the scope and export symbols for a module binding declaration.
    fn binding_info_for_declaration(
        &self,
        dir: &ModuleDir,
        declaration: LocalNodeId<Declaration>,
    ) -> Option<(LocalScopeId, LocalSymbolId, LocalSymbolId)> {
        // find the binding entry for the declaration
        let bindings = dir.module_bindings.read();
        let binding = bindings
            .iter()
            .find(|binding| binding.declaration == declaration)?;

        // return the binding scope and export symbols
        Some((
            binding.scope,
            binding.default_symbol,
            binding.export_assignment_symbol,
        ))
    }

    /// Resolve a node for an export entry, if any.
    fn export_entry_node(
        &self,
        module_id: ModuleId,
        export: &Export,
        symbols: &SymbolTable,
    ) -> Option<GlobalNodeIdAny> {
        // resolve a node for the export entry
        match export.kind {
            ExportKind::Local => export
                .symbol
                .and_then(|symbol| symbols.get_symbol(symbol).primary_declaration),
            ExportKind::ReExport => export.item.map(|item| item.into_global_any(module_id)),
        }
    }

    /// Resolve a reexport item to a concrete target symbol.
    fn resolve_reexport_item_target(
        &self,
        module: &Module,
        dir: &ModuleDir,
        scope_id: LocalScopeId,
        origin_module_id: ModuleId,
        origin_symbol: Option<GlobalSymbolId>,
        node: GlobalNodeIdAny,
        profile: ProfileId,
        default_name: StringId,
        item_node: GlobalNodeIdAny,
        item: DependencyItem,
        visited: &mut Vec<(ModuleTarget, StaticKey, SymbolSpace)>,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        // resolve local or remote target symbols
        match item {
            DependencyItem::Local { target_symbol, .. }
            | DependencyItem::Remote { target_symbol, .. } => Ok(Some(target_symbol)),
            DependencyItem::UnresolvedLocal { kind, name, .. } => {
                // resolve the name from the module namespace scope
                let Some(name_id) = name else {
                    return Ok(None);
                };
                let symbols = dir.symbols.read();
                let scope = symbols.get_scope_by_id(scope_id);
                let space_order = self.export_spaces_for_kind(kind);
                let key = StaticKey::Name(name_id);

                // try resolving in the local scope first, fall back to global augmentation scope
                // (for types defined in `global { }` blocks within module declarations)
                let symbol_id = self
                    .resolve_absolute_symbol(
                        module,
                        profile,
                        item_node,
                        (scope_id, scope, LocalScopeMark::end()),
                        key,
                        space_order,
                        &symbols,
                    )
                    .or_else(|_| {
                        let global_scope_id = dir.global_augmentation_scope;
                        let global_scope = symbols.get_scope_by_id(global_scope_id);
                        self.resolve_absolute_symbol(
                            module,
                            profile,
                            item_node,
                            (global_scope_id, global_scope, LocalScopeMark::end()),
                            key,
                            space_order,
                            &symbols,
                        )
                    })?;
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
                // resolve the target module
                let target_module = if let Some(target_module) = target_module {
                    target_module
                } else {
                    self.resolve_import(module, dir, profile, item_node, source, target)?
                };

                // namespace reexports produce a namespace symbol directly
                if mode == DependencyMode::Namespace {
                    let target_symbol = self.resolve_namespace_symbol(
                        origin_module_id,
                        node,
                        target_module,
                        profile,
                    )?;
                    return Ok(Some(target_symbol));
                }

                // resolve the import key and follow the reexport chain
                let Some(target_key) = self.reexport_import_key(mode, name, default_name) else {
                    return Ok(None);
                };
                self.resolve_reexport_chain_symbol(
                    origin_module_id,
                    origin_symbol,
                    node,
                    target_module,
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

    /// Collect namespace reexport targets for a specific scope.
    fn collect_namespace_exports_for_scope(
        &self,
        module: &Module,
        profile: ProfileId,
        scope_id: LocalScopeId,
    ) -> ResolveResult<Vec<destack_dir::NamespaceExport>> {
        // load module data for export discovery
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let mut exports = Vec::new();

        // walk dependency items for namespace exports
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

            // ensure the export is in the requested scope
            let (parent_scope_id, _) = tree.get_scope(parent_id);
            if parent_scope_id != scope_id {
                continue;
            }

            // resolve the target module
            let target_module = if let Some(target_module) = target_module {
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
                module_id: target_module,
                kind,
                item: item_id,
            });
        }

        Ok(exports)
    }

    /// Collect namespace exports for a module or module binding target.
    fn collect_namespace_exports_for_target(
        &self,
        origin_module_id: ModuleId,
        target: ModuleTarget,
        profile: ProfileId,
    ) -> ResolveResult<Vec<(ModuleId, destack_dir::NamespaceExport)>> {
        match target {
            ModuleTarget::Module(module_id) => {
                // ensure the target module is prepared
                self.require_resolve_module_prepare_if_needed(
                    origin_module_id,
                    module_id,
                    profile,
                )?;

                // load namespace exports from the module scope
                let module = self.program.modules.get(module_id);
                let module = module.read();
                let dir = module.dir(profile);
                let exports = self.collect_namespace_exports_for_scope(
                    &module,
                    profile,
                    dir.namespace_scope,
                )?;

                // attach the module id to each namespace export
                Ok(exports
                    .into_iter()
                    .map(|export| (module_id, export))
                    .collect())
            }
            ModuleTarget::Binding(specifier) => {
                // load bindings for the specifier
                let bindings =
                    self.module_bindings_for_specifier(origin_module_id, profile, specifier)?;
                let Some(bindings) = bindings else {
                    return Ok(Vec::new());
                };

                // collect namespace exports from each binding scope
                let mut exports = Vec::new();
                for binding_ref in bindings {
                    // ensure the binding module is prepared
                    self.require_resolve_module_prepare_if_needed(
                        origin_module_id,
                        binding_ref.module_id,
                        profile,
                    )?;

                    // load the binding scope
                    let module = self.program.modules.get(binding_ref.module_id);
                    let module = module.read();
                    let dir = module.dir(profile);
                    let Some((scope_id, _, _)) =
                        self.binding_info_for_declaration(dir, binding_ref.declaration)
                    else {
                        continue;
                    };

                    // collect namespace exports from the binding scope
                    let nested =
                        self.collect_namespace_exports_for_scope(&module, profile, scope_id)?;
                    exports.extend(
                        nested
                            .into_iter()
                            .map(|export| (binding_ref.module_id, export)),
                    );
                }

                Ok(exports)
            }
        }
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
    ) -> ResolveResult<ModuleTarget> {
        // derive the relative module context
        let is_relative = self.is_import_relative(target);
        let relative_module = if is_relative { Some(module.id) } else { None };

        // check if already resolved locally
        if let Some(&remote_target) = dir.imported_modules.read().get(&(relative_module, target)) {
            return Ok(remote_target);
        }

        // check if already resolved "globally" (since it's not relative we can avoid re-doing the work)
        if !is_relative && !module.is_builtin() {
            self.require_resolve_module_prepare_if_needed(
                module.id,
                self.program.root_module_id,
                profile,
            )?;
            let global_module = self.program.modules.get(self.program.root_module_id);
            let global_module = global_module.read();
            if let Some(&remote_target) = global_module
                .dir(profile)
                .imported_modules
                .read()
                .get(&(None, target))
            {
                dir.imported_modules
                    .write()
                    .insert((None, target), remote_target);
                return Ok(remote_target);
            }
        }

        // check for module bindings (i.e. `declare module`)
        if let Some(binding_target) =
            self.resolve_module_binding_target(module.id, profile, target)?
        {
            dir.imported_modules
                .write()
                .insert((relative_module, target), binding_target);
            // cache globally for non-relative imports (see above)
            if !is_relative && !module.is_builtin() {
                self.require_resolve_module_prepare_if_needed(
                    module.id,
                    self.program.root_module_id,
                    profile,
                )?;
                let global_module = self.program.modules.get(self.program.root_module_id);
                let global_module = global_module.read();
                global_module
                    .dir(profile)
                    .imported_modules
                    .write()
                    .insert((None, target), binding_target);
            }
            return Ok(binding_target);
        }

        // resolve specifier to module id (synchronous!)
        // (for builtin modules, always pass source module to support specifier aliases)
        let source_module = if module.is_builtin() {
            Some(module.id)
        } else {
            relative_module
        };
        let remote_module_id = self
            .resolve_specifier_to_module(target, source_module)
            .map_err(|_| ResolveError::UnresolvedModule {
                node: node.into_anchored(Some(profile)),
                target,
            })?;

        // require module to be bound
        self.require_import_module_validate(remote_module_id)?;

        // record the resolved import
        let remote_target = ModuleTarget::Module(remote_module_id);
        dir.imported_modules
            .write()
            .insert((relative_module, target), remote_target);
        Ok(remote_target)
    }

    /// Resolve the namespace symbol for a target module.
    fn resolve_namespace_symbol(
        &self,
        origin_module_id: ModuleId,
        node: GlobalNodeIdAny,
        target: ModuleTarget,
        profile: ProfileId,
    ) -> ResolveResult<GlobalSymbolId> {
        match target {
            ModuleTarget::Module(module_id) => {
                // ensure the target module is prepared
                self.require_resolve_module_prepare_if_needed(
                    origin_module_id,
                    module_id,
                    profile,
                )?;

                // load the module namespace symbol
                let target_module = self.program.modules.get(module_id);
                let target_module = target_module.read();
                let target_dir = target_module.dir(profile);
                Ok(target_dir.namespace_symbol.into_global(module_id))
            }
            ModuleTarget::Binding(specifier) => {
                // load bindings for the specifier
                let bindings =
                    self.module_bindings_for_specifier(origin_module_id, profile, specifier)?;

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
                self.require_resolve_module_prepare_if_needed(
                    origin_module_id,
                    binding_ref.module_id,
                    profile,
                )?;

                // resolve the declaration symbol for the binding
                let module = self.program.modules.get(binding_ref.module_id);
                let module = module.read();
                let dir = module.dir(profile);
                let tree = dir.tree.read();
                let declaration = tree.get(binding_ref.declaration);
                let symbol_id = declaration.descriptor().symbol;
                Ok(symbol_id.into_global(binding_ref.module_id))
            }
        }
    }

    /// Resolve the export assignment symbol for a target module, if present.
    fn resolve_export_assignment_symbol(
        &self,
        origin_module_id: ModuleId,
        target: ModuleTarget,
        profile: ProfileId,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        match target {
            ModuleTarget::Module(module_id) => {
                // ensure the target module is prepared
                self.require_resolve_module_prepare_if_needed(
                    origin_module_id,
                    module_id,
                    profile,
                )?;

                // load the module export assignment state
                let target_module = self.program.modules.get(module_id);
                let target_module = target_module.read();
                let target_dir = target_module.dir(profile);
                if target_dir.export_assignment.read().is_some() {
                    return Ok(Some(
                        target_dir.export_assignment_symbol.into_global(module_id),
                    ));
                }
                Ok(None)
            }
            ModuleTarget::Binding(specifier) => {
                // load bindings for the specifier
                let bindings =
                    self.module_bindings_for_specifier(origin_module_id, profile, specifier)?;
                let Some(bindings) = bindings else {
                    return Ok(None);
                };

                // track the first resolved export assignment symbol
                let mut resolved: Option<(GlobalSymbolId, Option<GlobalNodeIdAny>)> = None;
                for binding_ref in bindings {
                    // ensure the binding module is prepared
                    self.require_resolve_module_prepare_if_needed(
                        origin_module_id,
                        binding_ref.module_id,
                        profile,
                    )?;

                    // load the binding export assignment entry
                    let module = self.program.modules.get(binding_ref.module_id);
                    let module = module.read();
                    let dir = module.dir(profile);
                    let binding_exports = dir
                        .module_binding_exports
                        .read()
                        .get(&binding_ref.declaration.into_any())
                        .cloned();
                    let Some(binding_exports) = binding_exports else {
                        continue;
                    };
                    let Some((_, _, export_assignment_symbol)) =
                        self.binding_info_for_declaration(dir, binding_ref.declaration)
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
        }
    }

    /// Resolve the export assignment target for a module, if present.
    /// Returns either a module to redirect to, or a namespace symbol to look inside.
    /// This is used to follow `export =` chains when resolving named imports.
    fn resolve_export_assignment_target(
        &self,
        origin_module_id: ModuleId,
        target: ModuleTarget,
        profile: ProfileId,
    ) -> ResolveResult<Option<ExportAssignmentTarget>> {
        match target {
            ModuleTarget::Module(module_id) => {
                // ensure the target module is prepared
                self.require_resolve_module_prepare_if_needed(
                    origin_module_id,
                    module_id,
                    profile,
                )?;

                // check if module has an export assignment
                let module = self.program.modules.get(module_id);
                let module = module.read();
                let dir = module.dir(profile);
                let export_assignment = *dir.export_assignment.read();
                let Some(item_id) = export_assignment else {
                    return Ok(None);
                };

                // resolve the export assignment item
                let tree = dir.tree.read();
                let item = tree.get(item_id);
                self.resolve_export_assignment_target_for_item(
                    module_id,
                    profile,
                    dir,
                    &tree,
                    dir.namespace_scope,
                    item,
                )
            }
            ModuleTarget::Binding(specifier) => {
                // load bindings for the specifier
                let bindings =
                    self.module_bindings_for_specifier(origin_module_id, profile, specifier)?;
                let Some(bindings) = bindings else {
                    return Ok(None);
                };

                // check each binding for an export assignment target
                for binding_ref in bindings {
                    // ensure the binding module is prepared
                    self.require_resolve_module_prepare_if_needed(
                        origin_module_id,
                        binding_ref.module_id,
                        profile,
                    )?;

                    // load the binding export assignment entry
                    let module = self.program.modules.get(binding_ref.module_id);
                    let module = module.read();
                    let dir = module.dir(profile);
                    let binding_exports = dir
                        .module_binding_exports
                        .read()
                        .get(&binding_ref.declaration.into_any())
                        .cloned();
                    let Some(binding_exports) = binding_exports else {
                        continue;
                    };
                    let Some(item_id) = binding_exports.export_assignment else {
                        continue;
                    };

                    // get binding scope for import lookup
                    let Some((scope_id, _, _)) =
                        self.binding_info_for_declaration(dir, binding_ref.declaration)
                    else {
                        continue;
                    };

                    // read the export assignment dependency item
                    let tree = dir.tree.read();
                    let item = tree.get(item_id);

                    // resolve the export assignment item
                    if let Some(target) = self.resolve_export_assignment_target_for_item(
                        binding_ref.module_id,
                        profile,
                        dir,
                        &tree,
                        scope_id,
                        item,
                    )? {
                        return Ok(Some(target));
                    }
                }

                Ok(None)
            }
        }
    }

    /// Resolve an export assignment target from a dependency item.
    fn resolve_export_assignment_target_for_item(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        dir: &ModuleDir,
        tree: &NodeTree,
        scope_id: LocalScopeId,
        item: &DependencyItem,
    ) -> ResolveResult<Option<ExportAssignmentTarget>> {
        // resolve the assignment target based on the dependency item
        match item {
            DependencyItem::Remote { target_module, .. } => {
                Ok(Some(ExportAssignmentTarget::Module(*target_module)))
            }
            DependencyItem::UnresolvedRemote {
                target_module: Some(target_module),
                ..
            } => Ok(Some(ExportAssignmentTarget::Module(*target_module))),
            DependencyItem::Local { target_symbol, .. } => {
                // export = localSymbol: the target could be a namespace
                Ok(Some(ExportAssignmentTarget::Namespace(*target_symbol)))
            }
            DependencyItem::Value { value, .. } => self.resolve_export_assignment_value_target(
                module_id, profile, dir, tree, scope_id, *value,
            ),
            _ => Ok(None),
        }
    }

    /// Resolve an export assignment target from a value expression.
    fn resolve_export_assignment_value_target(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        dir: &ModuleDir,
        tree: &NodeTree,
        scope_id: LocalScopeId,
        value: LocalNodeId<Expression>,
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
                && let Some(redirect) =
                    self.find_import_redirect_for_name(module_id, profile, tree, scope_id, name)?
            {
                return Ok(Some(ExportAssignmentTarget::Module(redirect)));
            }

            return Ok(Some(ExportAssignmentTarget::Namespace(*target_symbol)));
        }

        // handle unresolved paths
        if let Expression::UnresolvedPath { path, .. } = expr
            && let Some(name) = path.first_segment()
        {
            if let Some(redirect) =
                self.find_import_redirect_for_name(module_id, profile, tree, scope_id, name)?
            {
                return Ok(Some(ExportAssignmentTarget::Module(redirect)));
            }

            let symbols = dir.symbols.read();
            let key = StaticKey::Name(name);
            let symbol_id = self.find_namespace_symbol_in_scope(&symbols, scope_id, key);
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
    ) -> Option<LocalSymbolId> {
        // prefer namespace symbols for class or namespace merges
        let scope = symbols.get_scope_by_id(scope_id);
        let mut fallback = None;
        for (candidate_key, symbol_id) in &scope.named_symbols {
            if *candidate_key != key {
                continue;
            }

            let symbol = symbols.get_symbol(*symbol_id);
            if symbol.kind == destack_dir::SymbolKind::Namespace {
                return Some(*symbol_id);
            }

            if fallback.is_none() {
                fallback = Some(*symbol_id);
            }
        }

        fallback
    }

    /// Find an import redirect target for a name in a binding scope.
    /// Used to resolve `export = X` where X is an import alias.
    fn find_import_redirect_for_name(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        tree: &NodeTree,
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
                    name: item_name,
                    alias,
                    target,
                    ..
                } if matches!(
                    source,
                    DependencySource::ImportEquals | DependencySource::RequireCall
                ) && alias.or(*item_name) == Some(name) =>
                {
                    // found `import X = require("target")` where X is our name
                    // resolve module bindings before falling back to module specifiers
                    if let Some(binding_target) =
                        self.resolve_module_binding_target(module_id, profile, *target)?
                    {
                        return Ok(Some(binding_target));
                    }

                    // resolve the specifier to a module target
                    return Ok(self
                        .resolve_specifier_to_module(*target, Some(module_id))
                        .ok()
                        .map(ModuleTarget::Module));
                }
                DependencyItem::Remote {
                    mode: DependencyMode::Namespace,
                    name: item_name,
                    alias,
                    target_module,
                    ..
                } if alias.or(*item_name) == Some(name) => {
                    // already resolved import, use its target module
                    return Ok(Some(*target_module));
                }
                _ => continue,
            }
        }

        Ok(None)
    }

    /// Resolve a symbol by looking inside a namespace symbol's scope.
    /// Used for `export = LocalNamespace` where we need to find members of the namespace.
    fn resolve_symbol_in_namespace(
        &self,
        _node: GlobalNodeIdAny,
        namespace_symbol: GlobalSymbolId,
        profile: ProfileId,
        kind: DependencyKind,
        key: StaticKey,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        // load the namespace symbol's module and check if it's a namespace
        let module = self.program.modules.get(namespace_symbol.module_id);
        let module = module.read();
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();

        let symbol = symbols.get_symbol(namespace_symbol.local_id);

        // if this is a namespace, look in its scope
        if symbol.kind == destack_dir::SymbolKind::Namespace {
            let (scope_id, _) = symbol.scope;
            let scope = symbols.get_scope_by_id(scope_id);
            let space_order = self.export_spaces_for_kind(kind);

            let (preferred, fallback) =
                self.find_symbol_in_scope(scope, key, space_order, &symbols, None);

            if let Some(symbol_id) = preferred.or(fallback) {
                return Ok(Some(symbol_id.into_global(namespace_symbol.module_id)));
            }
        }

        // if not a namespace (or not found), check for merged namespace with same name
        // this handles `class Foo {} namespace Foo {}` where export = Foo points to the class
        if let Some(name) = symbol.name() {
            let symbol_key = StaticKey::Name(name);
            let (scope_id, _) = symbol.scope;
            let scope = symbols.get_scope_by_id(scope_id);

            // look for a namespace symbol with the same name in the same scope
            for (k, sym_id) in &scope.named_symbols {
                if *k == symbol_key {
                    let other_symbol = symbols.get_symbol(*sym_id);
                    if other_symbol.kind == destack_dir::SymbolKind::Namespace {
                        // found a merged namespace, look in its scope
                        let (ns_scope_id, _) = other_symbol.scope;
                        let ns_scope = symbols.get_scope_by_id(ns_scope_id);
                        let space_order = self.export_spaces_for_kind(kind);

                        let (preferred, fallback) =
                            self.find_symbol_in_scope(ns_scope, key, space_order, &symbols, None);

                        if let Some(symbol_id) = preferred.or(fallback) {
                            return Ok(Some(symbol_id.into_global(namespace_symbol.module_id)));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Resolve a dependency item.
    pub(super) fn resolve_dependency_item(
        &self,
        module: &Module,
        dir: &ModuleDir,
        profile: ProfileId,
        item_id: LocalNodeId<DependencyItem>,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> ResolveResult<Option<DependencyItem>> {
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
                // capture the origin symbol for cycle reporting
                let origin_symbol = symbol.map(|symbol| symbol.into_global(module.id));

                // resolve the target module or binding
                let remote_target = self.resolve_import(
                    module,
                    dir,
                    profile,
                    item_id.into_global_any(module.id),
                    *source,
                    *target,
                )?;

                // resolve target symbol based on mode
                let target_symbol = match mode {
                    DependencyMode::Item => {
                        // resolve a named import from the target
                        let key = name.map(StaticKey::Name).ok_or(
                            ResolveError::UnsupportedConstruct {
                                node: item_id
                                    .into_global_any(module.id)
                                    .into_anchored(Some(profile)),
                            },
                        )?;
                        self.resolve_remote_item_symbol(
                            module,
                            item_id.into_global_any(module.id),
                            remote_target,
                            profile,
                            *kind,
                            origin_symbol,
                            key,
                        )?
                    }
                    DependencyMode::Default => {
                        // resolve the default export from the target
                        let default_name = self.program.strings.intern("default");
                        let key = StaticKey::Name(default_name);
                        self.resolve_remote_item_symbol(
                            module,
                            item_id.into_global_any(module.id),
                            remote_target,
                            profile,
                            *kind,
                            origin_symbol,
                            key,
                        )?
                    }
                    DependencyMode::Namespace => {
                        // prefer export assignment for import equals
                        if *source == DependencySource::ImportEquals {
                            if let Some(symbol) = self.resolve_export_assignment_symbol(
                                module.id,
                                remote_target,
                                profile,
                            )? {
                                symbol
                            } else {
                                self.resolve_namespace_symbol(
                                    module.id,
                                    item_id.into_global_any(module.id),
                                    remote_target,
                                    profile,
                                )?
                            }
                        } else {
                            // check for namespace exports without alias
                            if alias.is_none()
                                && matches!(source, DependencySource::ExportStatement)
                            {
                                // register `export * from` in module scope
                                let (item_scope_id, _) = tree.get_scope(item_id);
                                if item_scope_id == dir.namespace_scope {
                                    dir.namespace_exports.write().push(
                                        destack_dir::NamespaceExport {
                                            module_id: remote_target,
                                            kind: *kind,
                                            item: item_id,
                                        },
                                    );
                                }
                            }
                            // resolve the namespace symbol
                            self.resolve_namespace_symbol(
                                module.id,
                                item_id.into_global_any(module.id),
                                remote_target,
                                profile,
                            )?
                        }
                    }
                };

                DependencyItem::Remote {
                    mode: *mode,
                    kind: *kind,
                    name: *name,
                    alias: *alias,
                    target: *target,
                    target_module: remote_target,
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
                    return Ok(None);
                };

                // resolve the local symbol in scope
                let (scope_id, scope, mark) = symbols.get_scope(item_id, tree);
                let mark = if self.export_item_parent(tree, item_id).is_some() {
                    // export specifiers can reference later declarations
                    LocalScopeMark::end()
                } else {
                    mark
                };
                let space_order = self.export_spaces_for_kind(*kind);
                let key = StaticKey::Name(*name_id);
                let node = item_id.into_global_any(module.id);

                // try resolving in the local scope first
                let target_symbol_id = self
                    .resolve_absolute_symbol(
                        module,
                        profile,
                        node,
                        (scope_id, scope, mark),
                        key,
                        space_order,
                        symbols,
                    )
                    // fallback to global augmentation scope for types like AllowSharedBuffer
                    // that are defined in `global { }` blocks within module declarations
                    .or_else(|_| {
                        let global_scope_id = dir.global_augmentation_scope;
                        let global_scope = symbols.get_scope_by_id(global_scope_id);
                        self.resolve_absolute_symbol(
                            module,
                            profile,
                            node,
                            (global_scope_id, global_scope, LocalScopeMark::end()),
                            key,
                            space_order,
                            symbols,
                        )
                    })?;

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
                return Ok(None);
            }
        };

        Ok(Some(resolved_item))
    }

    /// Resolve an item symbol in a remote module, searching through namespace exports if needed.
    fn resolve_remote_item_symbol(
        &self,
        module: &Module,
        node: GlobalNodeIdAny,
        remote_target: ModuleTarget,
        profile: ProfileId,
        kind: DependencyKind,
        origin_symbol: Option<GlobalSymbolId>,
        key: StaticKey,
    ) -> ResolveResult<GlobalSymbolId> {
        let default_name = self.program.strings.intern("default");

        // resolve explicit exports and reexport chains first
        let mut visited = Vec::new();
        let resolved_symbol = self.resolve_reexport_chain_symbol(
            module.id,
            origin_symbol,
            node,
            remote_target,
            profile,
            kind,
            key,
            default_name,
            &mut visited,
        )?;
        if let Some(symbol_id) = resolved_symbol {
            return Ok(symbol_id);
        }

        // fall back to export assignment target (e.g., `export = X`)
        // handles both module redirects and local namespace lookups
        if let Some(target) =
            self.resolve_export_assignment_target(module.id, remote_target, profile)?
        {
            match target {
                ExportAssignmentTarget::Module(redirect_target) => {
                    // recursively resolve the symbol in the redirected module
                    return self.resolve_remote_item_symbol(
                        module,
                        node,
                        redirect_target,
                        profile,
                        kind,
                        origin_symbol,
                        key,
                    );
                }
                ExportAssignmentTarget::Namespace(namespace_symbol) => {
                    // look up the key in the namespace symbol's scope
                    if let Some(symbol) = self.resolve_symbol_in_namespace(
                        node,
                        namespace_symbol,
                        profile,
                        kind,
                        key,
                    )? {
                        return Ok(symbol);
                    }
                }
            }
        }

        // fall back to namespace exports
        let global_symbol = self.resolve_symbol_via_namespace_exports(
            module,
            node,
            remote_target,
            profile,
            kind,
            key,
            origin_symbol,
            default_name,
        )?;

        Ok(global_symbol)
    }

    /// Resolve a symbol through namespace exports of a module.
    /// This is used when a symbol isn't found in the direct namespace scope.
    fn resolve_symbol_via_namespace_exports(
        &self,
        module: &Module,
        node: GlobalNodeIdAny,
        via_target: ModuleTarget,
        profile: ProfileId,
        kind: DependencyKind,
        key: StaticKey,
        origin_symbol: Option<GlobalSymbolId>,
        default_name: StringId,
    ) -> ResolveResult<GlobalSymbolId> {
        // resolve the scope for missing symbol errors
        let (via_scope, via_module_id) =
            self.export_chain_scope_for_target(module.id, via_target, profile)?;

        // collect namespace exports from export * statements
        let namespace_exports =
            self.collect_namespace_exports_for_target(module.id, via_target, profile)?;
        if namespace_exports.is_empty() {
            return Err(ResolveError::MissingSymbol {
                node: node.into_anchored(Some(profile)),
                scope: via_scope,
                via_module: via_module_id,
                key,
            });
        }

        // seed the namespace export queue
        let mut found: Option<(GlobalSymbolId, GlobalNodeIdAny)> = None;
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
            let mut visited_exports = Vec::new();
            if let Some(symbol_id) = self.resolve_reexport_chain_symbol(
                module.id,
                origin_symbol,
                node,
                namespace_target,
                profile,
                kind,
                key,
                default_name,
                &mut visited_exports,
            )? {
                match found {
                    None => {
                        found = Some((symbol_id, origin_item.into_global_any(source_module_id)));
                    }
                    Some((existing, other_node)) => {
                        if existing != symbol_id {
                            let node = origin_item.into_global_any(source_module_id);
                            self.check_can_merge_declarations(
                                existing,
                                symbol_id,
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
            let nested_exports =
                self.collect_namespace_exports_for_target(module.id, namespace_target, profile)?;
            for (nested_source_module_id, nested) in nested_exports {
                if self.namespace_export_allows_kind(nested.kind, kind) {
                    queue.push_back((nested_source_module_id, nested.module_id, nested.item));
                }
            }
        }

        // return the resolved symbol if any
        if let Some((symbol_id, _)) = found {
            return Ok(symbol_id);
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
        export_kind == DependencyKind::Value
    }

    /// Report a conflicting export error if two symbols cannot merge.
    fn check_can_merge_declarations(
        &self,
        left: GlobalSymbolId,
        right: GlobalSymbolId,
        node: GlobalNodeIdAny,
        other_node: GlobalNodeIdAny,
        module: ModuleId,
        name: Option<StaticKey>,
    ) {
        let left_module = self.program.modules.get(left.module_id);
        let left_module = left_module.read();
        let left_symbols = left_module.dir_base().symbols.read();
        let left_symbol = left_symbols.get_symbol(left.local_id);

        let right_module = self.program.modules.get(right.module_id);
        let right_module = right_module.read();
        let right_symbols = right_module.dir_base().symbols.read();
        let right_symbol = right_symbols.get_symbol(right.local_id);

        // check if the symbols can merge (e.g., interface + class)
        let language_type = left_module.language_type;
        let can_merge = can_merge_declarations(
            language_type,
            SymbolDescriptor::from(left_symbol),
            SymbolDescriptor::from(right_symbol),
        );

        // NOTE #Suspicious: should we separate export conflicts per profile? (from ImportError)
        // (also see other usages of ImportError::Conflicting* across resolve)
        if !can_merge {
            self.error(ImportError::ConflictingExport {
                node: node.into(),
                other_node: other_node.into(),
                module,
                name,
            });
        }
    }
}
