use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use parking_lot::RwLock;

use destack_source::{FileId, FileVersion, LanguageType, ModuleId, ModuleVersion, PackageId, Uri};

use crate::{ModuleAst, ModuleDir, ModuleMir, ModuleType, TsConfigId};

/// A Module is a single source unit.
/// Destack treats all modules as "strict mode".
#[derive(Debug)]
pub struct Module {
    /// The id of the Module itself.
    pub id: ModuleId,
    /// The version of the Module (increments on each recompilation).
    pub version: ModuleVersion,
    /// The version of the source File this module was compiled from.
    pub source_version: FileVersion,
    /// The underlying source File (might be empty if placeholder or synthetic module).
    pub file_id: FileId,
    /// The URI of the Module.
    pub uri: Uri,
    /// The path to the Module.
    pub path: Option<PathBuf>,
    /// The package of the Module (every module belongs to a package).
    pub package_id: PackageId,
    /// The tsconfig of the Module (if any).
    pub tsconfig_id: Option<TsConfigId>,
    /// The source type of the Module (Script vs Module).
    pub module_type: ModuleType,
    /// The language type of the Module (Destack, TypeScript, JavaScript, etc.).
    pub language_type: LanguageType,

    // nocheckin
    /// The AST-level module data (syntactic).
    pub ast: ModuleAst,
    /// The DIR-level module data (semantic, target-independent).
    pub dir: ModuleDir,
    /// The MIR-level module data (target-specific).
    pub mir: ModuleMir,
}

#[allow(clippy::too_many_arguments)]
impl Module {
    /// Create a new blank Module (no AST yet, will be populated by Import).
    pub fn blank(
        id: ModuleId,
        file_id: FileId,
        source_version: FileVersion,
        uri: Uri,
        path: Option<PathBuf>,
        package_id: PackageId,
        tsconfig_id: Option<TsConfigId>,
        module_type: ModuleType,
        language_type: LanguageType,
    ) -> Self {
        Self {
            id,
            version: ModuleVersion::INITIAL,
            source_version,
            file_id,
            uri,
            path,
            package_id,
            tsconfig_id,
            module_type,
            language_type,
            ast: ModuleAst::new(id),
            dir: ModuleDir::new(id),
            mir: ModuleMir::new(id),
        }
    }

    /// Create a new Module from an AST.
    pub fn from_ast(
        id: ModuleId,
        file_id: FileId,
        source_version: FileVersion,
        uri: Uri,
        path: Option<PathBuf>,
        package_id: PackageId,
        tsconfig_id: Option<TsConfigId>,
        module_type: ModuleType,
        language_type: LanguageType,
        ast: ModuleAst,
    ) -> Self {
        let dir = ModuleDir::new(id);
        let mir = ModuleMir::new(id);
        Self {
            id,
            version: ModuleVersion::INITIAL,
            source_version,
            file_id,
            uri,
            path,
            package_id,
            tsconfig_id,
            module_type,
            language_type,
            ast,
            dir,
            mir,
        }
    }

    /// Check if this module has been parsed (has AST content).
    pub fn is_parsed(&self) -> bool {
        // nocheckin
        !self.ast.roots.is_empty() || !self.ast.tree.is_empty()
    }

    /// Reset the module for recompilation.
    /// Clears AST, DIR, and MIR data, increments version, and updates source_version.
    pub fn reset(&mut self, _new_source_version: FileVersion) {
        todo!("#Suspicious: revisit Module::reset"); // (use versions and task dependencies..?)
        // self.version = self.version.next();
        // self.source_version = new_source_version;
        // self.ast = ModuleAst::new(self.id);
        // self.dir = ModuleDir::new(self.id);
        // self.mir = ModuleMir::new(self.id);
    }

    /// Check if this module is stale (source file has changed since compilation).
    pub fn is_stale(&self, file_version: FileVersion) -> bool {
        self.source_version != file_version
    }
}

/// Registry of Modules. THREAD-SAFE.
#[derive(Debug)]
pub struct ModuleRegistry {
    /// The modules by id.
    modules_by_id: DashMap<ModuleId, Arc<RwLock<Module>>>,
    /// URI-based index for looking up modules by their URI.
    modules_by_uri: DashMap<Uri, ModuleId>,
    /// Path-based index for looking up modules by their path (only for modules with valid paths).
    modules_by_path: DashMap<PathBuf, ModuleId>,
    /// FileId-based index for looking up modules by their source file id.
    modules_by_file_id: DashMap<FileId, ModuleId>,
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleRegistry {
    /// Create a new ModuleRegistry.
    pub fn new() -> Self {
        Self {
            modules_by_id: DashMap::new(),
            modules_by_uri: DashMap::new(),
            modules_by_path: DashMap::new(),
            modules_by_file_id: DashMap::new(),
        }
    }

    /// Insert a module into the registry.
    pub fn insert(&self, module: Module) {
        let uri = module.uri.clone();
        let path = module.path.clone();
        let file_id = module.file_id;
        let id = module.id;
        self.modules_by_id.insert(id, Arc::new(RwLock::new(module)));
        self.modules_by_uri.insert(uri, id);
        self.modules_by_file_id.insert(file_id, id);
        if let Some(path) = path {
            self.modules_by_path.insert(path, id);
        }
    }

    /// Check if a module exists by id.
    pub fn contains(&self, id: ModuleId) -> bool {
        self.modules_by_id.contains_key(&id)
    }

    /// Get a module by module id.
    ///
    /// # Panics
    /// Panics if the module is not found.
    #[inline]
    pub fn get(&self, id: ModuleId) -> Arc<RwLock<Module>> {
        self.modules_by_id
            .get(&id)
            .unwrap_or_else(|| panic!("module not found for id: {id:?}"))
            .clone()
    }

    /// Get a module id by its URI.
    pub fn get_id_by_uri(&self, uri: &Uri) -> Option<ModuleId> {
        self.modules_by_uri.get(uri).map(|r| *r.value())
    }

    /// Get a module by its URI.
    pub fn get_by_uri(&self, uri: &Uri) -> Option<Arc<RwLock<Module>>> {
        let id = self.get_id_by_uri(uri)?;
        Some(self.get(id))
    }

    /// Check if a module exists at the given URI.
    pub fn contains_uri(&self, uri: &Uri) -> bool {
        self.modules_by_uri.contains_key(uri)
    }

    /// Get a module id by its path.
    pub fn get_id_by_path(&self, path: &Path) -> Option<ModuleId> {
        self.modules_by_path.get(path).map(|r| *r.value())
    }

    /// Get a module by its path.
    pub fn get_by_path(&self, path: &Path) -> Option<Arc<RwLock<Module>>> {
        let id = self.get_id_by_path(path)?;
        Some(self.get(id))
    }

    /// Check if a module exists at the given path.
    pub fn contains_path(&self, path: &Path) -> bool {
        self.modules_by_path.contains_key(path)
    }

    /// Get a module id by its source file id.
    pub fn get_id_by_file_id(&self, file_id: FileId) -> Option<ModuleId> {
        self.modules_by_file_id.get(&file_id).map(|r| *r.value())
    }

    /// Get a module by its source file id.
    pub fn get_by_file_id(&self, file_id: FileId) -> Option<Arc<RwLock<Module>>> {
        let id = self.get_id_by_file_id(file_id)?;
        Some(self.get(id))
    }

    /// Iterate over the modules in the registry.
    pub fn iter(&self) -> impl Iterator<Item = Arc<RwLock<Module>>> {
        let snapshot: Vec<_> = self
            .modules_by_id
            .iter()
            .map(|r| r.value().clone())
            .collect();
        snapshot.into_iter()
    }

    /// Get the number of modules in the registry.
    pub fn len(&self) -> usize {
        self.modules_by_id.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.modules_by_id.is_empty()
    }
}
