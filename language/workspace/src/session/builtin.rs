use std::path::Path;
use std::sync::Arc;

use dashmap::DashMap;
use destack_base::StringId;
use destack_builtin::{BuiltinLibSource, CORE_SOURCES, LanguageItem, PRELUDE_SOURCE, builtin_lib};
use destack_dir::GlobalSymbolId;
use destack_source::{File, FileRegistry, FileType, LanguageType, ModuleId, PackageId, Uri};
use indexmap::IndexMap;

use crate::{Module, ModuleRegistry, ModuleType, Package, PackageKind, PackageRegistry, ProfileId};

/// Resolve a file type for a builtin source name.
fn file_type_for_builtin_name(name: &str) -> FileType {
    FileType::from_path(Path::new(name)).unwrap_or(FileType::Destack)
}

/// Well-known package ID for builtins.
pub const BUILTIN_PACKAGE_ID: PackageId = PackageId(1);

/// Well-known package name for builtins.
pub const BUILTIN_PACKAGE_NAME: &str = "@destack-sh/builtin";

/// Language builtins.
#[derive(Debug)]
pub struct LanguageBuiltins {
    /// The builtin package ID.
    pub package_id: PackageId,

    /// Core modules by path (e.g., "operator/arithmetic.ds" -> ModuleId).
    pub core_module_by_path: IndexMap<String, ModuleId>,

    /// Core module for each language item (LanguageItem -> ModuleId).
    pub core_module_by_item: IndexMap<LanguageItem, ModuleId>,

    /// The prelude module ID (re-exports items available without imports).
    pub prelude_module_id: ModuleId,

    /// Lib modules cache ("dom" -> modules, "es2024" -> modules).
    pub lib_module_by_name: DashMap<String, Vec<ModuleId>>,

    /// Ambient lib modules per profile.
    pub ambient_libs_by_profile: DashMap<ProfileId, Vec<ModuleId>>,

    /// Canonical lib symbols per profile.
    pub lib_symbols_by_profile: DashMap<ProfileId, IndexMap<StringId, GlobalSymbolId>>,

    /// Resolved language items cache (LanguageItem -> GlobalSymbolId).
    /// (Populated lazily when items are first resolved after module compilation.)
    pub items: DashMap<LanguageItem, GlobalSymbolId>,
}

impl LanguageBuiltins {
    /// Create builtins by registering core modules from embedded sources.
    pub fn embedded(
        files: Arc<FileRegistry>,
        modules: Arc<ModuleRegistry>,
        packages: Arc<PackageRegistry>,
    ) -> Self {
        // create builtin package
        let package = Package {
            id: BUILTIN_PACKAGE_ID,
            kind: PackageKind::Builtin,
            uri: Uri::from_string("builtin://"),
            path: None,
            name: Some(BUILTIN_PACKAGE_NAME.to_string()),
            version: None,
            manifest: None,
            dsconfig: None,
            tsconfig: None,
            targets: Default::default(),
        };
        packages.insert(package);

        // register core modules from embedded sources
        let mut core_modules = IndexMap::with_capacity(CORE_SOURCES.len());
        for source in CORE_SOURCES {
            let uri = Uri::from_string(source.virtual_path());

            // select file/language types for the builtin source
            let file_type = file_type_for_builtin_name(source.name);
            let language_type = LanguageType::from(file_type);

            // create file
            let file_id = files.next_id();
            let file = File::from_text(
                file_id,
                source.name.to_string(),
                uri.clone(),
                None,
                file_type,
                source.content.to_string(),
            );
            let file_version = file.version;
            files.insert(file);

            // create module id from path
            let module_path = if source.path.is_empty() {
                source.name.to_string()
            } else {
                format!("{}/{}", source.path, source.name)
            };
            let module_id = ModuleId::from_relative_path(
                BUILTIN_PACKAGE_ID,
                std::path::Path::new(&module_path),
            );

            // create blank module (will be parsed/bound later)
            let module = Module::blank(
                module_id,
                file_id,
                file_version,
                uri,
                None,
                BUILTIN_PACKAGE_ID,
                None,
                ModuleType::Module,
                language_type,
            );
            modules.insert(module);
            core_modules.insert(module_path, module_id);
        }

        // build language item -> module mapping, validating all items have modules
        let mut language_item_modules = IndexMap::with_capacity(LanguageItem::all().count());
        for item in LanguageItem::all() {
            let module_path = format!("{}.ds", item.module());
            let module_id = core_modules.get(&module_path).unwrap_or_else(|| {
                panic!(
                    "language item {item:?} references module '{module_path}' which is not in CORE_SOURCES"
                )
            });
            language_item_modules.insert(item, *module_id);
        }

        // look up prelude module ID (must be registered)
        let prelude_path = if PRELUDE_SOURCE.path.is_empty() {
            PRELUDE_SOURCE.name.to_string()
        } else {
            format!("{}/{}", PRELUDE_SOURCE.path, PRELUDE_SOURCE.name)
        };
        let prelude_module_id = *core_modules
            .get(&prelude_path)
            .unwrap_or_else(|| panic!("prelude module '{prelude_path}' is not in CORE_SOURCES"));

        Self {
            package_id: BUILTIN_PACKAGE_ID,
            core_module_by_path: core_modules,
            core_module_by_item: language_item_modules,
            prelude_module_id,
            lib_module_by_name: DashMap::new(),
            ambient_libs_by_profile: DashMap::new(),
            lib_symbols_by_profile: DashMap::new(),
            items: DashMap::new(),
        }
    }

    /// Get the ModuleId for a language item's defining module.
    pub fn module_for_item(&self, item: LanguageItem) -> ModuleId {
        *self
            .core_module_by_item
            .get(&item)
            .unwrap_or_else(|| panic!("language item {item:?} not registered"))
    }

    /// Load a lib module set (e.g., "dom", "es2024").
    /// Returns None if the lib name is not registered.
    pub fn load_lib(
        &self,
        name: &str,
        files: Arc<FileRegistry>,
        modules: Arc<ModuleRegistry>,
    ) -> Option<Vec<ModuleId>> {
        // check cache first
        if let Some(cached) = self.lib_module_by_name.get(name) {
            return Some(cached.clone());
        }

        let lib = builtin_lib(name)?;

        // recursively load dependencies first
        for &dependency in lib.dependencies {
            self.load_lib(dependency, files.clone(), modules.clone())?;
        }

        // register lib sources
        let mut module_ids = Vec::with_capacity(lib.sources.len());
        for source in lib.sources {
            let module_id = self.register_lib_source(source, files.clone(), modules.clone());
            module_ids.push(module_id);
        }

        // cache module ids
        self.lib_module_by_name
            .insert(name.to_string(), module_ids.clone());

        Some(module_ids)
    }

    /// Get the ambient lib modules for a profile, if any.
    pub fn ambient_libs(&self, profile_id: ProfileId) -> Option<Vec<ModuleId>> {
        self.ambient_libs_by_profile
            .get(&profile_id)
            .map(|modules| modules.clone())
    }

    /// Get the canonical lib symbol for a profile and name.
    pub fn lib_symbol(&self, profile_id: ProfileId, name: StringId) -> Option<GlobalSymbolId> {
        self.lib_symbols_by_profile
            .get(&profile_id)
            .and_then(|symbols| symbols.get(&name).copied())
    }

    /// Set the ambient lib modules for a profile.
    pub fn set_ambient_libs(&self, profile_id: ProfileId, modules: Vec<ModuleId>) {
        self.ambient_libs_by_profile.insert(profile_id, modules);
    }

    /// Set the canonical lib symbols for a profile.
    pub fn set_lib_symbols(
        &self,
        profile_id: ProfileId,
        symbols: IndexMap<StringId, GlobalSymbolId>,
    ) {
        self.lib_symbols_by_profile.insert(profile_id, symbols);
    }

    fn register_lib_source(
        &self,
        source: &BuiltinLibSource,
        files: Arc<FileRegistry>,
        modules: Arc<ModuleRegistry>,
    ) -> ModuleId {
        // check if module already exists
        let uri = Uri::from_string(source.virtual_path());
        if let Some(module_id) = modules.get_id_by_uri(&uri) {
            return module_id;
        }

        // select file/language types for the builtin source
        let file_type = file_type_for_builtin_name(source.name);
        let language_type = LanguageType::from(file_type);

        // create file
        let file_id = files.next_id();
        let file = File::from_text(
            file_id,
            source.name.to_string(),
            uri.clone(),
            None,
            file_type,
            source.content.to_string(),
        );
        let file_version = file.version;
        files.insert(file);

        // create module id from path
        let module_path = source.module_path();
        let module_id = ModuleId::from_relative_path(BUILTIN_PACKAGE_ID, Path::new(&module_path));

        // create blank module (will be parsed/bound later)
        let module = Module::blank(
            module_id,
            file_id,
            file_version,
            uri,
            None,
            BUILTIN_PACKAGE_ID,
            None,
            ModuleType::Module,
            language_type,
        );
        modules.insert(module);

        module_id
    }
}
