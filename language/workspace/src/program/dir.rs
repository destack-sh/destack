use destack_base::StringId;
use destack_dir::{self as dir};
use destack_source::{ModuleId, ModuleVersion};
use indexmap::IndexMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::{ImportMeta, Loader, ProfileId};

/// DIR-level module data.
/// The base DIR uses `profile_id: None`.
#[derive(Debug)]
#[allow(clippy::type_complexity)]
pub struct ModuleDir {
    /// The profile id this is targeting, if any.
    pub profile_id: Option<ProfileId>,
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
    pub export_assignment: RwLock<Option<dir::LocalNodeId<dir::DependencyItem>>>,
    /// Namespace exports: modules whose exports are re-exported via `export * from "..."`.
    pub namespace_exports: RwLock<Vec<dir::NamespaceExport>>,
    /// Module bindings (e.g., `declare module "foo"`).
    pub module_bindings: RwLock<Vec<dir::ModuleBinding>>,
    /// Export tables for module bindings.
    pub module_binding_exports: RwLock<IndexMap<dir::LocalNodeIdAny, dir::ModuleBindingExports>>,
    /// Resolved import specifiers to module ids (keyed by (relative_module, specifier, loader)).
    /// The loader component distinguishes imports with non-default loaders.
    pub imported_modules:
        RwLock<IndexMap<(Option<ModuleId>, StringId, Option<Loader>), dir::ModuleTarget>>,
    /// Exported symbols by key (space, name).
    pub exported_symbols: RwLock<IndexMap<(dir::SymbolSpace, dir::StaticKey), dir::Export>>,
}

/// Serializable snapshot of ModuleDir data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::type_complexity)]
pub struct ModuleDirData {
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
    /// The type side table of the Module (includes types, instances, resolutions).
    pub types: dir::TypeTable,
    /// The top-level expressions of the Module.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,

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
    /// Resolved import specifiers to module ids (keyed by (relative_module, specifier, loader)).
    /// The loader component distinguishes imports with non-default loaders.
    pub imported_modules: IndexMap<(Option<ModuleId>, StringId, Option<Loader>), dir::ModuleTarget>,
    /// Exported symbols by key (space, name).
    pub exported_symbols: IndexMap<(dir::SymbolSpace, dir::StaticKey), dir::Export>,
}

impl ModuleDir {
    /// Create a new base DIR.
    pub fn new_base(id: ModuleId, version: ModuleVersion) -> Self {
        // set up default namespace, symbol, scopes, etc.
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
            dir::SymbolKind::Namespace,
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

        Self {
            profile_id: None,
            id,
            version,
            tree: RwLock::new(dir::NodeTree::new(id)),
            symbols: RwLock::new(symbols),
            types: RwLock::new(dir::TypeTable::new(id)),
            roots: Vec::new(),
            import_meta: None,
            namespace_symbol: namespace_symbol_id,
            namespace_scope: namespace_scope_id,
            global_augmentation_scope: global_augmentation_scope_id,
            default_symbol: default_symbol_id,
            export_assignment_symbol: export_assignment_symbol_id,
            export_assignment: RwLock::new(None),
            namespace_exports: RwLock::new(Vec::new()),
            module_bindings: RwLock::new(Vec::new()),
            module_binding_exports: RwLock::new(IndexMap::new()),
            imported_modules: RwLock::new(IndexMap::new()),
            exported_symbols: RwLock::new(IndexMap::new()),
        }
    }

    /// Create a minimal base DIR for data modules (JSON, TOML, text, binary).
    ///
    /// Data modules have a simpler structure than code modules:
    /// - No AST to parse
    /// - Single default export (the data value itself)
    /// - No named exports
    pub fn new_data_base(id: ModuleId, version: ModuleVersion) -> Self {
        // create minimal symbol table with namespace and default symbols
        let mut symbols = dir::SymbolTable::new(id);
        let namespace_scope_id = symbols.insert_scope(dir::ScopeKind::Namespace, None, None);
        let global_augmentation_scope_id = symbols.insert_scope(
            dir::ScopeKind::Namespace,
            Some((namespace_scope_id, dir::LocalScopeMark::end())),
            None,
        );

        // namespace symbol
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

        // default symbol - this is what gets exported as `default`
        let (default_symbol_id, _) = symbols.insert_symbol(
            dir::SymbolKind::Item,
            dir::SymbolType::Void,
            dir::SymbolSpace::Value,
            dir::SymbolBinding::Runtime,
            None,
            (namespace_scope_id, dir::LocalScopeMark::end()),
            Some(dir::DependencyMode::Default),
        );

        // export assignment symbol (not used for data modules, but needed for structure)
        let (export_assignment_symbol_id, _) = symbols.insert_symbol(
            dir::SymbolKind::Namespace,
            dir::SymbolType::Void,
            dir::SymbolSpace::Value,
            dir::SymbolBinding::Runtime,
            None,
            (namespace_scope_id, dir::LocalScopeMark::end()),
            None,
        );

        // (exported_symbols will be populated during resolve phase when string pool is available)

        Self {
            profile_id: None,
            id,
            version,
            tree: RwLock::new(dir::NodeTree::new(id)),
            symbols: RwLock::new(symbols),
            types: RwLock::new(dir::TypeTable::new(id)),
            roots: Vec::new(),
            import_meta: None,
            namespace_symbol: namespace_symbol_id,
            namespace_scope: namespace_scope_id,
            global_augmentation_scope: global_augmentation_scope_id,
            default_symbol: default_symbol_id,
            export_assignment_symbol: export_assignment_symbol_id,
            export_assignment: RwLock::new(None),
            namespace_exports: RwLock::new(Vec::new()),
            module_bindings: RwLock::new(Vec::new()),
            module_binding_exports: RwLock::new(IndexMap::new()),
            imported_modules: RwLock::new(IndexMap::new()),
            exported_symbols: RwLock::new(IndexMap::new()),
        }
    }

    /// Clone a profile-dependent DIR from a base DIR.
    pub fn from_base(base: &ModuleDir, profile_id: ProfileId) -> Self {
        if base.profile_id.is_some() {
            panic!("expected base DIR for module {id:?}", id = base.id);
        }
        Self {
            profile_id: Some(profile_id),
            id: base.id,
            version: base.version,
            tree: RwLock::new(base.tree.read().clone()),
            symbols: RwLock::new(base.symbols.read().clone()),
            types: RwLock::new(base.types.read().clone()),
            roots: base.roots.clone(),
            import_meta: None,
            namespace_symbol: base.namespace_symbol,
            namespace_scope: base.namespace_scope,
            global_augmentation_scope: base.global_augmentation_scope,
            default_symbol: base.default_symbol,
            export_assignment_symbol: base.export_assignment_symbol,
            export_assignment: RwLock::new(*base.export_assignment.read()),
            namespace_exports: RwLock::new(base.namespace_exports.read().clone()),
            module_bindings: RwLock::new(base.module_bindings.read().clone()),
            module_binding_exports: RwLock::new(base.module_binding_exports.read().clone()),
            imported_modules: RwLock::new(base.imported_modules.read().clone()),
            exported_symbols: RwLock::new(base.exported_symbols.read().clone()),
        }
    }

    /// Create a serializable snapshot of this module dir.
    pub fn to_data(&self) -> ModuleDirData {
        // snapshot module dir state
        ModuleDirData {
            profile_id: self.profile_id,
            id: self.id,
            version: self.version,
            tree: self.tree.read().clone(),
            symbols: self.symbols.read().clone(),
            types: self.types.read().clone(),
            roots: self.roots.clone(),
            import_meta: self.import_meta.clone(),
            namespace_symbol: self.namespace_symbol,
            namespace_scope: self.namespace_scope,
            global_augmentation_scope: self.global_augmentation_scope,
            default_symbol: self.default_symbol,
            export_assignment_symbol: self.export_assignment_symbol,
            export_assignment: *self.export_assignment.read(),
            namespace_exports: self.namespace_exports.read().clone(),
            module_bindings: self.module_bindings.read().clone(),
            module_binding_exports: self.module_binding_exports.read().clone(),
            imported_modules: self.imported_modules.read().clone(),
            exported_symbols: self.exported_symbols.read().clone(),
        }
    }

    /// Rebuild a ModuleDir from serialized data.
    pub fn from_data(data: ModuleDirData) -> Self {
        // rebuild module dir from snapshot
        Self {
            profile_id: data.profile_id,
            id: data.id,
            version: data.version,
            tree: RwLock::new(data.tree),
            symbols: RwLock::new(data.symbols),
            types: RwLock::new(data.types),
            roots: data.roots,
            import_meta: data.import_meta,
            namespace_symbol: data.namespace_symbol,
            namespace_scope: data.namespace_scope,
            global_augmentation_scope: data.global_augmentation_scope,
            default_symbol: data.default_symbol,
            export_assignment_symbol: data.export_assignment_symbol,
            export_assignment: RwLock::new(data.export_assignment),
            namespace_exports: RwLock::new(data.namespace_exports),
            module_bindings: RwLock::new(data.module_bindings),
            module_binding_exports: RwLock::new(data.module_binding_exports),
            imported_modules: RwLock::new(data.imported_modules),
            exported_symbols: RwLock::new(data.exported_symbols),
        }
    }
}

impl Serialize for ModuleDir {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_data().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ModuleDir {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = ModuleDirData::deserialize(deserializer)?;
        Ok(ModuleDir::from_data(data))
    }
}
