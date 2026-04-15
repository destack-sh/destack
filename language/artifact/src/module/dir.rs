use std::sync::Arc;

use destack_core::StringId;
use destack_dir as dir;
use destack_source::{ModuleId, ProfileId};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{Loader, ModuleEdgeRelation};

/// The module-binding export table keyed by the owning declaration node.
pub type ModuleBindingExportTable = IndexMap<dir::LocalNodeIdAny, dir::ModuleBindingExports>;

/// The resolved module-import table keyed by module-relative import identity.
pub type ImportedModuleTable = IndexMap<
    (
        Option<ModuleId>,
        StringId,
        ModuleEdgeRelation,
        Option<Loader>,
    ),
    dir::ModuleResolution,
>;

/// The exported-symbol table keyed by symbol space and static key.
pub type ExportedSymbolTable = IndexMap<(dir::SymbolSpace, dir::StaticKey), dir::Export>;

/// The bound base DIR for one module.
#[derive(destack_artifact_macros::Image, Debug, Clone, Serialize, Deserialize)]
pub struct DirBase {
    /// The id of the Module.
    pub id: ModuleId,
    /// The main DIR node tree of the Module.
    pub tree: Arc<dir::NodeTree>,
    /// The symbol side table of the Module.
    pub symbols: Arc<dir::SymbolTable>,
    /// The type side table of the Module.
    pub types: Arc<dir::TypeTable>,
    /// The top-level expressions of the Module.
    pub roots: Arc<Vec<dir::LocalNodeId<dir::Expression>>>,
    /// Stable fallback node for diagnostics and synthetic types.
    pub anchor_node: dir::LocalNodeIdAny,
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
    /// Module bindings declared in the module.
    pub module_bindings: Arc<Vec<dir::ModuleBinding>>,
}

impl DirBase {
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

    /// Create one base artifact with the chosen default symbol kind.
    fn new_with_default_symbol(
        id: ModuleId,
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
            Some(dir::ExportMode::Named),
        );
        symbols.get_scope_by_id_mut(namespace_scope_id).owner_id = Some(namespace_symbol_id);
        let (default_symbol_id, _) = symbols.insert_symbol(
            default_symbol_kind,
            dir::SymbolType::Void,
            dir::SymbolSpace::Value,
            dir::SymbolBinding::Runtime,
            None,
            (namespace_scope_id, dir::LocalScopeMark::end()),
            Some(dir::ExportMode::Default),
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
            id,
            tree: Arc::new(tree),
            symbols: Arc::new(symbols),
            types: Arc::new(dir::TypeTable::new(id)),
            roots: Arc::new(Vec::new()),
            anchor_node,
            namespace_symbol: namespace_symbol_id,
            namespace_scope: namespace_scope_id,
            global_augmentation_scope: global_augmentation_scope_id,
            default_symbol: default_symbol_id,
            export_assignment_symbol: export_assignment_symbol_id,
            module_bindings: Arc::new(Vec::new()),
        }
    }

    /// Create a new base DIR artifact.
    pub fn new_base(id: ModuleId, anchor_source_id: u32) -> Self {
        Self::new_with_default_symbol(id, anchor_source_id, dir::SymbolKind::Namespace)
    }

    /// Create a minimal base DIR artifact for data modules.
    pub fn new_data_base(id: ModuleId, anchor_source_id: u32) -> Self {
        Self::new_with_default_symbol(id, anchor_source_id, dir::SymbolKind::Item)
    }
}

/// The prepared DIR for one profile-scoped module.
#[derive(destack_artifact_macros::Image, Debug, Clone, Serialize, Deserialize)]
pub struct DirPrepared {
    /// The profile id this is targeting.
    pub profile_id: ProfileId,
    /// The id of the Module.
    pub id: ModuleId,
    /// The main DIR node tree of the Module.
    pub tree: Arc<dir::NodeTree>,
    /// The symbol side table of the Module.
    pub symbols: Arc<dir::SymbolTable>,
    /// The type side table of the Module.
    pub types: Arc<dir::TypeTable>,
    /// The top-level expressions of the Module.
    pub roots: Arc<Vec<dir::LocalNodeId<dir::Expression>>>,
    /// Stable fallback node for diagnostics and synthetic types.
    pub anchor_node: dir::LocalNodeIdAny,
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
    /// Module bindings declared in the module.
    pub module_bindings: Arc<Vec<dir::ModuleBinding>>,
    /// Export tables for module bindings.
    pub module_binding_exports: Arc<ModuleBindingExportTable>,
    /// Resolved imported modules.
    pub imported_modules: Arc<ImportedModuleTable>,
    /// Exported module symbols.
    pub exported_symbols: Arc<ExportedSymbolTable>,
}

impl DirPrepared {
    /// Build one prepared artifact from one base artifact and prepared locals.
    pub fn from_base_with(
        base: &DirBase,
        profile_id: ProfileId,
        tree: dir::NodeTree,
        symbols: dir::SymbolTable,
        roots: Vec<dir::LocalNodeId<dir::Expression>>,
        export_assignment: Option<dir::LocalNodeId<dir::DependencyItem>>,
        module_binding_exports: ModuleBindingExportTable,
        imported_modules: ImportedModuleTable,
        exported_symbols: ExportedSymbolTable,
    ) -> Self {
        Self {
            profile_id,
            id: base.id,
            tree: Arc::new(tree),
            symbols: Arc::new(symbols),
            types: base.types.clone(),
            roots: Arc::new(roots),
            anchor_node: base.anchor_node,
            namespace_symbol: base.namespace_symbol,
            namespace_scope: base.namespace_scope,
            global_augmentation_scope: base.global_augmentation_scope,
            default_symbol: base.default_symbol,
            export_assignment_symbol: base.export_assignment_symbol,
            export_assignment,
            module_bindings: base.module_bindings.clone(),
            module_binding_exports: Arc::new(module_binding_exports),
            imported_modules: Arc::new(imported_modules),
            exported_symbols: Arc::new(exported_symbols),
        }
    }
}

/// The resolved DIR for one profile-scoped module.
#[derive(destack_artifact_macros::Image, Debug, Clone, Serialize, Deserialize)]
pub struct DirResolved {
    /// The profile id this is targeting.
    pub profile_id: ProfileId,
    /// The id of the Module.
    pub id: ModuleId,
    /// The main DIR node tree of the Module.
    pub tree: Arc<dir::NodeTree>,
    /// The symbol side table of the Module.
    pub symbols: Arc<dir::SymbolTable>,
    /// The type side table of the Module.
    pub types: Arc<dir::TypeTable>,
    /// The top-level expressions of the Module.
    pub roots: Arc<Vec<dir::LocalNodeId<dir::Expression>>>,
    /// Stable fallback node for diagnostics and synthetic types.
    pub anchor_node: dir::LocalNodeIdAny,
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
    /// Namespace exports declared in the module.
    pub namespace_exports: Arc<Vec<dir::NamespaceExport>>,
    /// Export tables for module bindings.
    pub module_binding_exports: Arc<ModuleBindingExportTable>,
    /// Resolved imported modules.
    pub imported_modules: Arc<ImportedModuleTable>,
    /// Exported module symbols.
    pub exported_symbols: Arc<ExportedSymbolTable>,
}

impl DirResolved {
    /// Build one resolved artifact from one prepared artifact and resolved locals.
    pub fn from_prepared_with(
        prepared: &DirPrepared,
        tree: dir::NodeTree,
        symbols: dir::SymbolTable,
        types: dir::TypeTable,
        export_assignment: Option<dir::LocalNodeId<dir::DependencyItem>>,
        namespace_exports: Vec<dir::NamespaceExport>,
        module_binding_exports: ModuleBindingExportTable,
        imported_modules: ImportedModuleTable,
        exported_symbols: ExportedSymbolTable,
    ) -> Self {
        Self {
            profile_id: prepared.profile_id,
            id: prepared.id,
            tree: Arc::new(tree),
            symbols: Arc::new(symbols),
            types: Arc::new(types),
            roots: prepared.roots.clone(),
            anchor_node: prepared.anchor_node,
            namespace_symbol: prepared.namespace_symbol,
            namespace_scope: prepared.namespace_scope,
            global_augmentation_scope: prepared.global_augmentation_scope,
            default_symbol: prepared.default_symbol,
            export_assignment_symbol: prepared.export_assignment_symbol,
            export_assignment,
            namespace_exports: Arc::new(namespace_exports),
            module_binding_exports: Arc::new(module_binding_exports),
            imported_modules: Arc::new(imported_modules),
            exported_symbols: Arc::new(exported_symbols),
        }
    }
}

/// The declared DIR for one profile-scoped module.
#[derive(destack_artifact_macros::Image, Debug, Clone, Serialize, Deserialize)]
pub struct DirDeclared {
    /// The profile id this is targeting.
    pub profile_id: ProfileId,
    /// The id of the Module.
    pub id: ModuleId,
    /// The main DIR node tree of the Module.
    pub tree: Arc<dir::NodeTree>,
    /// The symbol side table of the Module.
    pub symbols: Arc<dir::SymbolTable>,
    /// The type side table of the Module.
    pub types: Arc<dir::TypeTable>,
    /// The capture side table of the Module.
    pub captures: Arc<dir::CaptureTable>,
    /// The top-level expressions of the Module.
    pub roots: Arc<Vec<dir::LocalNodeId<dir::Expression>>>,
    /// Stable fallback node for diagnostics and synthetic types.
    pub anchor_node: dir::LocalNodeIdAny,
    /// The symbol of the Module namespace.
    pub namespace_symbol: dir::LocalSymbolId,
    /// The scope of the Module.
    pub namespace_scope: dir::LocalScopeId,
    /// The symbol of the Module default.
    pub default_symbol: dir::LocalSymbolId,
    /// The symbol of the Module export assignment.
    pub export_assignment_symbol: dir::LocalSymbolId,
}

impl DirDeclared {
    /// Build one declared artifact from one resolved artifact and declared locals.
    pub fn from_resolved_with(
        resolved: &DirResolved,
        symbols: dir::SymbolTable,
        types: dir::TypeTable,
        captures: dir::CaptureTable,
    ) -> Self {
        Self {
            profile_id: resolved.profile_id,
            id: resolved.id,
            tree: resolved.tree.clone(),
            symbols: Arc::new(symbols),
            types: Arc::new(types),
            captures: Arc::new(captures),
            roots: resolved.roots.clone(),
            anchor_node: resolved.anchor_node,
            namespace_symbol: resolved.namespace_symbol,
            namespace_scope: resolved.namespace_scope,
            default_symbol: resolved.default_symbol,
            export_assignment_symbol: resolved.export_assignment_symbol,
        }
    }
}

/// The interface DIR for one profile-scoped module.
#[derive(destack_artifact_macros::Image, Debug, Clone, Serialize, Deserialize)]
pub struct DirInterface {
    /// The profile id this is targeting.
    pub profile_id: ProfileId,
    /// The id of the Module.
    pub id: ModuleId,
    /// The main DIR node tree of the Module.
    pub tree: Arc<dir::NodeTree>,
    /// The symbol side table of the Module.
    pub symbols: Arc<dir::SymbolTable>,
    /// The type side table of the Module.
    pub types: Arc<dir::TypeTable>,
    /// The top-level expressions of the Module.
    pub roots: Arc<Vec<dir::LocalNodeId<dir::Expression>>>,
    /// Stable fallback node for diagnostics and synthetic types.
    pub anchor_node: dir::LocalNodeIdAny,
    /// The scope of the Module.
    pub namespace_scope: dir::LocalScopeId,
    /// Namespace exports declared in the module.
    pub namespace_exports: Arc<Vec<dir::NamespaceExport>>,
    /// Export tables for module bindings.
    pub module_binding_exports: Arc<ModuleBindingExportTable>,
    /// Exported module symbols.
    pub exported_symbols: Arc<ExportedSymbolTable>,
}

impl DirInterface {
    /// Build one interface artifact from one resolved artifact and one declared artifact.
    pub fn from_resolved_and_declared(resolved: &DirResolved, declared: &DirDeclared) -> Self {
        Self {
            profile_id: declared.profile_id,
            id: declared.id,
            tree: declared.tree.clone(),
            symbols: declared.symbols.clone(),
            types: declared.types.clone(),
            roots: declared.roots.clone(),
            anchor_node: declared.anchor_node,
            namespace_scope: declared.namespace_scope,
            namespace_exports: resolved.namespace_exports.clone(),
            module_binding_exports: resolved.module_binding_exports.clone(),
            exported_symbols: resolved.exported_symbols.clone(),
        }
    }

    /// Build one interface artifact from one resolved artifact, one declared artifact, and interface types.
    pub fn from_resolved_and_declared_with_types(
        resolved: &DirResolved,
        declared: &DirDeclared,
        types: dir::TypeTable,
    ) -> Self {
        Self {
            types: Arc::new(types),
            ..Self::from_resolved_and_declared(resolved, declared)
        }
    }
}

/// The analyzed DIR for one profile-scoped module.
#[derive(destack_artifact_macros::Image, Debug, Clone, Serialize, Deserialize)]
pub struct DirAnalyzed {
    /// The profile id this is targeting.
    pub profile_id: ProfileId,
    /// The id of the Module.
    pub id: ModuleId,
    /// The main DIR node tree of the Module.
    pub tree: Arc<dir::NodeTree>,
    /// The symbol side table of the Module.
    pub symbols: Arc<dir::SymbolTable>,
    /// The type side table of the Module.
    pub types: Arc<dir::TypeTable>,
    /// The capture side table of the Module.
    pub captures: Arc<dir::CaptureTable>,
    /// The top-level expressions of the Module.
    pub roots: Arc<Vec<dir::LocalNodeId<dir::Expression>>>,
    /// Stable fallback node for diagnostics and synthetic types.
    pub anchor_node: dir::LocalNodeIdAny,
    /// The symbol of the Module namespace.
    pub namespace_symbol: dir::LocalSymbolId,
    /// The scope of the Module.
    pub namespace_scope: dir::LocalScopeId,
    /// The symbol of the Module default.
    pub default_symbol: dir::LocalSymbolId,
    /// The symbol of the Module export assignment.
    pub export_assignment_symbol: dir::LocalSymbolId,
}

impl DirAnalyzed {
    /// Build one analyzed artifact from one interface artifact, one declared artifact, and analyzed locals.
    pub fn from_interface_and_declared_with(
        interface: &DirInterface,
        declared: &DirDeclared,
        types: dir::TypeTable,
        captures: dir::CaptureTable,
    ) -> Self {
        Self {
            profile_id: interface.profile_id,
            id: interface.id,
            tree: interface.tree.clone(),
            symbols: interface.symbols.clone(),
            types: Arc::new(types),
            captures: Arc::new(captures),
            roots: interface.roots.clone(),
            anchor_node: interface.anchor_node,
            namespace_symbol: declared.namespace_symbol,
            namespace_scope: interface.namespace_scope,
            default_symbol: declared.default_symbol,
            export_assignment_symbol: declared.export_assignment_symbol,
        }
    }
}

/// The elaborated DIR for one profile-scoped module.
#[derive(destack_artifact_macros::Image, Debug, Clone, Serialize, Deserialize)]
pub struct DirElaborated {
    /// The profile id this is targeting.
    pub profile_id: ProfileId,
    /// The id of the Module.
    pub id: ModuleId,
    /// The main DIR node tree of the Module.
    pub tree: Arc<dir::NodeTree>,
    /// The symbol side table of the Module.
    pub symbols: Arc<dir::SymbolTable>,
    /// The type side table of the Module.
    pub types: Arc<dir::TypeTable>,
    /// The capture side table of the Module.
    pub captures: Arc<dir::CaptureTable>,
    /// The top-level expressions of the Module.
    pub roots: Arc<Vec<dir::LocalNodeId<dir::Expression>>>,
    /// Stable fallback node for diagnostics and synthetic types.
    pub anchor_node: dir::LocalNodeIdAny,
    /// The scope of the Module.
    pub namespace_scope: dir::LocalScopeId,
}

impl DirElaborated {
    /// Build one elaborated artifact from one analyzed artifact and elaborated locals.
    pub fn from_analyzed_with(
        analyzed: &DirAnalyzed,
        tree: dir::NodeTree,
        symbols: dir::SymbolTable,
        types: dir::TypeTable,
    ) -> Self {
        Self {
            profile_id: analyzed.profile_id,
            id: analyzed.id,
            tree: Arc::new(tree),
            symbols: Arc::new(symbols),
            types: Arc::new(types),
            captures: analyzed.captures.clone(),
            roots: analyzed.roots.clone(),
            anchor_node: analyzed.anchor_node,
            namespace_scope: analyzed.namespace_scope,
        }
    }
}

/// The patched DIR for one profile-scoped module.
#[derive(destack_artifact_macros::Image, Debug, Clone, Serialize, Deserialize)]
pub struct DirPatched {
    /// The profile id this is targeting.
    pub profile_id: ProfileId,
    /// The id of the Module.
    pub id: ModuleId,
    /// The main DIR node tree of the Module.
    pub tree: Arc<dir::NodeTree>,
    /// The symbol side table of the Module.
    pub symbols: Arc<dir::SymbolTable>,
    /// The type side table of the Module.
    pub types: Arc<dir::TypeTable>,
    /// The capture side table of the Module.
    pub captures: Arc<dir::CaptureTable>,
    /// The top-level expressions of the Module.
    pub roots: Arc<Vec<dir::LocalNodeId<dir::Expression>>>,
    /// Stable fallback node for diagnostics and synthetic types.
    pub anchor_node: dir::LocalNodeIdAny,
    /// The scope of the Module.
    pub namespace_scope: dir::LocalScopeId,
}

impl DirPatched {
    /// Build one patched artifact from one elaborated artifact and a patched tree.
    pub fn from_elaborated_with(elaborated: &DirElaborated, tree: dir::NodeTree) -> Self {
        Self {
            profile_id: elaborated.profile_id,
            id: elaborated.id,
            tree: Arc::new(tree),
            symbols: elaborated.symbols.clone(),
            types: elaborated.types.clone(),
            captures: elaborated.captures.clone(),
            roots: elaborated.roots.clone(),
            anchor_node: elaborated.anchor_node,
            namespace_scope: elaborated.namespace_scope,
        }
    }
}
