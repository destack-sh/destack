use crate::analyze::common::TreeSymbolView;
use std::collections::HashSet;

use crate::import::{SymbolDescriptor, can_merge_declarations};
use crate::{Compiler, ImportError};
use destack_builtin::builtin_lib;
use destack_dir::{
    DependencyItem, DependencyKind, DependencyMode, Export, ExportKind, Expression,
    GlobalNodeIdAny, GlobalSymbolId, LocalNodeId, LocalScopeId, LocalSymbolId, ModuleBinding,
    ModuleBindingExports, NodeTree, NodeVisitor, NodeVisitorOptions, StaticKey, SymbolKind,
    SymbolSpace, SymbolSpaceOrder, SymbolTable, SymbolType, walk_expression,
};
use destack_source::{LanguageType, ModuleId};
use destack_workspace::{Module, ModuleDir, ProfileId};
use indexmap::IndexMap;
use rustc_hash::FxHashMap;

/// Collect canonical dependency symbols for one export initializer.
#[derive(Debug)]
struct ExportDependencyCollector<'a> {
    /// The compiler driving export resolution.
    compiler: &'a Compiler,
    /// The module being resolved.
    module: &'a Module,
    /// The active profile.
    profile: ProfileId,
    /// The module symbol table.
    symbols: &'a SymbolTable,
    /// Collected dependency symbols.
    dependencies: Vec<GlobalSymbolId>,
    /// Visitor options.
    options: NodeVisitorOptions,
}

impl<'a> ExportDependencyCollector<'a> {
    /// Create a new export dependency collector.
    fn new(
        compiler: &'a Compiler,
        module: &'a Module,
        profile: ProfileId,
        symbols: &'a SymbolTable,
    ) -> Self {
        Self {
            compiler,
            module,
            profile,
            symbols,
            dependencies: Vec::new(),
            options: NodeVisitorOptions::default(),
        }
    }

    /// Finish collection and return canonical dependency symbols.
    fn finish(mut self) -> Vec<GlobalSymbolId> {
        self.dependencies.sort_unstable();
        self.dependencies.dedup();
        self.dependencies
    }

    /// Return the namespace-like target symbol for member dependency collection.
    fn namespace_target_symbol_maybe(
        &self,
        target_symbol: GlobalSymbolId,
    ) -> Option<GlobalSymbolId> {
        // keep imported aliases on their resolved remote target
        if target_symbol.module_id == self.module.id
            && let Some(imported_symbol) = self
                .symbols
                .get_symbol(target_symbol.local_id)
                .target_symbol
        {
            return Some(imported_symbol);
        }

        // only namespace-like locals should drive member dependency lookup
        if target_symbol.module_id == self.module.id {
            let symbol = self.symbols.get_symbol(target_symbol.local_id);
            if symbol.kind != SymbolKind::Namespace {
                return None;
            }
        }

        Some(target_symbol)
    }
}

impl NodeVisitor for ExportDependencyCollector<'_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // member access: resolve the member symbol once at export publication time
        if let Expression::Member {
            left,
            name,
            static_arguments,
        } = expression
        {
            let left_expression = tree.get(*left);
            let member_key = StaticKey::Name(*name);

            if static_arguments.is_none()
                && let Some(target_symbol) = left_expression.target_symbol()
                && let Some(namespace_target) = self.namespace_target_symbol_maybe(target_symbol)
            {
                if let Ok(Some(member_symbol)) = self.compiler.resolve_symbol_in_namespace(
                    id.into_global_any(self.module.id),
                    namespace_target,
                    self.profile,
                    DependencyKind::Value,
                    member_key,
                    None,
                ) {
                    self.dependencies.push(member_symbol);
                } else if let Some(member_symbol) = self.compiler.query_static_member_symbol(
                    self.module,
                    self.profile,
                    target_symbol,
                    member_key,
                    tree,
                    self.symbols,
                ) {
                    self.dependencies.push(member_symbol);
                }
            }
        }

        // encoded path member access
        if let Expression::LocalReference {
            path,
            static_arguments,
            target_symbol,
        }
        | Expression::ModuleReference {
            path,
            static_arguments,
            target_symbol,
        }
        | Expression::GlobalReference {
            path,
            static_arguments,
            target_symbol,
        } = expression
        {
            if static_arguments.is_none() && path.segments.len() > 1 {
                let mut current_symbol = *target_symbol;

                for segment in path.segments.iter().copied().skip(1) {
                    let member_key = StaticKey::Name(segment);
                    let Some(member_symbol) = self.compiler.query_static_member_symbol(
                        self.module,
                        self.profile,
                        current_symbol,
                        member_key,
                        tree,
                        self.symbols,
                    ) else {
                        break;
                    };
                    current_symbol = member_symbol;
                }

                if current_symbol != *target_symbol {
                    self.dependencies.push(current_symbol);
                }
            }
        }

        // direct references
        if let Some(target_symbol) = expression.target_symbol() {
            if target_symbol.module_id == self.module.id
                && let Some(imported_symbol) = self
                    .symbols
                    .get_symbol(target_symbol.local_id)
                    .target_symbol
            {
                self.dependencies.push(imported_symbol);
            } else {
                self.dependencies.push(target_symbol);
            }
        }

        destack_core::ensure_sufficient_stack(|| {
            walk_expression(self, tree, id, expression);
        });
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Build export tables for module bindings in this module.
    pub(super) fn build_module_binding_exports(
        &self,
        module: &Module,
        dir: &ModuleDir,
        tree: &NodeTree,
        symbols: &mut SymbolTable,
        dependency_items_by_scope: &FxHashMap<LocalScopeId, Vec<LocalNodeId<DependencyItem>>>,
        export_items_by_scope: &FxHashMap<LocalScopeId, Vec<LocalNodeId<DependencyItem>>>,
        export_assignments_by_scope: &FxHashMap<LocalScopeId, Option<LocalNodeId<DependencyItem>>>,
    ) {
        // cache the default export name
        let default_name = self.program.strings.intern("default");

        // build the binding export table
        let bindings = dir.module_bindings.read().clone();
        if bindings.is_empty() {
            *dir.module_binding_exports.write() = IndexMap::new();
            return;
        }
        let mut binding_exports = IndexMap::with_capacity(bindings.len());
        for binding in bindings {
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
                &binding,
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
                    &binding,
                    tree,
                    symbols,
                    &mut exports,
                    export_assignment_item,
                    default_name,
                    export_items,
                );
            }

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
                    language_type,
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

        for symbol_id in anonymous_symbol_ids {
            if symbol_id == binding.default_symbol {
                continue;
            }
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
                    language_type,
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
        language_type: LanguageType,
        binding: &ModuleBinding,
        tree: &NodeTree,
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
        tree: &NodeTree,
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
        tree: &NodeTree,
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
        tree: &NodeTree,
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
        dir: &ModuleDir,
        tree: &NodeTree,
        symbols: &mut SymbolTable,
        dependency_items_by_scope: &FxHashMap<LocalScopeId, Vec<LocalNodeId<DependencyItem>>>,
        export_items_by_scope: &FxHashMap<LocalScopeId, Vec<LocalNodeId<DependencyItem>>>,
        export_assignments_by_scope: &FxHashMap<LocalScopeId, Option<LocalNodeId<DependencyItem>>>,
    ) {
        // cache the default export name
        let default_name = self.program.strings.intern("default");
        let is_commonjs_module = module.module_format.is_commonjs();

        // collect the export assignment if present
        let dependency_items = dependency_items_by_scope
            .get(&dir.namespace_scope)
            .map(|items| items.as_slice())
            .unwrap_or_default();
        let export_assignment_item = export_assignments_by_scope
            .get(&dir.namespace_scope)
            .copied()
            .flatten();
        *dir.export_assignment.write() = export_assignment_item;
        let export_items = export_items_by_scope
            .get(&dir.namespace_scope)
            .map(|items| items.as_slice())
            .unwrap_or_default();

        // insert exports declared by symbols
        let mut exports = dir.exported_symbols.write();
        let namespace_scope = symbols.get_scope_by_id(dir.namespace_scope);
        if export_items.is_empty()
            && namespace_scope.named_symbols.is_empty()
            && namespace_scope.anonymous_symbols.is_empty()
            && !self.module_is_ambient_lib(module)
            && !is_commonjs_module
        {
            exports.clear();
            return;
        }
        exports.reserve(
            namespace_scope.named_symbols.len()
                + namespace_scope.anonymous_symbols.len()
                + dependency_items.len(),
        );
        self.insert_symbol_exports(
            module.id,
            module.language_type,
            dir,
            symbols,
            &mut exports,
            export_assignment_item,
            default_name,
        );

        // insert exports declared by dependency items
        if !export_items.is_empty() {
            self.insert_dependency_exports(
                module.id,
                module.language_type,
                dir,
                tree,
                symbols,
                &mut exports,
                export_assignment_item,
                default_name,
                export_items,
            );
        }

        // add ambient exports when a builtin lib is global
        if self.module_is_ambient_lib(module) {
            self.insert_ambient_exports(module.id, dir, symbols, &mut exports);
        }

        // synthesize static named exports for commonjs modules
        if is_commonjs_module && export_assignment_item.is_none() {
            self.insert_commonjs_named_exports(
                module.id,
                dir.namespace_scope,
                dir.namespace_symbol.into_global(module.id),
                &dir.roots,
                tree,
                symbols,
                &mut exports,
            );
        }
    }

    /// Insert exports declared by symbols.
    fn insert_symbol_exports(
        &self,
        module_id: ModuleId,
        language_type: LanguageType,
        dir: &ModuleDir,
        symbols: &mut SymbolTable,
        exports: &mut IndexMap<(SymbolSpace, StaticKey), Export>,
        export_assignment_item: Option<LocalNodeId<DependencyItem>>,
        default_name: destack_core::StringId,
    ) {
        // collect namespace symbols
        let namespace_scope = symbols.get_scope_by_id(dir.namespace_scope);
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
                language_type,
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

        for symbol_id in anonymous_symbol_ids {
            if symbol_id == dir.default_symbol {
                continue;
            }
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
                language_type,
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
        language_type: LanguageType,
        dir: &ModuleDir,
        tree: &NodeTree,
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
                            Export::local(module_id, key, SymbolSpace::Value, dir.default_symbol);
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
        module: &Module,
        profile: ProfileId,
        dir: &ModuleDir,
        tree: &NodeTree,
        symbols: &mut SymbolTable,
    ) {
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
            let dependency_items = tree.iter_node_ids_of_type::<DependencyItem>();
            self.resolve_default_export_from_dependency_items(
                dir.default_symbol,
                tree,
                symbols,
                dependency_items.into_iter(),
            );
        }

        // resolve export targets for reexports
        let mut exports = dir.exported_symbols.write();
        self.finalize_export_targets(tree, &mut exports);
        self.finalize_export_dependencies(module, profile, tree, symbols, &mut exports);
    }

    /// Finalize export targets for module bindings after dependency resolution.
    pub(super) fn finalize_module_binding_exports(
        &self,
        module: &Module,
        profile: ProfileId,
        dir: &ModuleDir,
        tree: &NodeTree,
        symbols: &mut SymbolTable,
        dependency_items_by_scope: &FxHashMap<LocalScopeId, Vec<LocalNodeId<DependencyItem>>>,
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
                let dependency_items = dependency_items_by_scope
                    .get(&binding.scope)
                    .map(|items| items.as_slice())
                    .unwrap_or_default();
                self.resolve_default_export_from_dependency_items(
                    binding.default_symbol,
                    tree,
                    symbols,
                    dependency_items.iter().copied(),
                );
            }

            // resolve export targets for reexports
            if let Some(binding_exports) = binding_exports.get_mut(&binding_key) {
                self.finalize_export_targets(tree, &mut binding_exports.exports);
                self.finalize_export_dependencies(
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

    /// Finalize canonical export dependency symbols after target resolution.
    fn finalize_export_dependencies(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        exports: &mut IndexMap<(SymbolSpace, StaticKey), Export>,
    ) {
        // publish the canonical dependency surface once at export finalization time
        for export in exports.values_mut() {
            export.dependencies =
                self.export_dependencies_for_export(module, profile, tree, symbols, export);
        }
    }

    /// Compute canonical dependency symbols for one finalized export.
    fn export_dependencies_for_export(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        export: &Export,
    ) -> Vec<GlobalSymbolId> {
        // reexports depend directly on their resolved target symbol
        if export.kind == ExportKind::ReExport {
            return export.target.resolved().into_iter().collect();
        }

        // only local value exports with one direct binding initializer participate in
        // export dependency cycles
        let tree_symbol_view = TreeSymbolView::new(module, profile, tree, symbols);
        let Some((_, value_symbol)) =
            self.interface_value_symbol_for_export(symbols, module.id, export)
        else {
            return Vec::new();
        };
        let Some(declarator_id) =
            self.direct_binding_declarator_for_symbol(tree_symbol_view, value_symbol)
        else {
            return Vec::new();
        };
        let Some(value_id) = tree.get(declarator_id).value else {
            return Vec::new();
        };

        // collect dependencies from the initializer once
        let mut collector = ExportDependencyCollector::new(self, module, profile, symbols);
        collector.visit_expression(tree, value_id, tree.get(value_id));
        collector.finish()
    }

    /// Resolve a default export target from dependency items.
    fn resolve_default_export_from_dependency_items(
        &self,
        default_symbol: LocalSymbolId,
        tree: &NodeTree,
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

            // resolve the target symbol for the default export
            let value_expression = tree.get(*value);
            if let Some(target_symbol) = value_expression.target_symbol() {
                symbols
                    .get_symbol_mut(default_symbol)
                    .resolve_to(target_symbol);
                break;
            }
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

    /// Get any export parent expression for an item, if any.
    pub(crate) fn export_item_parent(
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
        let symbol_ids: Vec<LocalSymbolId> = symbols
            .active_named_symbols(scope)
            .map(|(_, symbol_id)| symbol_id)
            .chain(
                symbols
                    .active_anonymous_symbols(scope)
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
        let module = self.program.modules.get(symbol.module_id);
        let module = module.read();
        let symbols = module.dir(profile).symbols.read();
        symbols.get_symbol(symbol.local_id).space
    }

    /// Get the symbol type for a global symbol.
    pub(crate) fn symbol_type_for_global(
        &self,
        profile: ProfileId,
        symbol: GlobalSymbolId,
    ) -> SymbolType {
        let module = self.program.modules.get(symbol.module_id);
        let module = module.read();
        let symbols = module.dir(profile).symbols.read();
        symbols.get_symbol(symbol.local_id).ty
    }

    /// Check whether a symbol can be used as a value.
    ///
    /// This centralizes the type/value decision so Resolve can honor declaration-module reexports
    /// without silently treating nominal value symbols as type-only.
    pub(crate) fn symbol_is_value_capable(
        &self,
        profile: ProfileId,
        symbol: GlobalSymbolId,
    ) -> bool {
        // load the symbol entry
        let module = self.program.modules.get(symbol.module_id);
        let module = module.read();
        let symbols = module.dir(profile).symbols.read();
        let entry = symbols.get_symbol(symbol.local_id);

        // value-space symbols are always value-capable
        if entry.space == SymbolSpace::Value {
            return true;
        }

        // nominal declarations always introduce a value-capable symbol
        if matches!(
            entry.ty,
            SymbolType::Struct
                | SymbolType::Class
                | SymbolType::Enum
                | SymbolType::Newtype
                | SymbolType::Function
        ) {
            return true;
        }

        // type-value symbols only exclude type-only declarations
        if entry.space == SymbolSpace::TypeValue {
            return !matches!(
                entry.ty,
                SymbolType::Interface | SymbolType::TypeAlias | SymbolType::Extension
            );
        }

        // fall back to not value-capable
        false
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
