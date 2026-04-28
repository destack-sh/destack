use destack_core::StringId;
use destack_dir as dir;
use destack_source::{Loader, ModuleEdgeRelation, ModuleId, ProfileId};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// The module-binding export table keyed by the owning declaration node.
pub type ModuleBindingExportsByNode = IndexMap<dir::LocalNodeIdAny, dir::ModuleBindingExports>;

/// The exported-symbol table keyed by symbol space and static key.
pub type ExportBySymbolKey = IndexMap<(dir::SymbolSpace, dir::StaticKey), dir::Export>;

/// Key for one resolved import specifier edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImportResolutionKey {
    /// The source module when the edge is module-relative.
    pub source_module: Option<ModuleId>,
    /// The static import specifier.
    pub specifier: StringId,
    /// The import edge relation.
    pub relation: ModuleEdgeRelation,
    /// The loader override selected for the import.
    pub loader: Option<Loader>,
}

impl ImportResolutionKey {
    /// Create one import resolution key.
    pub fn new(
        source_module: Option<ModuleId>,
        specifier: StringId,
        relation: ModuleEdgeRelation,
        loader: Option<Loader>,
    ) -> Self {
        Self {
            source_module,
            specifier,
            relation,
            loader,
        }
    }
}

/// The import-resolution table keyed by resolved import edge.
pub type ImportResolutionTable = IndexMap<ImportResolutionKey, dir::ModuleResolution>;

/// Local declaration DIR for one module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirDeclared {
    /// The module id.
    pub id: ModuleId,
    /// The profile id this declaration artifact targets.
    pub profile_id: ProfileId,
    /// The declared DIR tree.
    pub tree: dir::Tree,
    /// The symbol side table.
    pub symbols: dir::SymbolTable,
    /// The declared type table.
    pub types: dir::TypeTable,
    /// The top-level expressions.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,
    /// Stable fallback node for diagnostics and synthetic types.
    pub anchor_node: dir::LocalNodeIdAny,
    /// The module namespace symbol.
    pub namespace_symbol: dir::LocalSymbolId,
    /// The module namespace scope.
    pub namespace_scope: dir::LocalScopeId,
    /// The global augmentation scope within this module.
    pub global_augmentation_scope: dir::LocalScopeId,
    /// The module default symbol.
    pub default_symbol: dir::LocalSymbolId,
    /// The module export assignment symbol.
    pub export_assignment_symbol: dir::LocalSymbolId,
    /// Export assignment item when present.
    pub export_assignment: Option<dir::LocalNodeId<dir::DependencyItem>>,
    /// Module bindings declared in the module.
    pub module_bindings: Vec<dir::ModuleBinding>,
}

impl DirDeclared {
    /// Create a stable anchor node for module-level diagnostics.
    fn create_anchor_node(
        tree: &mut dir::Tree,
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

    /// Create one empty declared DIR artifact with the chosen default symbol kind.
    fn new_with_default_symbol(
        id: ModuleId,
        profile_id: ProfileId,
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
        let mut tree = dir::Tree::new(id);
        let anchor_node = Self::create_anchor_node(&mut tree, namespace_scope_id, anchor_source_id);

        Self {
            id,
            profile_id,
            tree,
            symbols,
            types: dir::TypeTable::new(id),
            roots: Vec::new(),
            anchor_node,
            namespace_symbol: namespace_symbol_id,
            namespace_scope: namespace_scope_id,
            global_augmentation_scope: global_augmentation_scope_id,
            default_symbol: default_symbol_id,
            export_assignment_symbol: export_assignment_symbol_id,
            export_assignment: None,
            module_bindings: Vec::new(),
        }
    }

    /// Create a new empty declared DIR artifact.
    pub fn new(id: ModuleId, profile_id: ProfileId, anchor_source_id: u32) -> Self {
        Self::new_with_default_symbol(id, profile_id, anchor_source_id, dir::SymbolKind::Namespace)
    }

    /// Create a minimal declared DIR artifact for data modules.
    pub fn new_data(id: ModuleId, profile_id: ProfileId, anchor_source_id: u32) -> Self {
        Self::new_with_default_symbol(id, profile_id, anchor_source_id, dir::SymbolKind::Item)
    }
}

/// Exported module surface for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirExported {
    /// The module id.
    pub id: ModuleId,
    /// The profile id this export surface targets.
    pub profile_id: ProfileId,
    /// Profile-specific structural patch over the declared DIR.
    pub patch: dir::Patch,
    /// The visible top-level expressions.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,
    /// Stable fallback node for diagnostics and synthetic module edges.
    pub anchor_node: dir::LocalNodeIdAny,
    /// The module namespace symbol.
    pub namespace_symbol: dir::LocalSymbolId,
    /// The module namespace scope.
    pub namespace_scope: dir::LocalScopeId,
    /// The global augmentation scope within this module.
    pub global_augmentation_scope: dir::LocalScopeId,
    /// The module default symbol.
    pub default_symbol: dir::LocalSymbolId,
    /// The module export assignment symbol.
    pub export_assignment_symbol: dir::LocalSymbolId,
    /// Export assignment item when present.
    pub export_assignment: Option<dir::LocalNodeId<dir::DependencyItem>>,
    /// Module bindings declared in the module.
    pub module_bindings: Vec<dir::ModuleBinding>,
    /// Namespace exports declared in the module.
    pub namespace_exports: Vec<dir::NamespaceExport>,
    /// Export data for declared module bindings by declaration node.
    pub module_binding_exports_by_node: ModuleBindingExportsByNode,
    /// Resolved import targets by import key.
    pub import_resolutions: ImportResolutionTable,
    /// Export data by exported symbol key.
    pub export_by_symbol_key: ExportBySymbolKey,
}

impl DirExported {
    /// Build one exported DIR artifact from a declared artifact and exported locals.
    pub fn from_declared_with(
        declared: &DirDeclared,
        patch: dir::Patch,
        roots: Vec<dir::LocalNodeId<dir::Expression>>,
        export_assignment: Option<dir::LocalNodeId<dir::DependencyItem>>,
        namespace_exports: Vec<dir::NamespaceExport>,
        module_binding_exports_by_node: ModuleBindingExportsByNode,
        import_resolutions: ImportResolutionTable,
        export_by_symbol_key: ExportBySymbolKey,
    ) -> Self {
        Self {
            id: declared.id,
            profile_id: declared.profile_id,
            patch,
            roots,
            anchor_node: declared.anchor_node,
            namespace_symbol: declared.namespace_symbol,
            namespace_scope: declared.namespace_scope,
            global_augmentation_scope: declared.global_augmentation_scope,
            default_symbol: declared.default_symbol,
            export_assignment_symbol: declared.export_assignment_symbol,
            export_assignment,
            module_bindings: declared.module_bindings.clone(),
            namespace_exports,
            module_binding_exports_by_node,
            import_resolutions,
            export_by_symbol_key,
        }
    }
}

/// Checked local body semantics for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirChecked {
    /// The module id.
    pub id: ModuleId,
    /// The profile id this checked artifact targets.
    pub profile_id: ProfileId,
    /// Durable structural patch applied after declaration.
    pub patch: dir::Patch,
    /// Checked type table segment.
    pub types: dir::TypeTable,
    /// Capture side table.
    pub captures: dir::CaptureTable,
}

/// Elaborated local body semantics for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirElaborated {
    /// The module id.
    pub id: ModuleId,
    /// The profile id this elaborated artifact targets.
    pub profile_id: ProfileId,
    /// Durable structural patch applied after checking.
    pub patch: dir::Patch,
    /// Elaborated type table segment.
    pub types: dir::TypeTable,
    /// Elaborated type guard strategies.
    pub guards: dir::GuardTable,
}
