use std::collections::HashSet;

use crate::common::dir::{SymbolDescriptor, can_merge_declarations};
use crate::{Compiler, CompilerContext, ImportError};
use destack_artifact::{ExportedSymbolTable, ModuleBindingExportTable};
use destack_builtin::builtin_library;
use destack_dir::{
    DependencyItem, DependencyKind, DependencyMode, Export, ExportKind, ExportMode, Expression,
    GlobalNodeIdAny, GlobalSymbolId, LocalNodeId, LocalScopeId, LocalSymbolId, ModuleBinding,
    ModuleBindingExports, StaticKey, SymbolSpace, SymbolSpaceOrder, SymbolTable, SymbolType, Tree,
};
use destack_source::{LanguageType, ModuleId};
use destack_workspace::{Module, ProfileId};
use indexmap::IndexMap;
use rustc_hash::FxHashMap;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Build export tables for module bindings in this module.
    pub(super) fn build_module_binding_exports(
        &self,
        module: &Module,
        tree: &Tree,
        symbols: &mut SymbolTable,
        module_bindings: &[ModuleBinding],
        module_binding_exports: &mut ModuleBindingExportTable,
        dependency_items_by_scope: &FxHashMap<LocalScopeId, Vec<LocalNodeId<DependencyItem>>>,
        export_items_by_scope: &FxHashMap<LocalScopeId, Vec<LocalNodeId<DependencyItem>>>,
        export_assignments_by_scope: &FxHashMap<LocalScopeId, Option<LocalNodeId<DependencyItem>>>,
    ) {
        // cache the default export name
        let default_name = self.repository.strings.intern("default");

        // build the binding export table
        if module_bindings.is_empty() {
            *module_binding_exports = IndexMap::new();
            return;
        }
        let mut binding_exports = IndexMap::with_capacity(module_bindings.len());
        for binding in module_bindings {
            // collect the export assignment if present
            let dependency_items = dependency_items_by_scope
                .get(&binding.scope)
                .map(|items| items.as_slice())
                .unwrap_or_default();
            let export_assignment_item = export_assignments_by_scope
                .get(&binding.scope)
                .copied()
                .flatten();
            let export_items = export_items_by_scope
                .get(&binding.scope)
                .map(|items| items.as_slice())
                .unwrap_or_default();

            // insert exports declared by symbols
            let scope = symbols.get_scope_by_id(binding.scope);
            let mut exports = IndexMap::with_capacity(
                scope.named_symbols.len() + scope.anonymous_symbols.len() + dependency_items.len(),
            );
            self.insert_binding_symbol_exports(
                module.id,
                module.language_type,
                binding,
                symbols,
                &mut exports,
                export_assignment_item,
                default_name,
            );

            // insert exports declared by dependency items
            if !export_items.is_empty() {
                self.insert_binding_dependency_exports(
                    module.id,
                    module.language_type,
                    binding,
                    tree,
                    symbols,
                    &mut exports,
                    export_assignment_item,
                    default_name,
                    export_items,
                );
            }

            // insert ambient exports for remaining names
            self.insert_binding_ambient_exports(module.id, binding, symbols, &mut exports);

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
        *module_binding_exports = binding_exports;
    }

    /// Insert symbol exports for a module binding.
    fn insert_binding_symbol_exports(
        &self,
        module_id: ModuleId,
        language_type: LanguageType,
        binding: &ModuleBinding,
        symbols: &mut SymbolTable,
        exports: &mut IndexMap<(SymbolSpace, StaticKey), Export>,
        export_assignment_item: Option<LocalNodeId<DependencyItem>>,
        default_name: destack_core::StringId,
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
            if symbol.origin.is_global_augmentation() {
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
                ExportMode::Default => StaticKey::Name(default_name),
                ExportMode::Named => {
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
                    language_type,
                    symbols,
                    exports,
                    export,
                    symbol.primary_declaration,
                );
            }

            // align the binding default symbol with default export declarations
            if export_mode == ExportMode::Default
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
            if symbol.origin.is_global_augmentation() {
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
                ExportMode::Default => StaticKey::Name(default_name),
                ExportMode::Named => {
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
                    language_type,
                    symbols,
                    exports,
                    export,
                    symbol.primary_declaration,
                );
            }

            // align the binding default symbol with default export declarations
            if export_mode == ExportMode::Default
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
        language_type: LanguageType,
        binding: &ModuleBinding,
        tree: &Tree,
        symbols: &SymbolTable,
        exports: &mut IndexMap<(SymbolSpace, StaticKey), Export>,
        export_assignment_item: Option<LocalNodeId<DependencyItem>>,
        default_name: destack_core::StringId,
        export_items: &[LocalNodeId<DependencyItem>],
    ) {
        // dependency items are already filtered to export expressions
        for item_id in export_items {
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
                            language_type,
                            symbols,
                            exports,
                            export,
                            Some((*item_id).into_global_any(module_id)),
                        );
                    }
                    continue;
                }
                DependencyItem::Error => {
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
                    language_type,
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
            if symbol.origin.is_global_augmentation() {
                continue;
            }
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
            if symbol.origin.is_global_augmentation() {
                continue;
            }
            if symbol.kind == destack_dir::SymbolKind::Namespace {
                namespace_symbols.push(symbol_id);
            }

            // check merge group for additional namespace symbols (e.g., function+namespace merge)
            if let Some(merge_group) = symbol.merge_group {
                for &group_symbol_id in symbols.merge_group_symbols(merge_group) {
                    let group_symbol = symbols.get_symbol(group_symbol_id);
                    if group_symbol.origin.is_global_augmentation() {
                        continue;
                    }
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
    pub(super) fn dependency_items_by_scope(
        &self,
        tree: &Tree,
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

    /// Group export items by their declaring scope.
    pub(super) fn export_items_by_scope(
        &self,
        tree: &Tree,
        dependency_items_by_scope: &FxHashMap<LocalScopeId, Vec<LocalNodeId<DependencyItem>>>,
    ) -> FxHashMap<LocalScopeId, Vec<LocalNodeId<DependencyItem>>> {
        // filter dependency items to export statements
        let mut items_by_scope: FxHashMap<LocalScopeId, Vec<LocalNodeId<DependencyItem>>> =
            FxHashMap::default();
        for (scope_id, items) in dependency_items_by_scope {
            for item_id in items {
                if self.export_item_parent(tree, *item_id).is_none() {
                    continue;
                }
                items_by_scope.entry(*scope_id).or_default().push(*item_id);
            }
        }

        items_by_scope
    }

    /// Collect export assignment items by their declaring scope.
    pub(super) fn export_assignments_by_scope(
        &self,
        module_id: ModuleId,
        tree: &Tree,
        export_items_by_scope: &FxHashMap<LocalScopeId, Vec<LocalNodeId<DependencyItem>>>,
    ) -> FxHashMap<LocalScopeId, Option<LocalNodeId<DependencyItem>>> {
        // scan export statements for export assignments
        let mut assignments_by_scope: FxHashMap<LocalScopeId, Option<LocalNodeId<DependencyItem>>> =
            FxHashMap::default();
        for (scope_id, items) in export_items_by_scope {
            let mut assignment_item: Option<LocalNodeId<DependencyItem>> = None;
            for item_id in items {
                if self.export_statement_parent(tree, *item_id).is_none() {
                    continue;
                }

                let DependencyItem::Value { mode, .. } = tree.get(*item_id) else {
                    continue;
                };
                if *mode != DependencyMode::Namespace {
                    continue;
                }

                if let Some(existing) = assignment_item {
                    self.error(ImportError::ConflictingExport {
                        node: (*item_id).into_global_any(module_id).into(),
                        other_node: existing.into_global_any(module_id).into(),
                        module: module_id,
                        name: None,
                    });
                    continue;
                }

                assignment_item = Some(*item_id);
            }
            assignments_by_scope.insert(*scope_id, assignment_item);
        }

        assignments_by_scope
    }

    /// Build the export table for a module.
    pub(super) fn build_module_exports(
        &self,
        module: &Module,
        tree: &Tree,
        symbols: &mut SymbolTable,
        namespace_scope: LocalScopeId,
        default_symbol: LocalSymbolId,
        export_assignment_symbol: LocalSymbolId,
        export_assignment: &mut Option<LocalNodeId<DependencyItem>>,
        exported_symbols: &mut ExportedSymbolTable,
        dependency_items_by_scope: &FxHashMap<LocalScopeId, Vec<LocalNodeId<DependencyItem>>>,
        export_items_by_scope: &FxHashMap<LocalScopeId, Vec<LocalNodeId<DependencyItem>>>,
        export_assignments_by_scope: &FxHashMap<LocalScopeId, Option<LocalNodeId<DependencyItem>>>,
    ) {
        // cache the default export name
        let default_name = self.repository.strings.intern("default");

        // collect the export assignment if present
        let dependency_items = dependency_items_by_scope
            .get(&namespace_scope)
            .map(|items| items.as_slice())
            .unwrap_or_default();
        let export_assignment_item = export_assignments_by_scope
            .get(&namespace_scope)
            .copied()
            .flatten();
        *export_assignment = export_assignment_item;
        let export_items = export_items_by_scope
            .get(&namespace_scope)
            .map(|items| items.as_slice())
            .unwrap_or_default();

        // insert exports declared by symbols
        let namespace_scope_symbols = symbols.get_scope_by_id(namespace_scope);
        if export_items.is_empty()
            && namespace_scope_symbols.named_symbols.is_empty()
            && namespace_scope_symbols.anonymous_symbols.is_empty()
            && !self.module_is_ambient_lib(module)
        {
            exported_symbols.clear();
            return;
        }
        exported_symbols.reserve(
            namespace_scope_symbols.named_symbols.len()
                + namespace_scope_symbols.anonymous_symbols.len()
                + dependency_items.len(),
        );
        self.insert_symbol_exports(
            module.id,
            module.language_type,
            namespace_scope,
            default_symbol,
            symbols,
            exported_symbols,
            export_assignment_item,
            default_name,
        );

        // insert exports declared by dependency items
        if !export_items.is_empty() {
            self.insert_dependency_exports(
                module.id,
                module.language_type,
                default_symbol,
                tree,
                symbols,
                exported_symbols,
                export_assignment_item,
                default_name,
                export_items,
            );
        }

        // add ambient exports when a builtin lib is global
        if self.module_is_ambient_lib(module) {
            self.insert_ambient_exports(
                module.id,
                namespace_scope,
                default_symbol,
                export_assignment_symbol,
                symbols,
                exported_symbols,
            );
        }
    }

    /// Insert exports declared by symbols.
    fn insert_symbol_exports(
        &self,
        module_id: ModuleId,
        language_type: LanguageType,
        namespace_scope: LocalScopeId,
        default_symbol: LocalSymbolId,
        symbols: &mut SymbolTable,
        exports: &mut IndexMap<(SymbolSpace, StaticKey), Export>,
        export_assignment_item: Option<LocalNodeId<DependencyItem>>,
        default_name: destack_core::StringId,
    ) {
        // collect namespace symbols
        let namespace_scope = symbols.get_scope_by_id(namespace_scope);
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
                ExportMode::Default => StaticKey::Name(default_name),
                ExportMode::Named => {
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
                language_type,
                symbols,
                exports,
                export,
                symbol.primary_declaration,
            );

            // align the module default symbol with default export declarations
            if export_mode == ExportMode::Default
                && symbols.get_symbol(default_symbol).target_symbol.is_none()
            {
                symbols
                    .get_symbol_mut(default_symbol)
                    .resolve_to(symbol_id.into_global(module_id));
            }
        }

        for symbol_id in anonymous_symbol_ids {
            if symbol_id == default_symbol {
                continue;
            }
            let symbol = symbols.get_symbol(symbol_id);
            if !symbol.is_active() {
                continue;
            }
            if symbol.origin.is_global_augmentation() {
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
                ExportMode::Default => StaticKey::Name(default_name),
                ExportMode::Named => {
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
                language_type,
                symbols,
                exports,
                export,
                symbol.primary_declaration,
            );

            // align the module default symbol with default export declarations
            if export_mode == ExportMode::Default
                && symbols.get_symbol(default_symbol).target_symbol.is_none()
            {
                symbols
                    .get_symbol_mut(default_symbol)
                    .resolve_to(symbol_id.into_global(module_id));
            }
        }
    }

    /// Insert exports declared by dependency items.
    fn insert_dependency_exports(
        &self,
        module_id: ModuleId,
        language_type: LanguageType,
        default_symbol: LocalSymbolId,
        tree: &Tree,
        symbols: &SymbolTable,
        exports: &mut IndexMap<(SymbolSpace, StaticKey), Export>,
        export_assignment_item: Option<LocalNodeId<DependencyItem>>,
        default_name: destack_core::StringId,
        export_items: &[LocalNodeId<DependencyItem>],
    ) {
        // dependency items are already filtered to export expressions
        for item_id in export_items {
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
                            Export::local(module_id, key, SymbolSpace::Value, default_symbol);
                        self.insert_exports(
                            module_id,
                            language_type,
                            symbols,
                            exports,
                            export,
                            Some((*item_id).into_global_any(module_id)),
                        );
                    }
                    continue;
                }
                DependencyItem::Error => {
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
                    language_type,
                    symbols,
                    exports,
                    export,
                    Some((*item_id).into_global_any(module_id)),
                );
            }
        }
    }

    /// Finalize export targets after dependency resolution.
    pub(super) fn finalize_module_exports(
        &self,
        context: &CompilerContext<'_>,
        module: &Module,
        profile: ProfileId,
        tree: &Tree,
        symbols: &mut SymbolTable,
        default_symbol: LocalSymbolId,
        export_assignment_symbol: LocalSymbolId,
        export_assignment: Option<LocalNodeId<DependencyItem>>,
        exported_symbols: &mut ExportedSymbolTable,
    ) {
        // resolve export assignment target symbols
        if let Some(item_id) = export_assignment {
            let DependencyItem::Value { mode, value } = tree.get(item_id) else {
                return;
            };

            symbols
                .get_symbol_mut(export_assignment_symbol)
                .primary_declaration = Some(item_id.into_global_any(module.id));

            if *mode == DependencyMode::Namespace {
                let value_expression = tree.get(*value);
                if let Some(target_symbol) = value_expression.target_symbol() {
                    symbols
                        .get_symbol_mut(export_assignment_symbol)
                        .resolve_to(target_symbol);
                }
            }
        }

        // record the default export declaration and target from value expressions
        let dependency_items = tree.iter_node_ids_of_type::<DependencyItem>();
        self.resolve_default_export_from_dependency_items(
            module.id,
            default_symbol,
            tree,
            symbols,
            dependency_items.into_iter(),
        );

        // resolve export targets for reexports
        self.finalize_export_targets(tree, exported_symbols);
        self.normalize_reexport_export_spaces(
            module.id,
            module.language_type,
            tree,
            symbols,
            exported_symbols,
        );
        self.finalize_export_dependencies(
            context,
            module,
            profile,
            tree,
            symbols,
            exported_symbols,
        );
    }

    /// Finalize export targets for module bindings after dependency resolution.
    pub(super) fn finalize_module_binding_exports(
        &self,
        context: &CompilerContext<'_>,
        module: &Module,
        profile: ProfileId,
        tree: &Tree,
        symbols: &mut SymbolTable,
        module_bindings: &[ModuleBinding],
        binding_exports: &mut ModuleBindingExportTable,
        dependency_items_by_scope: &FxHashMap<LocalScopeId, Vec<LocalNodeId<DependencyItem>>>,
    ) {
        // finalize exports for each module binding
        for binding in module_bindings {
            let binding_key = binding.declaration.into_any();

            // resolve export assignment target symbols
            if let Some(item_id) = binding_exports
                .get(&binding_key)
                .and_then(|binding_exports| binding_exports.export_assignment)
            {
                let DependencyItem::Value { mode, value } = tree.get(item_id) else {
                    continue;
                };

                symbols
                    .get_symbol_mut(binding.export_assignment_symbol)
                    .primary_declaration = Some(item_id.into_global_any(module.id));

                if *mode == DependencyMode::Namespace {
                    let value_expression = tree.get(*value);
                    if let Some(target_symbol) = value_expression.target_symbol() {
                        symbols
                            .get_symbol_mut(binding.export_assignment_symbol)
                            .resolve_to(target_symbol);
                    }
                }
            }

            // record the default export declaration and target from value expressions
            let dependency_items = dependency_items_by_scope
                .get(&binding.scope)
                .map(|items| items.as_slice())
                .unwrap_or_default();
            self.resolve_default_export_from_dependency_items(
                module.id,
                binding.default_symbol,
                tree,
                symbols,
                dependency_items.iter().copied(),
            );

            // resolve export targets for reexports
            if let Some(binding_exports) = binding_exports.get_mut(&binding_key) {
                self.finalize_export_targets(tree, &mut binding_exports.exports);
                self.normalize_reexport_export_spaces(
                    module.id,
                    module.language_type,
                    tree,
                    symbols,
                    &mut binding_exports.exports,
                );
                self.finalize_export_dependencies(
                    context,
                    module,
                    profile,
                    tree,
                    symbols,
                    &mut binding_exports.exports,
                );
            }
        }
    }

    /// Resolve reexport targets after dependency resolution.
    fn finalize_export_targets(
        &self,
        tree: &Tree,
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

    /// Normalize reexport export spaces from the resolved dependency item kind.
    fn normalize_reexport_export_spaces(
        &self,
        module_id: ModuleId,
        language_type: LanguageType,
        tree: &Tree,
        symbols: &SymbolTable,
        exports: &mut IndexMap<(SymbolSpace, StaticKey), Export>,
    ) {
        let mut normalized = IndexMap::new();

        // rebuild reexport entries from their resolved dependency item kinds
        for export in exports.values().cloned() {
            if export.kind != ExportKind::ReExport {
                self.insert_exports(
                    module_id,
                    language_type,
                    symbols,
                    &mut normalized,
                    export,
                    None,
                );
                continue;
            }

            let Some(item_id) = export.item else {
                self.insert_exports(
                    module_id,
                    language_type,
                    symbols,
                    &mut normalized,
                    export,
                    None,
                );
                continue;
            };

            let spaces = match tree.get(item_id) {
                DependencyItem::Local { kind, .. }
                | DependencyItem::Remote { kind, .. }
                | DependencyItem::UnresolvedLocal { kind, .. }
                | DependencyItem::UnresolvedRemote { kind, .. } => match kind {
                    DependencyKind::Type => SymbolSpaceOrder::TypeOnly,
                    DependencyKind::Value => SymbolSpaceOrder::ValueOnly,
                },
                DependencyItem::Value { .. } => SymbolSpaceOrder::ValueOnly,
                DependencyItem::Error => {
                    continue;
                }
            };

            for space in spaces.spaces() {
                let mut export = export.clone();
                export.space = *space;
                self.insert_exports(
                    module_id,
                    language_type,
                    symbols,
                    &mut normalized,
                    export,
                    None,
                );
            }
        }

        *exports = normalized;
    }

    /// Finalize canonical export dependency symbols after target resolution.
    fn finalize_export_dependencies(
        &self,
        context: &CompilerContext<'_>,
        module: &Module,
        profile: ProfileId,
        tree: &Tree,
        symbols: &SymbolTable,
        exports: &mut IndexMap<(SymbolSpace, StaticKey), Export>,
    ) {
        // publish the canonical dependency surface once at export finalization time
        for export in exports.values_mut() {
            export.dependencies = self
                .export_dependencies_for_export(context, module, profile, tree, symbols, export);
        }
    }

    /// Compute canonical dependency symbols for one finalized export.
    fn export_dependencies_for_export(
        &self,
        context: &CompilerContext<'_>,
        module: &Module,
        profile: ProfileId,
        tree: &Tree,
        symbols: &SymbolTable,
        export: &Export,
    ) -> Vec<GlobalSymbolId> {
        // reexports depend directly on their resolved target symbol
        if export.kind == ExportKind::ReExport {
            return export.target.resolved().into_iter().collect();
        }

        let _ = (context, module, profile, tree, symbols, export);

        Vec::new()
    }

    /// Resolve a default export target from dependency items.
    fn resolve_default_export_from_dependency_items(
        &self,
        module_id: ModuleId,
        default_symbol: LocalSymbolId,
        tree: &Tree,
        symbols: &mut SymbolTable,
        dependency_items: impl Iterator<Item = LocalNodeId<DependencyItem>>,
    ) {
        // scan export statements for default assignments
        for item_id in dependency_items {
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

            // record the dependency item as the primary declaration
            symbols.get_symbol_mut(default_symbol).primary_declaration =
                Some(item_id.into_global_any(module_id));

            // resolve the target symbol for the default export
            let value_expression = tree.get(*value);
            if let Some(target_symbol) = value_expression.target_symbol() {
                let default_symbol = symbols.get_symbol_mut(default_symbol);
                default_symbol.resolve_to(target_symbol);
                break;
            }
        }
    }

    /// Get the export statement parent for an item, if any.
    fn export_statement_parent(
        &self,
        tree: &Tree,
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
    pub(crate) fn export_item_parent(
        &self,
        tree: &Tree,
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
        let builtins = self.repository.builtins.as_ref();
        let Some(lib_name) = builtins.library_name_for_module(module.id) else {
            return false;
        };
        let Some(lib) = builtin_library(lib_name) else {
            return false;
        };
        lib.is_ambient
    }

    /// Insert ambient exports without overriding explicit export entries.
    fn insert_ambient_exports(
        &self,
        module_id: ModuleId,
        namespace_scope: LocalScopeId,
        default_symbol: LocalSymbolId,
        export_assignment_symbol: LocalSymbolId,
        symbols: &SymbolTable,
        exports: &mut IndexMap<(SymbolSpace, StaticKey), Export>,
    ) {
        // collect namespace symbols
        let scope = symbols.get_scope_by_id(namespace_scope);
        let symbol_ids: Vec<LocalSymbolId> = symbols
            .active_named_symbols(scope)
            .map(|(_, symbol_id)| symbol_id)
            .chain(
                symbols
                    .active_anonymous_symbols(scope)
                    .filter(|symbol_id| *symbol_id != default_symbol),
            )
            .collect();

        // insert exports without overriding explicit entries
        for symbol_id in symbol_ids {
            // skip synthetic export assignment symbol
            if symbol_id == export_assignment_symbol {
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
    pub(crate) fn export_name_for_dependency(
        &self,
        mode: DependencyMode,
        name: Option<destack_dir::Name>,
        alias: Option<destack_core::StringId>,
        default_name: destack_core::StringId,
    ) -> Option<destack_core::StringId> {
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
        let dir = self
            .dir_prepared(symbol.module_id, profile)
            .unwrap_or_else(|| {
                panic!(
                    "missing prepared dir for symbol space lookup: module={:?} profile={:?}",
                    symbol.module_id, profile
                )
            });
        dir.symbols.get_symbol(symbol.local_id).space
    }

    /// Get the symbol type for a global symbol.
    pub(crate) fn symbol_type_for_global(
        &self,
        profile: ProfileId,
        symbol: GlobalSymbolId,
    ) -> SymbolType {
        let dir = self
            .dir_prepared(symbol.module_id, profile)
            .unwrap_or_else(|| {
                panic!(
                    "missing prepared dir for symbol type lookup: module={:?} profile={:?}",
                    symbol.module_id, profile
                )
            });
        dir.symbols.get_symbol(symbol.local_id).ty
    }

    /// Insert exports into the table and report conflicts.
    fn insert_exports(
        &self,
        module_id: ModuleId,
        language_type: LanguageType,
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
            self.insert_exports(
                module_id,
                language_type,
                symbols,
                exports,
                type_export,
                node,
            );

            let mut value_export = export;
            value_export.space = SymbolSpace::Value;
            self.insert_exports(
                module_id,
                language_type,
                symbols,
                exports,
                value_export,
                node,
            );
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

            // allow declaration merges even when symbols were bound separately
            if self.local_export_symbols_can_merge(
                language_type,
                symbols,
                existing_symbol,
                next_symbol,
            ) {
                exports.insert((space, key), export);
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

    /// Return true when two local export symbols can merge as declarations.
    fn local_export_symbols_can_merge(
        &self,
        language_type: LanguageType,
        symbols: &SymbolTable,
        left: LocalSymbolId,
        right: LocalSymbolId,
    ) -> bool {
        // load symbols and descriptors
        let left_symbol = symbols.get_symbol(left);
        let right_symbol = symbols.get_symbol(right);
        let left_descriptor = SymbolDescriptor::from(left_symbol);
        let right_descriptor = SymbolDescriptor::from(right_symbol);

        // apply declaration order to order-sensitive merges
        match (
            left_symbol.primary_declaration,
            right_symbol.primary_declaration,
        ) {
            (Some(left_node), Some(right_node))
                if left_node.local_id.id > right_node.local_id.id =>
            {
                can_merge_declarations(language_type, right_descriptor, left_descriptor)
            }
            _ => can_merge_declarations(language_type, left_descriptor, right_descriptor),
        }
    }
}
