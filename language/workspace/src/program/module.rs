use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;

use destack_builtin::BuiltinLibKind;
use destack_source::{FileId, FileVersion, LanguageType, ModuleId, ModuleVersion, PackageId, Uri};

use crate::{Loader, ModuleTarget, SourceType, TsConfigId};

/// The source/origin of a module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleSource {
    /// User/project code.
    User,
    /// Builtin library code.
    Builtin(BuiltinLibKind),
}

/// The runtime module system format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ModuleFormat {
    /// ECMAScript module format.
    #[default]
    Esm,
    /// CommonJS module format.
    CommonJs,
}

impl ModuleFormat {
    /// Detect a module format from extension and package context.
    pub fn detect(
        path: Option<&Path>,
        language_type: LanguageType,
        source_type: SourceType,
        package_type: Option<&str>,
        tsconfig_format: Option<Self>,
    ) -> Self {
        // destack modules always use esm semantics
        if language_type.is_destack() {
            return Self::Esm;
        }

        // extension based module formats are authoritative
        if let Some(path) = path
            && let Some(format) = Self::from_extension(path)
        {
            return format;
        }

        // tsconfig module targets can force commonjs or esm semantics
        if let Some(format) = tsconfig_format {
            return format;
        }

        // package json type defines js or ts module format defaults
        if let Some(format) = Self::from_package_type(package_type) {
            return format;
        }

        // default typescript modules to esm semantics
        if language_type.is_typescript() {
            return Self::Esm;
        }

        // fall back to script or module source semantics
        if source_type.is_module() {
            Self::Esm
        } else {
            Self::CommonJs
        }
    }

    /// Detect a module format from one file extension.
    pub fn from_extension(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?;
        match extension {
            "mjs" | "mts" | "ds" => Some(Self::Esm),
            "cjs" | "cts" => Some(Self::CommonJs),
            _ => None,
        }
    }

    /// Detect a module format from one package type value.
    pub fn from_package_type(package_type: Option<&str>) -> Option<Self> {
        match package_type {
            Some("module") => Some(Self::Esm),
            Some("commonjs") => Some(Self::CommonJs),
            _ => None,
        }
    }

    /// Detect a module format from one tsconfig module target.
    pub fn from_tsconfig_target(target: ModuleTarget) -> Option<Self> {
        match target {
            ModuleTarget::CommonJs => Some(Self::CommonJs),
            ModuleTarget::Es2015
            | ModuleTarget::Es2020
            | ModuleTarget::Es2022
            | ModuleTarget::EsNext
            | ModuleTarget::Preserve => Some(Self::Esm),
            ModuleTarget::Amd
            | ModuleTarget::Umd
            | ModuleTarget::System
            | ModuleTarget::Node16
            | ModuleTarget::NodeNext
            | ModuleTarget::None => None,
        }
    }

    /// Return true when this module format is CommonJS.
    pub fn is_commonjs(self) -> bool {
        matches!(self, Self::CommonJs)
    }
}

/// A Module is a single source unit.
/// Destack treats all modules as "strict mode".
#[derive(Debug)]
pub struct Module {
    /// The id of the Module itself.
    pub id: ModuleId,
    /// The underlying source File (might be empty if placeholder or synthetic module).
    pub file_id: FileId,
    /// The URI of the Module.
    pub uri: Uri,
    /// The path to the Module.
    pub path: Option<PathBuf>,
    /// The package of the Module (every module belongs to a package).
    pub package_id: PackageId,
    /// The language type of the Module (Destack, TypeScript, JavaScript, etc.).
    pub language_type: LanguageType,
    /// How the module content is loaded/interpreted.
    pub loader: Loader,
    /// The source/origin of the module (user code or builtin).
    pub source: ModuleSource,
}

impl Module {
    /// Create a new blank Module identity with the given loader.
    pub fn blank(
        id: ModuleId,
        file_id: FileId,
        uri: Uri,
        path: Option<PathBuf>,
        package_id: PackageId,
        language_type: LanguageType,
        loader: Loader,
        source: ModuleSource,
    ) -> Self {
        Self {
            id,
            file_id,
            uri,
            path,
            package_id,
            language_type,
            loader,
            source,
        }
    }

    /// Whether this module is user/project code.
    #[inline]
    pub fn is_user(&self) -> bool {
        matches!(self.source, ModuleSource::User)
    }

    /// Whether this module is a builtin library.
    #[inline]
    pub fn is_builtin(&self) -> bool {
        matches!(self.source, ModuleSource::Builtin(_))
    }

    /// Whether this module is a code module.
    #[inline]
    pub fn is_code(&self) -> bool {
        self.loader.is_code()
    }
}

/// One module entry in the registry.
#[derive(Debug)]
struct ModuleRegistryEntry {
    /// The immutable module identity and origin data.
    module: Arc<Module>,
    /// The current module version.
    module_version: ModuleVersion,
    /// The current source file version.
    source_version: FileVersion,
    /// The current tsconfig id.
    tsconfig_id: Option<TsConfigId>,
    /// The current source type.
    source_type: SourceType,
    /// The current runtime module format.
    module_format: ModuleFormat,
}

/// Registry of Modules. THREAD-SAFE.
#[derive(Debug)]
pub struct ModuleRegistry {
    /// The module entries by id.
    entries_by_id: DashMap<ModuleId, ModuleRegistryEntry>,
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
            entries_by_id: DashMap::new(),
            modules_by_uri: DashMap::new(),
            modules_by_path: DashMap::new(),
            modules_by_file_id: DashMap::new(),
        }
    }

    /// Insert a module into the registry.
    #[allow(clippy::too_many_arguments)]
    pub fn insert(
        &self,
        module: Module,
        module_version: ModuleVersion,
        source_version: FileVersion,
        tsconfig_id: Option<TsConfigId>,
        source_type: SourceType,
        module_format: ModuleFormat,
    ) {
        let uri = module.uri.clone();
        let path = module.path.clone();
        let file_id = module.file_id;
        let id = module.id;
        self.entries_by_id.insert(
            id,
            ModuleRegistryEntry {
                module: Arc::new(module),
                module_version,
                source_version,
                tsconfig_id,
                source_type,
                module_format,
            },
        );
        self.modules_by_uri.insert(uri, id);
        self.modules_by_file_id.insert(file_id, id);
        if let Some(path) = path {
            self.modules_by_path.insert(path, id);
        }
    }

    /// Check if a module exists by id.
    pub fn contains(&self, id: ModuleId) -> bool {
        self.entries_by_id.contains_key(&id)
    }

    /// Get a module by module id.
    ///
    /// # Panics
    /// Panics if the module is not found.
    #[inline]
    pub fn get(&self, id: ModuleId) -> Arc<Module> {
        self.entries_by_id
            .get(&id)
            .unwrap_or_else(|| panic!("module not found for id: {id:?}"))
            .module
            .clone()
    }

    /// Get a module id by its URI.
    pub fn get_id_by_uri(&self, uri: &Uri) -> Option<ModuleId> {
        self.modules_by_uri.get(uri).map(|r| *r.value())
    }

    /// Get a module by its URI.
    pub fn get_by_uri(&self, uri: &Uri) -> Option<Arc<Module>> {
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
    pub fn get_by_path(&self, path: &Path) -> Option<Arc<Module>> {
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
    pub fn get_by_file_id(&self, file_id: FileId) -> Option<Arc<Module>> {
        let id = self.get_id_by_file_id(file_id)?;
        Some(self.get(id))
    }

    /// Iterate over the modules in the registry.
    pub fn iter(&self) -> impl Iterator<Item = Arc<Module>> {
        let snapshot: Vec<_> = self
            .entries_by_id
            .iter()
            .map(|entry| entry.module.clone())
            .collect();
        snapshot.into_iter()
    }

    /// Get the number of modules in the registry.
    pub fn len(&self) -> usize {
        self.entries_by_id.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.entries_by_id.is_empty()
    }

    /// Return the current module version.
    pub fn version(&self, id: ModuleId) -> ModuleVersion {
        self.entries_by_id
            .get(&id)
            .unwrap_or_else(|| panic!("module version not found for id: {id:?}"))
            .module_version
    }

    /// Return the current source file version.
    pub fn source_version(&self, id: ModuleId) -> FileVersion {
        self.entries_by_id
            .get(&id)
            .unwrap_or_else(|| panic!("module source version not found for id: {id:?}"))
            .source_version
    }

    /// Return the current tsconfig id.
    pub fn tsconfig_id(&self, id: ModuleId) -> Option<TsConfigId> {
        self.entries_by_id
            .get(&id)
            .unwrap_or_else(|| panic!("module tsconfig id not found for id: {id:?}"))
            .tsconfig_id
    }

    /// Return the current source type.
    pub fn source_type(&self, id: ModuleId) -> SourceType {
        self.entries_by_id
            .get(&id)
            .unwrap_or_else(|| panic!("module source type not found for id: {id:?}"))
            .source_type
    }

    /// Return the current runtime module format.
    pub fn module_format(&self, id: ModuleId) -> ModuleFormat {
        self.entries_by_id
            .get(&id)
            .unwrap_or_else(|| panic!("module format not found for id: {id:?}"))
            .module_format
    }

    /// Set the current source version.
    pub fn set_source_version(&self, id: ModuleId, source_version: FileVersion) {
        self.entries_by_id
            .get_mut(&id)
            .unwrap_or_else(|| panic!("module source version not found for id: {id:?}"))
            .source_version = source_version;
    }

    /// Set the current tsconfig id.
    pub fn set_tsconfig_id(&self, id: ModuleId, tsconfig_id: Option<TsConfigId>) {
        self.entries_by_id
            .get_mut(&id)
            .unwrap_or_else(|| panic!("module tsconfig id not found for id: {id:?}"))
            .tsconfig_id = tsconfig_id;
    }

    /// Set the current source type and runtime module format.
    pub fn set_semantics(
        &self,
        id: ModuleId,
        source_type: SourceType,
        module_format: ModuleFormat,
    ) {
        let mut entry = self
            .entries_by_id
            .get_mut(&id)
            .unwrap_or_else(|| panic!("module semantics not found for id: {id:?}"));
        entry.source_type = source_type;
        entry.module_format = module_format;
    }

    /// Increment and return the module version.
    pub fn bump_version(&self, id: ModuleId) -> ModuleVersion {
        let mut entry = self
            .entries_by_id
            .get_mut(&id)
            .unwrap_or_else(|| panic!("module version not found for id: {id:?}"));
        let next_version = entry.module_version.next();
        entry.module_version = next_version;
        next_version
    }
}
