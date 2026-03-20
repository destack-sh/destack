use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use dashmap::DashMap;
use destack_builtin::{
    BuiltinLibraryKind, BuiltinLibrarySource, BuiltinOutputFormat, BuiltinPlatform, BuiltinRuntime,
    INTRINSIC_SOURCES, LanguageSymbol, PRELUDE_BUILTIN_SOURCE, builtin_library,
    resolve_profile_builtin_library_name,
};
use destack_source::{
    File, FileRegistry, FileType, LanguageType, ModuleId, ModuleVersion, PackageId, PackageVersion,
    Uri,
};
use indexmap::IndexMap;
use parking_lot::Mutex;

use crate::{
    Loader, Module, ModuleFormat, ModuleRegistry, ModuleSource, OutputFormat, Package, PackageKind,
    PackageRegistry, Platform, ProfileKey, Runtime, SourceType, TargetArch, TargetEnv,
    TargetVendor,
};

/// Resolve a file type for a builtin source name.
fn file_type_for_builtin_name(name: &str) -> FileType {
    FileType::from_path(Path::new(name)).unwrap_or(FileType::Destack)
}

/// Well-known package ID for builtins.
pub const BUILTIN_PACKAGE_ID: PackageId = PackageId(1);

/// Well-known package name for builtins.
pub const BUILTIN_PACKAGE_NAME: &str = "@destack/builtin";

/// Key for builtin library and symbol data.
/// This intentionally excludes profile diagnostic flags so runtime policy variants
/// can reuse the same builtin library graph and symbols, since builtins do not depend on those.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BuiltinLibraryKey {
    /// Output format for builtin selection.
    output: OutputFormat,
    /// Runtime for builtin selection.
    runtime: Runtime,
    /// Platform for builtin selection.
    platform: Platform,
    /// Target architecture for builtin selection.
    target_arch: Option<TargetArch>,
    /// Target vendor for builtin selection.
    target_vendor: Option<TargetVendor>,
    /// Target environment for builtin selection.
    target_env: Option<TargetEnv>,
    /// Normalized library set.
    lib: Vec<String>,
}

impl BuiltinLibraryKey {
    /// Build a builtin key from a full profile key.
    fn from_profile_key(profile_key: &ProfileKey) -> Self {
        Self {
            output: profile_key.output,
            runtime: profile_key.runtime,
            platform: profile_key.platform,
            target_arch: profile_key.target_arch.clone(),
            target_vendor: profile_key.target_vendor.clone(),
            target_env: profile_key.target_env.clone(),
            lib: profile_key.lib.clone(),
        }
    }
}

/// Input-side builtin library selection for one profile key.
#[derive(Debug, Clone, Default)]
pub struct BuiltinLibrarySelection {
    /// Builtin libraries in dependency order.
    pub ordered_libraries: Vec<String>,
    /// Module batches in library dependency order.
    pub modules_to_resolve: Vec<Vec<ModuleId>>,
    /// All selected library modules in load order without duplicates.
    pub library_modules: Vec<ModuleId>,
    /// Ambient library modules in load order without duplicates.
    pub ambient_modules: Vec<ModuleId>,
}

/// Language and library builtins.
#[derive(Debug)]
pub struct Builtins {
    /// The builtin package ID.
    pub package_id: PackageId,
    /// Intrinsic modules by path.
    pub intrinsic_module_by_path: IndexMap<String, ModuleId>,
    /// Intrinsic module for each language item.
    pub intrinsic_module_by_item: IndexMap<LanguageSymbol, ModuleId>,
    /// The prelude module ID (re-exports items available without imports).
    pub prelude_module_id: ModuleId,

    /// Library modules cache ((profile, "dom") -> modules).
    library_module_by_name: DashMap<(BuiltinLibraryKey, String), Vec<ModuleId>>,
    /// Selected builtin library modules for one profile key.
    library_selection_by_key: DashMap<BuiltinLibraryKey, BuiltinLibrarySelection>,
    /// Input-side lock for builtin library registration.
    library_load_lock: Mutex<()>,
    /// Library name for each registered library module.
    pub library_name_by_module: DashMap<ModuleId, &'static str>,
}

impl Builtins {
    /// Create builtins by registering intrinsic modules from embedded sources.
    pub fn embedded(
        files: Arc<FileRegistry>,
        modules: Arc<ModuleRegistry>,
        packages: Arc<PackageRegistry>,
    ) -> Self {
        // create builtin package
        let package = Package {
            id: BUILTIN_PACKAGE_ID,
            package_version: PackageVersion::INITIAL,
            kind: PackageKind::Builtin,
            uri: Uri::from_string("builtin://"),
            path: None,
            name: Some(BUILTIN_PACKAGE_NAME.to_string()),
            version: None,
            manifest: None,
            config: None,
            tsconfig: None,
            targets: Default::default(),
        };
        packages.insert(package);

        // register intrinsic modules from embedded sources
        let mut intrinsic_modules = IndexMap::with_capacity(INTRINSIC_SOURCES.len());
        for source in INTRINSIC_SOURCES {
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
            let module_path = source.module_path();
            let module_id = ModuleId::from_relative_path(
                BUILTIN_PACKAGE_ID,
                std::path::Path::new(&module_path),
            );

            // create blank module
            let loader = Loader::from_file_type(file_type);
            let module = Module::blank(
                module_id,
                file_id,
                uri,
                None,
                BUILTIN_PACKAGE_ID,
                language_type,
                loader,
                ModuleSource::Builtin(BuiltinLibraryKind::Intrinsic),
            );
            modules.insert(
                module,
                ModuleVersion::INITIAL,
                file_version,
                None,
                SourceType::Module,
                ModuleFormat::Esm,
            );
            intrinsic_modules.insert(module_path, module_id);
        }

        // build language item -> module mapping, validating all items have modules
        let mut language_symbol_modules = IndexMap::with_capacity(LanguageSymbol::all().count());
        for item in LanguageSymbol::all() {
            let module_path = format!("{}.ds", item.module());
            let module_id = intrinsic_modules.get(&module_path).unwrap_or_else(|| {
                panic!(
                    "language item {item:?} references module '{module_path}' which is not in INTRINSIC_SOURCES"
                )
            });
            language_symbol_modules.insert(item, *module_id);
        }

        // look up prelude module ID (must be registered)
        let prelude_path = PRELUDE_BUILTIN_SOURCE.module_path();
        let prelude_module_id = *intrinsic_modules.get(&prelude_path).unwrap_or_else(|| {
            panic!("prelude module '{prelude_path}' is not in INTRINSIC_SOURCES")
        });

        Self {
            package_id: BUILTIN_PACKAGE_ID,
            intrinsic_module_by_path: intrinsic_modules,
            intrinsic_module_by_item: language_symbol_modules,
            prelude_module_id,
            library_module_by_name: DashMap::new(),
            library_selection_by_key: DashMap::new(),
            library_load_lock: Mutex::new(()),
            library_name_by_module: DashMap::new(),
        }
    }

    /// Get the ModuleId for a language item's defining module.
    pub fn module_for_item(&self, item: LanguageSymbol) -> ModuleId {
        *self
            .intrinsic_module_by_item
            .get(&item)
            .unwrap_or_else(|| panic!("language item {item:?} not registered"))
    }

    /// Build a builtin library key from a profile key.
    fn library_key(profile_key: &ProfileKey) -> BuiltinLibraryKey {
        BuiltinLibraryKey::from_profile_key(profile_key)
    }

    /// Return the cached builtin library selection for one profile key.
    pub fn library_selection(&self, profile_key: &ProfileKey) -> Option<BuiltinLibrarySelection> {
        let lib_cache_key = Self::library_key(profile_key);
        self.library_selection_by_key
            .get(&lib_cache_key)
            .map(|selection| selection.clone())
    }

    /// Return whether one builtin library has sources for one profile key.
    pub fn has_library_for_profile(&self, name: &str, profile_key: &ProfileKey) -> bool {
        let Some(lib) = builtin_library(name) else {
            return false;
        };

        let runtime = BuiltinRuntime::from(profile_key.runtime);
        let output = BuiltinOutputFormat::from(profile_key.output);
        let platform = BuiltinPlatform::from(profile_key.platform);

        lib.sources
            .iter()
            .any(|source| source.matches_target(runtime, output, platform))
    }

    /// Cache the builtin library selection for one profile key.
    pub fn set_library_selection(
        &self,
        profile_key: &ProfileKey,
        selection: BuiltinLibrarySelection,
    ) {
        let lib_cache_key = Self::library_key(profile_key);
        self.library_selection_by_key
            .insert(lib_cache_key, selection);
    }

    /// Load a library module set (e.g., "dom", "es2024").
    /// Returns None if the library name is not registered.
    pub fn load_library(
        &self,
        name: &str,
        files: Arc<FileRegistry>,
        modules: Arc<ModuleRegistry>,
        profile_key: &ProfileKey,
    ) -> Option<Vec<ModuleId>> {
        let _guard = self.library_load_lock.lock();
        let name = self.profile_builtin_library_name(name, profile_key);

        // create a local cycle guard
        let mut loading = HashSet::new();

        // load the library with cycle tracking
        self.load_library_inner(&name, files, modules, profile_key, &mut loading)
    }

    /// Load a library module set with cycle tracking.
    fn load_library_inner(
        &self,
        name: &str,
        files: Arc<FileRegistry>,
        modules: Arc<ModuleRegistry>,
        profile_key: &ProfileKey,
        loading: &mut HashSet<String>,
    ) -> Option<Vec<ModuleId>> {
        // derive the cache key shared across policy-only profile variants
        let lib_cache_key = Self::library_key(profile_key);

        // return cached modules when available
        if let Some(cached) = self.cached_library_modules(name, &lib_cache_key) {
            return Some(cached);
        }

        // skip recursive cycles in the current load chain
        if loading.contains(name) {
            return Some(Vec::new());
        }

        // load the library metadata
        let lib = builtin_library(name)?;

        // check if the library has any sources for this target
        let runtime = BuiltinRuntime::from(profile_key.runtime);
        let output = BuiltinOutputFormat::from(profile_key.output);
        let platform = BuiltinPlatform::from(profile_key.platform);
        let filtered_sources = lib
            .sources
            .iter()
            .copied()
            .filter(|source| source.matches_target(runtime, output, platform))
            .collect::<Vec<_>>();
        if filtered_sources.is_empty() {
            return None;
        }

        // track this library for the current load chain
        loading.insert(name.to_string());

        // recursively load dependencies
        let mut dependencies = Vec::new();
        let mut seen_dependencies = HashSet::new();

        // seed dependencies from library metadata
        for &dependency in lib.dependencies {
            let dependency = self.profile_builtin_library_name(dependency, profile_key);
            if seen_dependencies.insert(dependency.clone()) {
                dependencies.push(dependency);
            }
        }

        // extend dependencies with reference lib directives
        for &reference in lib.reference_libs {
            let reference = self.profile_builtin_library_name(reference, profile_key);
            if seen_dependencies.insert(reference.clone()) {
                dependencies.push(reference);
            }
        }

        // load each dependency in order
        for dependency in dependencies {
            // skip self references
            if dependency == lib.name {
                continue;
            }

            // load each dependency or exit early
            if self
                .load_library_inner(
                    &dependency,
                    files.clone(),
                    modules.clone(),
                    profile_key,
                    loading,
                )
                .is_none()
            {
                loading.remove(name);
                return None;
            }
        }

        // register library sources
        let mut module_ids = Vec::with_capacity(filtered_sources.len());
        for source in filtered_sources {
            let module_id =
                self.register_lib_source(&source, lib.kind, files.clone(), modules.clone());
            module_ids.push(module_id);
        }

        // register module to library name mappings
        for module_id in &module_ids {
            self.library_name_by_module.insert(*module_id, lib.name);
        }

        // cache module ids
        self.library_module_by_name.insert(
            (lib_cache_key.clone(), name.to_string()),
            module_ids.clone(),
        );

        loading.remove(name);

        Some(module_ids)
    }

    /// Resolve one builtin library name against the active profile library set.
    fn profile_builtin_library_name(&self, name: &str, profile_key: &ProfileKey) -> String {
        resolve_profile_builtin_library_name(name, &profile_key.lib)
            .unwrap_or_else(|| name.to_string())
    }

    /// Clone cached module ids and refresh library name mappings.
    fn cached_library_modules(
        &self,
        name: &str,
        lib_cache_key: &BuiltinLibraryKey,
    ) -> Option<Vec<ModuleId>> {
        // read cached module ids
        let cached = self
            .library_module_by_name
            .get(&(lib_cache_key.clone(), name.to_string()))?;

        // refresh module to library name mappings when possible
        if let Some(library) = builtin_library(name) {
            for module_id in cached.iter() {
                self.library_name_by_module.insert(*module_id, library.name);
            }
        }

        Some(cached.clone())
    }

    /// Get the library name for a module, if any.
    pub fn library_name_for_module(&self, module_id: ModuleId) -> Option<&'static str> {
        self.library_name_by_module
            .get(&module_id)
            .map(|name| *name)
    }

    /// Get builtin modules that define intrinsic bindings.
    pub fn intrinsic_module_ids(&self) -> Vec<ModuleId> {
        // filter intrinsic modules to primitive paths
        let mut module_ids = Vec::new();
        let mut seen = HashSet::new();
        for (path, module_id) in self.intrinsic_module_by_path.iter() {
            if !path.starts_with("primitive/") {
                continue;
            }
            if seen.insert(*module_id) {
                module_ids.push(*module_id);
            }
        }
        module_ids
    }

    /// Register a library source as a module.
    fn register_lib_source(
        &self,
        source: &BuiltinLibrarySource,
        kind: BuiltinLibraryKind,
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
        let loader = Loader::from_file_type(file_type);
        let module = Module::blank(
            module_id,
            file_id,
            uri,
            None,
            BUILTIN_PACKAGE_ID,
            language_type,
            loader,
            ModuleSource::Builtin(kind),
        );
        modules.insert(
            module,
            ModuleVersion::INITIAL,
            file_version,
            None,
            SourceType::Module,
            ModuleFormat::Esm,
        );

        module_id
    }
}
