use std::collections::HashSet;

use crate::{Compiler, ImportError, ResolveError, ResolveResult, TaskResultCollector};
use destack_builtin::builtin_lib;
use destack_dir::{
    Declaration, DependencyItem, DependencyKind, DependencyMode, Export, ExportKind, Expression,
    GlobalNodeIdAny, GlobalSymbolId, LocalNodeId, LocalScopeId, LocalSymbolId, ModuleBinding,
    ModuleBindingExports, NodeTree, StaticKey, SymbolSpace, SymbolSpaceOrder, SymbolTable,
};

use destack_source::ModuleId;
use destack_workspace::{ImportMeta, Module, ModuleDir, ProfileId};
use indexmap::IndexMap;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Prepare the per profile DIR by cloning from the base DIR.
    pub(super) fn resolve_module_prepare(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ResolveResult<()> {
        // resolve libs if needed
        if self.options.load_libs {
            let module = self.program.modules.get(module_id);
            let module = module.read();
            if module.is_user() {
                self.require_resolve_libs(profile_id)?;
            }
        }

        // load the module and skip when the profile dir already exists
        let module = self.program.modules.get(module_id);
        let mut module = module.write();
        if module
            .code()
            .dirs
            .iter()
            .any(|dir| dir.profile_id == Some(profile_id))
        {
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

        // build export table from bound declarations
        let dir = module.dir(profile_id);
        let tree = dir.tree.read();
        let mut symbols = dir.symbols.write();
        self.build_module_exports(&module, dir, &tree, &mut symbols);
        self.build_module_binding_exports(&module, dir, &tree, &mut symbols);
        Ok(())
    }

    /// Resolve expressions, dependencies, and declarations (phase 1).
    pub(super) fn resolve_module_direct(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> ResolveResult<()> {
        // load module data for read
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);

        // resolve module dependency expressions
        {
            let mut tree = dir.tree.write();
            let mut symbols = dir.symbols.write();
            let mut collector = TaskResultCollector::new();
            for expression_id in tree.iter_node_ids_of_type::<Expression>() {
                match tree.get(expression_id) {
                    Expression::UnresolvedImport { .. } | Expression::UnresolvedReExport { .. } => {
                        self.collect(
                            &mut collector,
                            self.resolve_expression(
                                &module,
                                dir,
                                profile,
                                expression_id,
                                &mut tree,
                                &mut symbols,
                            ),
                        );
                    }
                    _ => {}
                }
            }
            if let Some(dependency) = collector.try_into_yield_any() {
                return Err(ResolveError::Yield { dependency });
            }
        }

        // resolve dependencies
        self.resolve_dependency_items(module_id, profile)?;

        // build the global symbol table (after dependency resolution)
        self.require_global_symbol_cache(module.id, profile)?;

        // resolve expressions
        {
            let mut tree = dir.tree.write();
            let mut symbols = dir.symbols.write();
            let mut collector = TaskResultCollector::new();
            for expression_id in tree.iter_node_ids_of_type::<Expression>() {
                self.collect(
                    &mut collector,
                    self.resolve_expression(
                        &module,
                        dir,
                        profile,
                        expression_id,
                        &mut tree,
                        &mut symbols,
                    ),
                );
            }
            if let Some(dependency) = collector.try_into_yield_any() {
                return Err(ResolveError::Yield { dependency });
            }
        }

        // resolve declarations (e.g., extensions, types/aliases)
        {
            let mut tree = dir.tree.write();
            let mut symbols = dir.symbols.write();
            let mut collector = TaskResultCollector::new();
            for declaration_id in tree.iter_node_ids_of_type::<Declaration>() {
                self.collect(
                    &mut collector,
                    self.resolve_declaration(&module, dir, declaration_id, &mut tree, &mut symbols),
                );
            }
            if let Some(dependency) = collector.try_into_yield_any() {
                return Err(ResolveError::Yield { dependency });
            }
        }

        // finalize export targets (after dependency resolution)
        {
            let tree = dir.tree.read();
            let mut symbols = dir.symbols.write();
            self.finalize_module_exports(dir, &tree, &mut symbols);
            self.finalize_module_binding_exports(dir, &tree, &mut symbols);
        }

        Ok(())
    }

    /// Resolve dependency items (imports/reexports) for a module.
    pub(super) fn resolve_dependency_items(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> ResolveResult<()> {
        // load module data for read
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);
        let item_ids = {
            let tree = dir.tree.read();
            tree.iter_node_ids_of_type::<DependencyItem>()
        };

        // resolve dependency items
        let mut collector = TaskResultCollector::new();
        for item_id in item_ids {
            // resolve the dependency item (with read locks)
            let resolved_item = {
                let tree = dir.tree.read();
                let symbols = dir.symbols.read();
                self.resolve_dependency_item(&module, dir, profile, item_id, &tree, &symbols)
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

            // apply resolved dependency updates (with write locks)
            if let Some(resolved_item) = resolved_item {
                let mut tree = dir.tree.write();
                let mut symbols = dir.symbols.write();
                if let Some(symbol_id) = resolved_item.symbol()
                    && let Some(target_symbol) = resolved_item.target_symbol()
                {
                    symbols.get_symbol_mut(symbol_id).resolve_to(target_symbol);
                }

                *tree.get_mut(item_id) = resolved_item;
            }
        }

        // yield unresolved dependency items (after collection)
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
    ) {
        // cache the default export name
        let default_name = self.program.strings.intern("default");

        // build the binding export table
        let bindings = dir.module_bindings.read().clone();
        let mut binding_exports = IndexMap::new();
        for binding in bindings {
            // collect the export assignment if present
            let export_assignment_item =
                self.collect_binding_export_assignment(module.id, &binding, tree);

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
        binding: &ModuleBinding,
        tree: &NodeTree,
    ) -> Option<LocalNodeId<DependencyItem>> {
        // scan export statements in the binding scope
        let mut export_assignment_item: Option<LocalNodeId<DependencyItem>> = None;
        for item_id in tree.iter_node_ids_of_type::<DependencyItem>() {
            // skip items outside the binding scope
            if !self.dependency_item_in_scope(tree, item_id, binding.scope) {
                continue;
            }

            // skip nonexport statements
            if self.export_statement_parent(tree, item_id).is_none() {
                continue;
            }

            // skip nonassignment values
            let DependencyItem::Value { mode, .. } = tree.get(item_id) else {
                continue;
            };
            if *mode != DependencyMode::Namespace {
                continue;
            }

            // report conflicts and keep the first assignment
            if let Some(existing) = export_assignment_item {
                self.error(ImportError::ConflictingExport {
                    node: item_id.into_global_any(module_id).into(),
                    other_node: existing.into_global_any(module_id).into(),
                    module: module_id,
                    name: None,
                });
                continue;
            }

            export_assignment_item = Some(item_id);
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
        // collect symbols declared in the binding scope
        let scope = symbols.get_scope_by_id(binding.scope);
        let symbol_ids: Vec<LocalSymbolId> = scope
            .named_symbols
            .iter()
            .map(|(_, symbol_id)| *symbol_id)
            .chain(
                scope
                    .anonymous_symbols
                    .iter()
                    .copied()
                    .filter(|symbol_id| *symbol_id != binding.default_symbol),
            )
            .collect();

        let mut seen_exports = HashSet::new();
        for symbol_id in symbol_ids {
            // read symbol metadata
            let symbol = symbols.get_symbol(symbol_id);
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
    ) {
        // walk dependency items under export expressions in the binding scope
        for item_id in tree.iter_node_ids_of_type::<DependencyItem>() {
            // skip dependency items outside the binding scope
            if !self.dependency_item_in_scope(tree, item_id, binding.scope) {
                continue;
            }

            // skip nonexport dependency items
            if self.export_item_parent(tree, item_id).is_none() {
                continue;
            }

            // reject exports when export assignment is present
            if let Some(export_assignment_item) = export_assignment_item
                && export_assignment_item != item_id
            {
                self.error(ImportError::ConflictingExport {
                    node: item_id.into_global_any(module_id).into(),
                    other_node: export_assignment_item.into_global_any(module_id).into(),
                    module: module_id,
                    name: None,
                });
                continue;
            }

            // extract export metadata from the dependency item
            let item = tree.get(item_id);
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
                            Some(item_id.into_global_any(module_id)),
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
                let export = Export::reexport(key, *space, item_id);
                self.insert_exports(
                    module_id,
                    symbols,
                    exports,
                    export,
                    Some(item_id.into_global_any(module_id)),
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

        let symbol_ids: Vec<LocalSymbolId> = scope
            .named_symbols
            .iter()
            .map(|(_, symbol_id)| *symbol_id)
            .chain(
                scope
                    .anonymous_symbols
                    .iter()
                    .copied()
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

                for (key, symbol_id) in &namespace_scope.named_symbols {
                    let symbol = symbols.get_symbol(*symbol_id);

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
                        if !exports.contains_key(&(space, *key)) {
                            exports.insert(
                                (space, *key),
                                Export::local(module_id, *key, space, *symbol_id),
                            );
                        }
                    }
                }
            }
        }
    }

    /// Build the export table for a module.
    fn build_module_exports(
        &self,
        module: &Module,
        dir: &ModuleDir,
        tree: &NodeTree,
        symbols: &mut SymbolTable,
    ) {
        // cache the default export name
        let default_name = self.program.strings.intern("default");

        // collect the export assignment if present
        let export_assignment_item = self.collect_export_assignment(module, dir, tree);

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
    ) -> Option<LocalNodeId<DependencyItem>> {
        // scan export statements for export assignments
        let mut export_assignment_item: Option<LocalNodeId<DependencyItem>> = None;
        for item_id in tree.iter_node_ids_of_type::<DependencyItem>() {
            // only process items at the module's namespace scope level
            let (item_scope, _) = tree.get_scope(item_id);
            if item_scope != dir.namespace_scope {
                continue;
            }

            // skip nonexport statements
            if self.export_statement_parent(tree, item_id).is_none() {
                continue;
            }

            // skip nonassignment values
            let DependencyItem::Value { mode, .. } = tree.get(item_id) else {
                continue;
            };
            if *mode != DependencyMode::Namespace {
                continue;
            }

            // report conflicts and keep the first assignment
            if let Some(existing) = export_assignment_item {
                self.error(ImportError::ConflictingExport {
                    node: item_id.into_global_any(module.id).into(),
                    other_node: existing.into_global_any(module.id).into(),
                    module: module.id,
                    name: None,
                });
                continue;
            }

            export_assignment_item = Some(item_id);
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
        let symbol_ids: Vec<LocalSymbolId> = namespace_scope
            .named_symbols
            .iter()
            .map(|(_, symbol_id)| *symbol_id)
            .chain(
                namespace_scope
                    .anonymous_symbols
                    .iter()
                    .copied()
                    .filter(|symbol_id| *symbol_id != dir.default_symbol),
            )
            .collect();

        for symbol_id in symbol_ids {
            // read symbol metadata
            let symbol = symbols.get_symbol(symbol_id);
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
    ) {
        // walk dependency items under export expressions
        for item_id in tree.iter_node_ids_of_type::<DependencyItem>() {
            // only process items at the module's namespace scope level
            let (item_scope, _) = tree.get_scope(item_id);
            if item_scope != dir.namespace_scope {
                continue;
            }

            // skip nonexport dependency items
            if self.export_item_parent(tree, item_id).is_none() {
                continue;
            }

            // reject exports when export assignment is present
            if let Some(export_assignment_item) = export_assignment_item
                && export_assignment_item != item_id
            {
                self.error(ImportError::ConflictingExport {
                    node: item_id.into_global_any(module_id).into(),
                    other_node: export_assignment_item.into_global_any(module_id).into(),
                    module: module_id,
                    name: None,
                });
                continue;
            }

            // extract export metadata from the dependency item
            let item = tree.get(item_id);
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
                            Some(item_id.into_global_any(module_id)),
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
                let export = Export::reexport(key, *space, item_id);
                self.insert_exports(
                    module_id,
                    symbols,
                    exports,
                    export,
                    Some(item_id.into_global_any(module_id)),
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
                for item_id in tree.iter_node_ids_of_type::<DependencyItem>() {
                    // skip items outside the binding scope
                    if !self.dependency_item_in_scope(tree, item_id, binding.scope) {
                        continue;
                    }

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

    /// Check if a dependency item is declared in a specific scope.
    fn dependency_item_in_scope(
        &self,
        tree: &NodeTree,
        item_id: LocalNodeId<DependencyItem>,
        scope_id: LocalScopeId,
    ) -> bool {
        // compare the dependency scope to the target scope
        let (item_scope_id, _) = tree.get_scope(item_id);
        item_scope_id == scope_id
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
        let symbol_ids: Vec<LocalSymbolId> = scope
            .named_symbols
            .iter()
            .map(|(_, symbol_id)| *symbol_id)
            .chain(
                scope
                    .anonymous_symbols
                    .iter()
                    .copied()
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
        name: Option<destack_base::StringId>,
        alias: Option<destack_base::StringId>,
        default_name: destack_base::StringId,
    ) -> Option<destack_base::StringId> {
        // prefer explicit aliases
        if alias.is_some() {
            return alias;
        }

        // use mode defaults for unnamed exports
        match mode {
            DependencyMode::Item => name,
            DependencyMode::Default => name.or(Some(default_name)),
            DependencyMode::Namespace => None,
        }
    }

    /// Get the symbol space for a global symbol.
    pub(super) fn symbol_space_for_global(
        &self,
        profile: ProfileId,
        symbol: GlobalSymbolId,
    ) -> SymbolSpace {
        let module = self.program.modules.get(symbol.module_id);
        let module = module.read();
        let symbols = module.dir(profile).symbols.read();
        symbols.get_symbol(symbol.local_id).space
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
    ) -> ResolveResult<()> {
        // load module data for canonical resolution
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();

        // collect symbols that have target_symbol but no canonical_symbol
        // (only include symbols with primary_declaration, others are internal or incomplete)
        let symbols_to_resolve: Vec<_> = (0..symbols.symbol_count())
            .map(LocalSymbolId::new)
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

        Ok(())
    }
}
