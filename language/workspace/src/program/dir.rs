use destack_base::StringId;
use destack_dir::{self as dir};
use destack_source::{ModuleId, ModuleVersion};
use indexmap::IndexMap;
use parking_lot::RwLock;

/// DIR-level module data.
#[derive(Debug)]
pub struct ModuleDir {
    /// The id of the Module.
    pub id: ModuleId,
    /// The version of the Module.
    pub version: ModuleVersion,

    /// The main DIR node tree of the Module.
    pub tree: RwLock<dir::NodeTree>,
    /// The symbol side table of the Module.
    pub symbols: RwLock<dir::SymbolTable>,
    /// The type side table of the Module (includes types, instances, resolutions).
    pub types: RwLock<dir::TypeTable>,
    /// The top-level expressions of the Module.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,

    /// The symbol of the Module namespace.
    pub namespace_symbol: dir::LocalSymbolId,
    /// The scope of the Module.
    pub namespace_scope: dir::LocalScopeId,
    /// The symbol of the Module default.
    pub default_symbol: dir::LocalSymbolId,
    /// Namespace exports: modules whose exports are re-exported via `export * from "..."`.
    pub namespace_exports: RwLock<Vec<ModuleId>>,
    /// Resolved import specifiers to module ids (keyed by (relative_module, specifier)).
    pub imported_modules: RwLock<IndexMap<(Option<ModuleId>, StringId), ModuleId>>,
    /// Exported symbols by key (space, name).
    pub exported_symbols: RwLock<IndexMap<(dir::SymbolSpace, dir::StaticKey), dir::LocalSymbolId>>,
}

impl ModuleDir {
    /// Create a new ModuleDir.
    pub fn new(id: ModuleId, version: ModuleVersion) -> Self {
        // set up default namespace and default symbol
        let mut symbols = dir::SymbolTable::new(id);
        let namespace_scope_id = symbols.insert_scope(dir::ScopeKind::Namespace, None, None);
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
            dir::SymbolKind::Namespace,
            dir::SymbolType::Void,
            dir::SymbolSpace::Value,
            dir::SymbolBinding::Runtime,
            None,
            (namespace_scope_id, dir::LocalScopeMark::end()),
            Some(dir::DependencyMode::Default),
        );

        Self {
            id,
            version,

            tree: RwLock::new(dir::NodeTree::new(id)),
            symbols: RwLock::new(symbols),
            types: RwLock::new(dir::TypeTable::new(id)),
            roots: Vec::new(),

            namespace_symbol: namespace_symbol_id,
            namespace_scope: namespace_scope_id,
            default_symbol: default_symbol_id,
            namespace_exports: RwLock::new(Vec::new()),
            imported_modules: RwLock::new(IndexMap::new()),
            exported_symbols: RwLock::new(IndexMap::new()),
        }
    }
}
