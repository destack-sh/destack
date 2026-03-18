use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use dashmap::DashMap;
use destack_builtin::{
    BuiltinLibKind, BuiltinLibSource, BuiltinOutputFormat, BuiltinPlatform, BuiltinRuntime,
    CORE_SOURCES, LanguageSymbol, PRELUDE_SOURCE, builtin_lib,
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

/// Key for builtin lib and symbol data.
/// This intentionally excludes profile diagnostic flags so runtime policy variants
///  can reuse the same builtin lib graph and symbols, since builtins do not depend on those.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BuiltinLibKey {
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
    /// Normalized lib set.
    lib: Vec<String>,
}

impl BuiltinLibKey {
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

/// Input-side builtin lib selection for one profile key.
#[derive(Debug, Clone, Default)]
pub struct BuiltinLibSelection {
    /// Builtin libs in dependency order.
    pub ordered_libs: Vec<String>,
    /// Module batches in lib dependency order.
    pub modules_to_resolve: Vec<Vec<ModuleId>>,
    /// All selected lib modules in load order without duplicates.
    pub lib_modules: Vec<ModuleId>,
    /// Ambient lib modules in load order without duplicates.
    pub ambient_modules: Vec<ModuleId>,
}

/// Language and library builtins.
#[derive(Debug)]
pub struct Builtins {
    /// The builtin package ID.
    pub package_id: PackageId,
    /// Core modules by path (e.g., "operator/arithmetic.ds" -> ModuleId).
    pub core_module_by_path: IndexMap<String, ModuleId>,
    /// Core module for each language item (LanguageSymbol -> ModuleId).
    pub core_module_by_item: IndexMap<LanguageSymbol, ModuleId>,
    /// The prelude module ID (re-exports items available without imports).
    pub prelude_module_id: ModuleId,

    /// Lib modules cache ((profile, "dom") -> modules).
    lib_module_by_name: DashMap<(BuiltinLibKey, String), Vec<ModuleId>>,
    /// Selected builtin lib modules for one profile key.
    lib_selection_by_key: DashMap<BuiltinLibKey, BuiltinLibSelection>,
    /// Input-side lock for builtin lib registration.
    lib_load_lock: Mutex<()>,
    /// Lib name for each registered lib module.
    pub lib_name_by_module: DashMap<ModuleId, &'static str>,
}

impl Builtins {
    /// Create builtins by registering core modules from embedded sources.
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
            let loader = Loader::from_file_type(file_type);
            let module = Module::blank(
                module_id,
                file_id,
                uri,
                None,
                BUILTIN_PACKAGE_ID,
                language_type,
                loader,
                ModuleSource::Builtin(BuiltinLibKind::Core),
            );
            modules.insert(
                module,
                ModuleVersion::INITIAL,
                file_version,
                None,
                SourceType::Module,
                ModuleFormat::Esm,
            );
            core_modules.insert(module_path, module_id);
        }

        // build language item -> module mapping, validating all items have modules
        let mut language_symbol_modules = IndexMap::with_capacity(LanguageSymbol::all().count());
        for item in LanguageSymbol::all() {
            let module_path = format!("{}.ds", item.module());
            let module_id = core_modules.get(&module_path).unwrap_or_else(|| {
                panic!(
                    "language item {item:?} references module '{module_path}' which is not in CORE_SOURCES"
                )
            });
            language_symbol_modules.insert(item, *module_id);
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
            core_module_by_item: language_symbol_modules,
            prelude_module_id,
            lib_module_by_name: DashMap::new(),
            lib_selection_by_key: DashMap::new(),
            lib_load_lock: Mutex::new(()),
            lib_name_by_module: DashMap::new(),
        }
    }

    /// Get the ModuleId for a language item's defining module.
    pub fn module_for_item(&self, item: LanguageSymbol) -> ModuleId {
        *self
            .core_module_by_item
            .get(&item)
            .unwrap_or_else(|| panic!("language item {item:?} not registered"))
    }

    /// Build a builtin lib key from a profile key.
    fn lib_key(profile_key: &ProfileKey) -> BuiltinLibKey {
        BuiltinLibKey::from_profile_key(profile_key)
    }

    /// Return the cached builtin lib selection for one profile key.
    pub fn lib_selection(&self, profile_key: &ProfileKey) -> Option<BuiltinLibSelection> {
        let lib_cache_key = Self::lib_key(profile_key);
        self.lib_selection_by_key
            .get(&lib_cache_key)
            .map(|selection| selection.clone())
    }

    /// Return whether one builtin lib has sources for one profile key.
    pub fn has_lib_for_profile(&self, name: &str, profile_key: &ProfileKey) -> bool {
        let Some(lib) = builtin_lib(name) else {
            return false;
        };

        let runtime = BuiltinRuntime::from(profile_key.runtime);
        let output = BuiltinOutputFormat::from(profile_key.output);
        let platform = BuiltinPlatform::from(profile_key.platform);

        lib.sources
            .iter()
            .any(|source| source.matches_target(runtime, output, platform))
    }

    /// Cache the builtin lib selection for one profile key.
    pub fn set_lib_selection(&self, profile_key: &ProfileKey, selection: BuiltinLibSelection) {
        let lib_cache_key = Self::lib_key(profile_key);
        self.lib_selection_by_key.insert(lib_cache_key, selection);
    }

    /// Load a lib module set (e.g., "dom", "es2024").
    /// Returns None if the lib name is not registered.
    pub fn load_lib(
        &self,
        name: &str,
        files: Arc<FileRegistry>,
        modules: Arc<ModuleRegistry>,
        profile_key: &ProfileKey,
    ) -> Option<Vec<ModuleId>> {
        let _guard = self.lib_load_lock.lock();

        // create a local cycle guard
        let mut loading = HashSet::new();

        // load the lib with cycle tracking
        self.load_lib_inner(name, files, modules, profile_key, &mut loading)
    }

    /// Load a lib module set with cycle tracking.
    fn load_lib_inner(
        &self,
        name: &str,
        files: Arc<FileRegistry>,
        modules: Arc<ModuleRegistry>,
        profile_key: &ProfileKey,
        loading: &mut HashSet<String>,
    ) -> Option<Vec<ModuleId>> {
        // derive the cache key shared across policy-only profile variants
        let lib_cache_key = Self::lib_key(profile_key);

        // return cached modules when available
        if let Some(cached) = self.cached_lib_modules(name, &lib_cache_key) {
            return Some(cached);
        }

        // skip recursive cycles in the current load chain
        if loading.contains(name) {
            return Some(Vec::new());
        }

        // load the lib metadata
        let lib = builtin_lib(name)?;

        // check if the lib has any sources for this target
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

        // track this lib for the current load chain
        loading.insert(name.to_string());

        // recursively load dependencies
        let mut dependencies = Vec::new();
        let mut seen_dependencies = HashSet::new();

        // seed dependencies from lib metadata
        for &dependency in lib.dependencies {
            if seen_dependencies.insert(dependency.to_string()) {
                dependencies.push(dependency.to_string());
            }
        }

        // extend dependencies with reference lib directives
        for reference in lib.reference_libs() {
            if seen_dependencies.insert(reference.to_string()) {
                dependencies.push(reference.to_string());
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
                .load_lib_inner(
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

        // register lib sources
        let mut module_ids = Vec::with_capacity(filtered_sources.len());
        for source in filtered_sources {
            let module_id =
                self.register_lib_source(&source, lib.kind, files.clone(), modules.clone());
            module_ids.push(module_id);
        }

        // register module to lib name mappings
        for module_id in &module_ids {
            self.lib_name_by_module.insert(*module_id, lib.name);
        }

        // cache module ids
        self.lib_module_by_name.insert(
            (lib_cache_key.clone(), name.to_string()),
            module_ids.clone(),
        );

        loading.remove(name);

        Some(module_ids)
    }

    /// Clone cached module ids and refresh lib name mappings.
    fn cached_lib_modules(
        &self,
        name: &str,
        lib_cache_key: &BuiltinLibKey,
    ) -> Option<Vec<ModuleId>> {
        // read cached module ids
        let cached = self
            .lib_module_by_name
            .get(&(lib_cache_key.clone(), name.to_string()))?;

        // refresh module to lib name mappings when possible
        if let Some(lib) = builtin_lib(name) {
            for module_id in cached.iter() {
                self.lib_name_by_module.insert(*module_id, lib.name);
            }
        }

        Some(cached.clone())
    }
    /// Get the lib name for a module, if any.
    pub fn lib_name_for_module(&self, module_id: ModuleId) -> Option<&'static str> {
        self.lib_name_by_module.get(&module_id).map(|name| *name)
    }

    /// Get builtin modules that define intrinsic bindings.
    pub fn intrinsic_module_ids(&self) -> Vec<ModuleId> {
        // filter core modules to intrinsic paths
        let mut module_ids = Vec::new();
        let mut seen = HashSet::new();
        for (path, module_id) in self.core_module_by_path.iter() {
            if !path.starts_with("intrinsic/") {
                // high-tech filter
                continue;
            }
            if seen.insert(*module_id) {
                module_ids.push(*module_id);
            }
        }
        module_ids
    }

    /// Register a lib source as a module.
    fn register_lib_source(
        &self,
        source: &BuiltinLibSource,
        kind: BuiltinLibKind,
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
