use destack_core::StringId;
use destack_dir::{self as dir};
use destack_source::{ModuleId, ModuleVersion};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{ImportEdgeKind, ImportMeta, Loader, ProfileId};

/// Mutable import-time DIR state for one module snapshot.
/// FUGU #Architecture: this is still a broad mutable import working set and should shrink toward narrower phase-local tables.
#[derive(Debug)]
#[allow(clippy::type_complexity)]
pub struct ImportDir {
    /// The profile id this is targeting, if any.
    pub profile_id: Option<ProfileId>,
    /// The id of the Module.
    pub id: ModuleId,
    /// The version of the Module.
    pub version: ModuleVersion,

    /// The main DIR node tree of the Module.
    pub tree: dir::NodeTree,
    /// The symbol side table of the Module.
    pub symbols: dir::SymbolTable,
    /// The type side table of the Module.
    pub types: dir::TypeTable,
    /// The capture side table of the Module.
    pub captures: dir::CaptureTable,
    /// The top-level expressions of the Module.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,
    /// Stable fallback node for diagnostics and synthetic types.
    pub anchor_node: dir::LocalNodeIdAny,

    /// Metadata exposed via import.meta.
    pub import_meta: Option<ImportMeta>,
    /// The symbol of the Module namespace.
    pub namespace_symbol: dir::LocalSymbolId,
    /// The scope of the Module.
    pub namespace_scope: dir::LocalScopeId,
    /// The scope for global augmentations within this module.
    pub global_augmentation_scope: dir::LocalScopeId,
    /// The symbol of the Module default.
    pub default_symbol: dir::LocalSymbolId,
    /// The symbol of the Module export assignment.
    pub export_assignment_symbol: dir::LocalSymbolId,
    /// Export assignment item (`export = ...`) when present.
    pub export_assignment: Option<dir::LocalNodeId<dir::DependencyItem>>,
    /// Namespace exports: modules whose exports are re-exported via `export * from "..."`.
    pub namespace_exports: Vec<dir::NamespaceExport>,
    /// Module bindings (e.g., `declare module "foo"`).
    pub module_bindings: Vec<dir::ModuleBinding>,
    /// Export tables for module bindings.
    pub module_binding_exports: IndexMap<dir::LocalNodeIdAny, dir::ModuleBindingExports>,
    /// Resolved import specifiers to module ids.
    pub imported_modules: IndexMap<
        (Option<ModuleId>, StringId, ImportEdgeKind, Option<Loader>),
        dir::ModuleResolution,
    >,
    /// Exported symbols by key (space, name).
    pub exported_symbols: IndexMap<(dir::SymbolSpace, dir::StaticKey), dir::Export>,
}

/// Immutable published DIR artifact data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::type_complexity)]
pub struct ModuleDir {
    /// The profile id this is targeting, if any.
    pub profile_id: Option<ProfileId>,
    /// The id of the Module.
    pub id: ModuleId,
    /// The version of the Module.
    pub version: ModuleVersion,

    /// The main DIR node tree of the Module.
    pub tree: Arc<dir::NodeTree>,
    /// The symbol side table of the Module.
    pub symbols: Arc<dir::SymbolTable>,
    /// The type side table of the Module (includes types, instances, resolutions).
    pub types: Arc<dir::TypeTable>,
    /// The capture side table of the Module.
    pub captures: Arc<dir::CaptureTable>,
    /// The top-level expressions of the Module.
    pub roots: Arc<Vec<dir::LocalNodeId<dir::Expression>>>,
    /// Stable fallback node for diagnostics and synthetic types.
    pub anchor_node: dir::LocalNodeIdAny,

    /// Metadata exposed via import.meta.
    pub import_meta: Option<ImportMeta>,
    /// The symbol of the Module namespace.
    pub namespace_symbol: dir::LocalSymbolId,
    /// The scope of the Module.
    pub namespace_scope: dir::LocalScopeId,
    /// The scope for global augmentations within this module.
    pub global_augmentation_scope: dir::LocalScopeId,
    /// The symbol of the Module default.
    pub default_symbol: dir::LocalSymbolId,
    /// The symbol of the Module export assignment.
    pub export_assignment_symbol: dir::LocalSymbolId,
    /// Export assignment item (`export = ...`) when present.
    pub export_assignment: Option<dir::LocalNodeId<dir::DependencyItem>>,
    /// Namespace exports: modules whose exports are re-exported via `export * from "..."`.
    pub namespace_exports: Arc<Vec<dir::NamespaceExport>>,
    /// Module bindings (e.g., `declare module "foo"`).
    pub module_bindings: Arc<Vec<dir::ModuleBinding>>,
    /// Export tables for module bindings.
    pub module_binding_exports: Arc<IndexMap<dir::LocalNodeIdAny, dir::ModuleBindingExports>>,
    /// Resolved import specifiers to module ids (keyed by (relative_module, specifier, edge, loader)).
    /// The edge and loader components distinguish require-style imports and non-default loaders.
    pub imported_modules: Arc<
        IndexMap<
            (Option<ModuleId>, StringId, ImportEdgeKind, Option<Loader>),
            dir::ModuleResolution,
        >,
    >,
    /// Exported symbols by key (space, name).
    pub exported_symbols: Arc<IndexMap<(dir::SymbolSpace, dir::StaticKey), dir::Export>>,
}

impl ImportDir {
    /// Create a stable anchor node for module-level diagnostics.
    fn create_anchor_node(
        tree: &mut dir::NodeTree,
        scope_id: dir::LocalScopeId,
        anchor_source_id: u32,
    ) -> dir::LocalNodeIdAny {
        // anchor nodes should always point to a real AST id
        let scope = (scope_id, dir::LocalScopeMark::end());
        let anchor_slot =
            tree.reserve_from_source(dir::NodeType::Expression, anchor_source_id, scope, None);
        let expression = dir::Expression::TypeLiteral {
            value: dir::TypeLiteral::Void,
        };
        tree.insert(anchor_slot, expression).into_any()
    }

    /// Create one base import DIR with the chosen default symbol kind.
    fn new_with_default_symbol(
        id: ModuleId,
        version: ModuleVersion,
        anchor_source_id: u32,
        default_symbol_kind: dir::SymbolKind,
    ) -> Self {
        // set up the namespace, scopes, and default symbols
        let mut symbols = dir::SymbolTable::new(id);
        let namespace_scope_id = symbols.insert_scope(dir::ScopeKind::Namespace, None, None);
        let global_augmentation_scope_id = symbols.insert_scope(
            dir::ScopeKind::Namespace,
            Some((namespace_scope_id, dir::LocalScopeMark::end())),
            None,
        );
        let (namespace_symbol_id, _) = symbols.insert_symbol(
            dir::SymbolKind::Namespace,
            dir::SymbolType::Void,
            dir::SymbolSpace::Value,
            dir::SymbolBinding::Runtime,
            None,
            (namespace_scope_id, dir::LocalScopeMark::end()),
            Some(dir::DependencyMode::Namespace),
        );
        symbols.get_scope_by_id_mut(namespace_scope_id).owner_id = Some(namespace_symbol_id);
        let (default_symbol_id, _) = symbols.insert_symbol(
            default_symbol_kind,
            dir::SymbolType::Void,
            dir::SymbolSpace::Value,
            dir::SymbolBinding::Runtime,
            None,
            (namespace_scope_id, dir::LocalScopeMark::end()),
            Some(dir::DependencyMode::Default),
        );
        let (export_assignment_symbol_id, _) = symbols.insert_symbol(
            dir::SymbolKind::Namespace,
            dir::SymbolType::Void,
            dir::SymbolSpace::Value,
            dir::SymbolBinding::Runtime,
            None,
            (namespace_scope_id, dir::LocalScopeMark::end()),
            None,
        );

        // create a stable anchor node for diagnostics
        let mut tree = dir::NodeTree::new(id);
        let anchor_node = Self::create_anchor_node(&mut tree, namespace_scope_id, anchor_source_id);

        Self {
            profile_id: None,
            id,
            version,
            tree,
            symbols,
            types: dir::TypeTable::new(id),
            captures: dir::CaptureTable::new(),
            roots: Vec::new(),
            anchor_node,
            import_meta: None,
            namespace_symbol: namespace_symbol_id,
            namespace_scope: namespace_scope_id,
            global_augmentation_scope: global_augmentation_scope_id,
            default_symbol: default_symbol_id,
            export_assignment_symbol: export_assignment_symbol_id,
            export_assignment: None,
            namespace_exports: Vec::new(),
            module_bindings: Vec::new(),
            module_binding_exports: IndexMap::new(),
            imported_modules: IndexMap::new(),
            exported_symbols: IndexMap::new(),
        }
    }

    /// Create a new base import DIR.
    pub fn new_base(id: ModuleId, version: ModuleVersion, anchor_source_id: u32) -> Self {
        Self::new_with_default_symbol(id, version, anchor_source_id, dir::SymbolKind::Namespace)
    }

    /// Create a minimal base import DIR for data modules.
    ///
    /// Data modules have a simpler structure than code modules:
    /// - No syntax tree, but a diagnostic anchor is still provided
    /// - Single default export (the data value itself)
    /// - No named exports
    pub fn new_data_base(id: ModuleId, version: ModuleVersion, anchor_source_id: u32) -> Self {
        Self::new_with_default_symbol(id, version, anchor_source_id, dir::SymbolKind::Item)
    }

    /// Freeze this import DIR into one published DIR artifact.
    pub fn into_dir(self) -> ModuleDir {
        let Self {
            profile_id,
            id,
            version,
            tree,
            symbols,
            types,
            captures,
            roots,
            anchor_node,
            import_meta,
            namespace_symbol,
            namespace_scope,
            global_augmentation_scope,
            default_symbol,
            export_assignment_symbol,
            export_assignment,
            namespace_exports,
            module_bindings,
            module_binding_exports,
            imported_modules,
            exported_symbols,
        } = self;

        ModuleDir {
            profile_id,
            id,
            version,
            tree: Arc::new(tree),
            symbols: Arc::new(symbols),
            types: Arc::new(types),
            captures: Arc::new(captures),
            roots: Arc::new(roots),
            anchor_node,
            import_meta,
            namespace_symbol,
            namespace_scope,
            global_augmentation_scope,
            default_symbol,
            export_assignment_symbol,
            export_assignment,
            namespace_exports: Arc::new(namespace_exports),
            module_bindings: Arc::new(module_bindings),
            module_binding_exports: Arc::new(module_binding_exports),
            imported_modules: Arc::new(imported_modules),
            exported_symbols: Arc::new(exported_symbols),
        }
    }
}

impl ModuleDir {
    /// Clone a profile-dependent DIR from a base DIR.
    pub fn from_base(base: &ModuleDir, profile_id: ProfileId) -> Self {
        if base.profile_id.is_some() {
            panic!("expected base DIR for module {id:?}", id = base.id);
        }
        Self {
            profile_id: Some(profile_id),
            id: base.id,
            version: base.version,
            tree: base.tree.clone(),
            symbols: base.symbols.clone(),
            types: base.types.clone(),
            captures: base.captures.clone(),
            roots: base.roots.clone(),
            anchor_node: base.anchor_node,
            import_meta: None,
            namespace_symbol: base.namespace_symbol,
            namespace_scope: base.namespace_scope,
            global_augmentation_scope: base.global_augmentation_scope,
            default_symbol: base.default_symbol,
            export_assignment_symbol: base.export_assignment_symbol,
            export_assignment: base.export_assignment,
            namespace_exports: base.namespace_exports.clone(),
            module_bindings: base.module_bindings.clone(),
            module_binding_exports: base.module_binding_exports.clone(),
            imported_modules: base.imported_modules.clone(),
            exported_symbols: base.exported_symbols.clone(),
        }
    }

    /// Return the mutable node tree, cloning only when still shared.
    pub fn tree_mut(&mut self) -> &mut dir::NodeTree {
        Arc::make_mut(&mut self.tree)
    }

    /// Return the mutable tree, symbols, and types together for one phase-local working set.
    pub fn tree_symbols_types_mut(
        &mut self,
    ) -> (
        &mut dir::NodeTree,
        &mut dir::SymbolTable,
        &mut dir::TypeTable,
    ) {
        let Self {
            tree,
            symbols,
            types,
            ..
        } = self;

        (
            Arc::make_mut(tree),
            Arc::make_mut(symbols),
            Arc::make_mut(types),
        )
    }

    /// Return the mutable symbol table, cloning only when still shared.
    pub fn symbols_mut(&mut self) -> &mut dir::SymbolTable {
        Arc::make_mut(&mut self.symbols)
    }

    /// Return the mutable type table, cloning only when still shared.
    pub fn types_mut(&mut self) -> &mut dir::TypeTable {
        Arc::make_mut(&mut self.types)
    }

    /// Return the mutable capture table, cloning only when still shared.
    pub fn captures_mut(&mut self) -> &mut dir::CaptureTable {
        Arc::make_mut(&mut self.captures)
    }

    /// Return the mutable roots list, cloning only when still shared.
    pub fn roots_mut(&mut self) -> &mut Vec<dir::LocalNodeId<dir::Expression>> {
        Arc::make_mut(&mut self.roots)
    }

    /// Return the mutable namespace exports, cloning only when still shared.
    pub fn namespace_exports_mut(&mut self) -> &mut Vec<dir::NamespaceExport> {
        Arc::make_mut(&mut self.namespace_exports)
    }

    /// Return the mutable module bindings, cloning only when still shared.
    pub fn module_bindings_mut(&mut self) -> &mut Vec<dir::ModuleBinding> {
        Arc::make_mut(&mut self.module_bindings)
    }

    /// Return the mutable binding export tables, cloning only when still shared.
    pub fn module_binding_exports_mut(
        &mut self,
    ) -> &mut IndexMap<dir::LocalNodeIdAny, dir::ModuleBindingExports> {
        Arc::make_mut(&mut self.module_binding_exports)
    }

    /// Return the mutable imported-module table, cloning only when still shared.
    pub fn imported_modules_mut(
        &mut self,
    ) -> &mut IndexMap<
        (Option<ModuleId>, StringId, ImportEdgeKind, Option<Loader>),
        dir::ModuleResolution,
    > {
        Arc::make_mut(&mut self.imported_modules)
    }

    /// Return the mutable exported-symbol table, cloning only when still shared.
    pub fn exported_symbols_mut(
        &mut self,
    ) -> &mut IndexMap<(dir::SymbolSpace, dir::StaticKey), dir::Export> {
        Arc::make_mut(&mut self.exported_symbols)
    }
}
