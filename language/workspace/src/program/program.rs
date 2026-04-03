use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::{
    ArtifactKey, ArtifactStore, EnvSnapshot, Loader, Platform, ProfileKey, Runtime,
};
use destack_ast as ast;
use destack_core::StringPool;
use destack_source::{
    DiagnosticCollector, DiagnosticStore, File, FileId, FileRegistry, FileSystem, FileType,
    LanguageType, ModuleId, ModuleVersion, PackageId, PackageVersion, Uri,
};
use indexmap::IndexMap;

use crate::{
    Builtins, CompilerOptions, DestackOptions, DsPathAliases, FormatterOptions, LinterOptions,
    Module, ModuleDetection, ModuleFormat, ModuleRegistry, ModuleSource, Package, PackageKind,
    PackageRegistry, Profile, ProfileConfig, ProfileEnv, ProfileId, ProfileRegistry, SourceType,
    Target, TargetId, TsConfig, TsConfigId, TsConfigOptions, TsConfigRegistry,
    builtin_libs_for_type_entries, discover_typescript_type_entries,
    normalize_typescript_lib_names, normalize_typescript_type_entries,
    profile_flags_for_compiler_options, typescript_default_libs,
};

/// Tsconfig context for one module profile decision.
#[derive(Debug, Clone)]
struct ModuleTsConfigContext {
    /// The normalized tsconfig options.
    options: TsConfigOptions,
    /// The tsconfig directory for resolving relative paths.
    directory: PathBuf,
}

/// A Program in a session.
#[derive(Debug)]
pub struct Program {
    // meta
    /// Default formatter options.
    pub formatter: FormatterOptions,
    /// Default linter options.
    pub linter: LinterOptions,
    /// The current working directory.
    pub cwd: PathBuf,
    /// The file system.
    pub fs: Arc<dyn FileSystem>,
    /// The files in the program.
    pub files: Arc<FileRegistry>,

    // content
    /// The packages.
    pub packages: Arc<PackageRegistry>,
    /// The modules.
    pub modules: Arc<ModuleRegistry>,
    /// The tsconfigs (separate registry as tsconfigs can be nested within packages).
    pub tsconfigs: Arc<TsConfigRegistry>,
    /// The combined string pool.
    pub strings: Arc<StringPool>,
    /// The profiles in this program.
    pub profiles: Arc<ProfileRegistry>,
    /// The diagnostic collector.
    pub diagnostics: DiagnosticCollector,
    /// The diagnostic store.
    pub diagnostic_store: DiagnosticStore,

    // builtins
    /// Language builtins.
    pub builtins: Option<Arc<Builtins>>,
    /// The root module id.
    pub root_module_id: ModuleId,
    /// Fallback file for diagnostics without source anchors.
    pub fallback_file_id: FileId,
}

#[allow(clippy::too_many_arguments)]
impl Program {
    const DIAGNOSTIC_TARGET_NAME: &'static str = "__diagnostic";

    /// Create a new Program with default options.
    pub fn from_fs(cwd: PathBuf, fs: Arc<dyn FileSystem>, files: Arc<FileRegistry>) -> Self {
        Self::from_options(
            FormatterOptions::default(),
            LinterOptions::default(),
            cwd,
            fs,
            files,
        )
    }

    /// Create a new Program.
    pub fn from_options(
        formatter: FormatterOptions,
        linter: LinterOptions,
        cwd: PathBuf,
        fs: Arc<dyn FileSystem>,
        files: Arc<FileRegistry>,
    ) -> Self {
        // set up content registries
        let modules = Arc::new(ModuleRegistry::new());
        let packages = Arc::new(PackageRegistry::new());
        let tsconfigs = Arc::new(TsConfigRegistry::new());
        let profiles = Arc::new(ProfileRegistry::new());
        let strings = Arc::new(StringPool::new());
        let diagnostics = DiagnosticCollector::new();
        let diagnostic_store = DiagnosticStore::new();

        // create the semantic program
        Self {
            formatter,
            linter,
            cwd,
            fs,
            files,

            modules,
            packages,
            tsconfigs,
            profiles,
            strings,
            diagnostics,
            diagnostic_store,
            builtins: None,

            root_module_id: ModuleId::EPHEMERAL,
            fallback_file_id: FileId::new(0),
        }
        .with_root()
    }

    /// Create a new Program with shared registries (for use with Session).
    pub fn new(
        formatter: FormatterOptions,
        linter: LinterOptions,
        cwd: PathBuf,
        fs: Arc<dyn FileSystem>,
        files: Arc<FileRegistry>,
        modules: Arc<ModuleRegistry>,
        packages: Arc<PackageRegistry>,
        tsconfigs: Arc<TsConfigRegistry>,
        strings: Arc<StringPool>,
        builtins: Option<Arc<Builtins>>,
    ) -> Self {
        let diagnostics = DiagnosticCollector::new();
        let profiles = Arc::new(ProfileRegistry::new());
        let diagnostic_store = DiagnosticStore::new();

        Self {
            formatter,
            linter,
            cwd,
            fs,
            files,

            modules,
            packages,
            tsconfigs,
            profiles,
            strings,
            diagnostics,
            diagnostic_store,
            builtins,

            root_module_id: ModuleId::EPHEMERAL,
            fallback_file_id: FileId::new(0),
        }
        .with_root()
    }

    /// Attach one synthetic root package and module to the program.
    fn with_root(mut self) -> Self {
        let (root_module_id, fallback_file_id) =
            Self::make_root(&self.modules, &self.packages, self.files.clone());

        self.root_module_id = root_module_id;
        self.fallback_file_id = fallback_file_id;
        self
    }

    /// Create and insert the root file and module for caching.
    fn make_root(
        modules: &ModuleRegistry,
        packages: &Arc<PackageRegistry>,
        files: Arc<FileRegistry>,
    ) -> (ModuleId, FileId) {
        // ephemeral package for root
        let root_package_id = PackageId::EPHEMERAL;
        let root_uri = Uri::from_string("<root>");
        let root_package = Package {
            id: root_package_id,
            package_version: PackageVersion::INITIAL,
            kind: PackageKind::Ephemeral,
            uri: root_uri.clone(),
            path: None,
            name: Some("<root>".to_string()),
            version: None,
            manifest: None,
            config: None,
            tsconfig: None,
            targets: IndexMap::new(),
        };
        packages.insert(root_package);

        // root file
        let root_file_id = files.next_id();
        let root_file = File::from_text(
            root_file_id,
            "<destack>".to_string(),
            root_uri.clone(),
            None,
            FileType::Destack,
            String::new(),
        );
        files.insert(root_file);

        // root module (uses ephemeral module id)
        let root_module_id = ModuleId::EPHEMERAL;
        let root_file = files.get(root_file_id);
        let root_module = Module::blank(
            root_module_id,
            root_file_id,
            root_uri,
            None,
            root_package_id,
            LanguageType::Destack,
            Loader::Destack,
            ModuleSource::User,
        );

        modules.insert(
            root_module,
            ModuleVersion::INITIAL,
            root_file.version,
            None, // no tsconfig for root module
            SourceType::Script,
            ModuleFormat::Esm,
        );
        (root_module_id, root_file_id)
    }

    /// Drop cached module graphs for a profile.
    pub fn drop_module_graph(&self, artifacts: &ArtifactStore, profile_id: ProfileId) {
        artifacts.invalidate(&ArtifactKey::module_graph(profile_id));
    }

    /// Register a module with inline content (pre-loaded, no filesystem read needed).
    pub fn register_inline_module(&self, uri: Uri, content: String, ty: FileType) -> ModuleId {
        // check if module already exists
        if let Some(module_id) = self.modules.get_id_by_uri(&uri) {
            return module_id;
        }

        // create a loaded file (not blank)
        let file_id = self.files.next_id();
        let name = uri
            .to_path()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "<string>".to_string());
        let file = File::from_text(file_id, name, uri.clone(), None, ty, content);
        let file_version = file.version;
        self.files.insert(file);

        // use ephemeral package for inline content
        let package_id = PackageId::EPHEMERAL;
        let module_id =
            ModuleId::from_relative_path(package_id, std::path::Path::new(uri.as_ref()));
        let source_type = uri
            .to_path()
            .and_then(SourceType::from_extension)
            .unwrap_or(SourceType::Script);
        let language_type = LanguageType::from(ty);
        let module_format = ModuleFormat::detect(None, language_type, source_type, None, None);
        let loader = Loader::from_file_type(ty);
        let module = Module::blank(
            module_id,
            file_id,
            uri,
            None,
            package_id,
            language_type,
            loader,
            ModuleSource::User,
        );
        self.modules.insert(
            module,
            ModuleVersion::INITIAL,
            file_version,
            None,
            source_type,
            module_format,
        );

        module_id
    }

    /// Detect source type for one module file.
    pub fn detect_module_source_type(
        &self,
        path: Option<&Path>,
        package_id: PackageId,
        tsconfig_id: Option<TsConfigId>,
        has_import_export: bool,
    ) -> SourceType {
        // collect detection inputs from package and tsconfig
        let package_type = self.package_module_type_for_id(package_id);
        let module_detection = self.tsconfig_module_detection_for_id(tsconfig_id);

        // detect from path when available
        if let Some(path) = path {
            return SourceType::detect(
                path,
                has_import_export,
                module_detection,
                package_type.as_deref(),
            );
        }

        // force module mode when requested
        if module_detection == ModuleDetection::Force {
            return SourceType::Module;
        }

        // honor package type without a filesystem path
        if let Some(package_type) = package_type.as_deref() {
            if package_type == "module" {
                return SourceType::Module;
            }

            if package_type == "commonjs" {
                return SourceType::Script;
            }
        }

        // fall back to syntax-based auto detection
        if has_import_export {
            SourceType::Module
        } else {
            SourceType::Script
        }
    }

    /// Detect module format for one module file.
    pub fn detect_module_format(
        &self,
        path: Option<&Path>,
        language_type: LanguageType,
        source_type: SourceType,
        package_id: PackageId,
        tsconfig_id: Option<TsConfigId>,
    ) -> ModuleFormat {
        // collect format overrides from package and tsconfig
        let package_type = self.package_module_type_for_id(package_id);
        let tsconfig_format = self.tsconfig_module_format_for_id(tsconfig_id);

        ModuleFormat::detect(
            path,
            language_type,
            source_type,
            package_type.as_deref(),
            tsconfig_format,
        )
    }

    /// Recompute source type and module format for one module.
    pub fn refresh_module_semantics(&self, artifacts: &ArtifactStore, module_id: ModuleId) {
        // capture current module state and detection inputs
        let module = self.modules.get(module_id);
        let (
            path,
            package_id,
            tsconfig_id,
            language_type,
            previous_source_type,
            previous_module_format,
        ) = {
            let module = module.as_ref();
            (
                module.path.clone(),
                module.package_id,
                self.modules.tsconfig_id(module_id),
                module.language_type,
                self.modules.source_type(module_id),
                self.modules.module_format(module_id),
            )
        };
        let has_import_export = self.module_has_import_export_syntax(artifacts, module_id);

        // detect fresh semantics from current workspace state
        let source_type = self.detect_module_source_type(
            path.as_deref(),
            package_id,
            tsconfig_id,
            has_import_export,
        );
        let module_format = self.detect_module_format(
            path.as_deref(),
            language_type,
            source_type,
            package_id,
            tsconfig_id,
        );

        // write updates only when semantics changed
        if source_type == previous_source_type && module_format == previous_module_format {
            return;
        }

        self.modules
            .set_semantics(module_id, source_type, module_format);
    }

    /// Increment a module version and return the updated value.
    pub fn bump_module_version(&self, module_id: ModuleId) -> ModuleVersion {
        self.modules.bump_version(module_id)
    }

    /// Access tsconfig for a module via closure.
    pub fn with_tsconfig<T>(&self, module: &Module, f: impl FnOnce(&TsConfig) -> T) -> Option<T> {
        let tsconfig_id = self.modules.tsconfig_id(module.id)?;
        let tsconfig = self.tsconfigs.get(tsconfig_id);

        Some(f(&tsconfig.read()))
    }

    /// Access tsconfig options for a module via closure.
    pub fn with_tsconfig_options<T>(
        &self,
        module: &Module,
        f: impl FnOnce(&TsConfigOptions) -> T,
    ) -> Option<T> {
        let tsconfig_id = self.modules.tsconfig_id(module.id)?;
        let tsconfig = self.tsconfigs.get(tsconfig_id);
        Some(f(&tsconfig.read().options))
    }

    /// Access package config options for a module via closure.
    pub fn with_config_options<T>(
        &self,
        module: &Module,
        f: impl FnOnce(&DestackOptions) -> T,
    ) -> Option<T> {
        let package = self.packages.get(module.package_id);
        let package_guard = package.read();
        let config = package_guard.config.as_ref()?;
        Some(f(&config.options))
    }

    /// Get effective linter options for a module.
    pub fn get_linter_options(&self, module_id: ModuleId) -> LinterOptions {
        let module = self.modules.get(module_id);
        let module = module.as_ref();

        // try package config first
        if let Some(options) = self.with_config_options(module, |ds| ds.linter.clone()) {
            return options;
        }

        // fall back to program defaults
        self.linter.clone()
    }

    /// Build effective profile compiler options for one module.
    fn profile_compiler_options_for_module(
        &self,
        package: &Package,
        module: &Module,
    ) -> (CompilerOptions, Option<ModuleTsConfigContext>) {
        // start from package compiler options when config exists
        let mut compiler_options = package
            .config
            .as_ref()
            .map(|config| config.options.compiler.clone())
            .unwrap_or_default();

        // only use tsconfig when no package config exists
        let tsconfig_context = if package.config.is_none() {
            self.with_tsconfig(module, |tsconfig| ModuleTsConfigContext {
                options: tsconfig.options.clone(),
                directory: tsconfig.directory.clone(),
            })
        } else {
            None
        };

        // use tsconfig defaults when package config is absent
        if let Some(tsconfig_context) = tsconfig_context.as_ref() {
            Self::apply_tsconfig_profile_overrides(
                &mut compiler_options,
                &tsconfig_context.options,
            );
            self.apply_tsconfig_implicit_type_overrides(
                module,
                &mut compiler_options,
                tsconfig_context,
            );
        }

        (compiler_options, tsconfig_context)
    }

    /// Apply tsconfig settings that affect profile and resolution behavior.
    fn apply_tsconfig_profile_overrides(
        compiler_options: &mut CompilerOptions,
        tsconfig_options: &TsConfigOptions,
    ) {
        let ts_compiler_options = &tsconfig_options.compiler;

        compiler_options.base_url = ts_compiler_options.base_url.clone();
        compiler_options.paths = ts_compiler_options.paths.as_ref().map(|paths| {
            let mut mapped_paths = DsPathAliases::default();
            for (key, values) in paths {
                mapped_paths.insert(key.clone(), values.clone());
            }
            mapped_paths
        });
        compiler_options.module = ts_compiler_options.module;
        compiler_options.es_target = ts_compiler_options.es_target;
        compiler_options.allow_js = ts_compiler_options.allow_js;
        compiler_options.check_js = ts_compiler_options.check_js;
        compiler_options.skip_lib_check = ts_compiler_options.skip_lib_check;

        if !ts_compiler_options.lib.is_empty() {
            compiler_options.lib = normalize_typescript_lib_names(&ts_compiler_options.lib);
        }

        // keep normalized type entries for downstream resolver and analysis
        if !ts_compiler_options.types.is_empty() {
            compiler_options.types = normalize_typescript_type_entries(&ts_compiler_options.types);
        }
    }

    /// Add implicit type entries from tsconfig type root resolution.
    fn apply_tsconfig_implicit_type_overrides(
        &self,
        module: &Module,
        compiler_options: &mut CompilerOptions,
        tsconfig_context: &ModuleTsConfigContext,
    ) {
        // skip when tsconfig types are explicit
        if !tsconfig_context.options.compiler.types.is_empty() {
            return;
        }

        // discover normalized type entries from effective type roots
        let discovered_types = discover_typescript_type_entries(
            self.fs.as_ref(),
            &tsconfig_context.options,
            tsconfig_context.directory.as_path(),
            module.path.as_deref(),
        );
        if discovered_types.is_empty() {
            return;
        }

        // merge discovered types into compiler options
        let mut types = normalize_typescript_type_entries(&compiler_options.types);
        let mut seen = HashSet::new();
        for type_name in &types {
            seen.insert(type_name.clone());
        }

        for discovered_type in discovered_types {
            if seen.insert(discovered_type.clone()) {
                types.push(discovered_type);
            }
        }

        compiler_options.types = types;
    }

    /// Get the profile id for a module with the default profile selection.
    pub fn default_profile_id_for_module(&self, module_id: ModuleId) -> ProfileId {
        // get package for module
        let module = self.modules.get(module_id);
        let module = module.as_ref();
        let package = self.packages.get(module.package_id);
        let package = package.read();

        // derive profile compiler options from package config or tsconfig
        let (compiler_options, tsconfig_context) =
            self.profile_compiler_options_for_module(&package, module);

        // get target and profile config from the package config
        let (target, profile_config) = if let Some(config) = package.config.as_ref() {
            // resolve the configured or fallback target
            let target = config
                .options
                .default_target
                .as_ref()
                .and_then(|name| {
                    let target_id = TargetId::new(package.id, name);
                    package.targets.get(&target_id)
                })
                .or_else(|| package.targets.values().find(|target| !target.synthetic))
                .cloned()
                .unwrap_or_else(|| self.fallback_target_for_module(module));
            let profile_config = compiler_options
                .profile
                .as_ref()
                .and_then(|name| config.options.profiles.get(name));
            (target, profile_config)
        } else {
            // pick a target based on the module language when no package config exists
            (self.fallback_target_for_module(module), None)
        };

        let key = Self::profile_key_for_target(
            &target,
            &compiler_options,
            profile_config,
            tsconfig_context.as_ref().map(|context| &context.options),
        );

        self.profiles.get_or_create(key)
    }

    /// Ensure a target exists for a module and return its id.
    pub fn ensure_target_for_module(&self, module_id: ModuleId) -> TargetId {
        let module = self.modules.get(module_id);
        let module = module.as_ref();
        let package_id = module.package_id;
        let diagnostic_id = TargetId::new(package_id, Self::DIAGNOSTIC_TARGET_NAME);

        let package = self.packages.get(package_id);
        let mut package = package.write();

        if package.targets.contains_key(&diagnostic_id) {
            return diagnostic_id;
        }

        if let Some((existing_id, _)) = package.targets.iter().find(|(_, target)| {
            !target.synthetic && (target.emit.is_native() || target.emit.is_wasm())
        }) {
            return existing_id.clone();
        }

        // resolve base target for diagnostics
        let base_target = if let Some(config) = package.config.as_ref() {
            config
                .options
                .default_target
                .as_ref()
                .and_then(|name| {
                    let target_id = TargetId::new(package_id, name);
                    package
                        .targets
                        .get(&target_id)
                        .filter(|target| !target.synthetic)
                })
                .or_else(|| package.targets.values().find(|target| !target.synthetic))
                .cloned()
                .unwrap_or_else(|| self.fallback_target_for_module(module))
        } else {
            self.fallback_target_for_module(module)
        };

        let synthetic_target = Target::synthetic_for(&base_target, Self::DIAGNOSTIC_TARGET_NAME);
        package
            .targets
            .insert(diagnostic_id.clone(), synthetic_target);

        drop(package);
        let _ = self.packages.bump_version(package_id);

        diagnostic_id
    }

    /// Pick a fallback target based on the module language type.
    /// TODO #Cleanup: should we have fallback profiles at all? or only explicitly (sometimes?)?
    fn fallback_target_for_module(&self, module: &Module) -> Target {
        if module.language_type.is_destack() {
            Target::native("default")
        } else {
            Target::js("default")
        }
    }

    /// Get the profile data for a profile id.
    pub fn profile(&self, profile_id: ProfileId) -> Profile {
        self.profiles
            .get(profile_id)
            .unwrap_or_else(|| panic!("missing profile data for {profile_id:?}"))
    }

    /// Get the profile id for a target in the module's package.
    pub fn profile_id_for_target(
        &self,
        module_id: ModuleId,
        target_id: &TargetId,
    ) -> Option<ProfileId> {
        let module = self.modules.get(module_id);
        let module = module.as_ref();
        let package = self.packages.get(module.package_id);
        let package = package.read();
        let target = package
            .targets
            .get(target_id)
            .cloned()
            .or_else(|| Target::implicit_for_name(&target_id.name))?;

        let (compiler_options, tsconfig_context) =
            self.profile_compiler_options_for_module(&package, module);
        let profile_config = package.config.as_ref().and_then(|config| {
            target
                .profile
                .as_ref()
                .or(compiler_options.profile.as_ref())
                .and_then(|name| config.options.profiles.get(name))
        });

        let key = Self::profile_key_for_target(
            &target,
            &compiler_options,
            profile_config,
            tsconfig_context.as_ref().map(|context| &context.options),
        );

        Some(self.profiles.get_or_create(key))
    }

    /// Get the profile id for a target, falling back to the module default profile.
    pub fn profile_id_for_target_or_default(
        &self,
        module_id: ModuleId,
        target_id: &TargetId,
    ) -> ProfileId {
        self.profile_id_for_target(module_id, target_id)
            .unwrap_or_else(|| self.default_profile_id_for_module(module_id))
    }

    /// Collect additive library types for a target.
    fn collect_types_for_target(
        target: &Target,
        compiler_options: &CompilerOptions,
        profile_config: Option<&ProfileConfig>,
    ) -> Vec<String> {
        let mut type_entries = Vec::new();

        // add compiler level type entries first
        if !compiler_options.types.is_empty() {
            type_entries.extend(compiler_options.types.clone());
        }

        // add target level type entries
        if let Some(target_types) = &target.types {
            type_entries.extend(target_types.clone());
        }

        // add profile level type entries
        if let Some(profile_types) = profile_config.and_then(|profile| profile.types.as_ref()) {
            type_entries.extend(profile_types.clone());
        }

        // only builtin compatible entries participate in profile libs
        builtin_libs_for_type_entries(&type_entries)
    }

    /// Build the effective library set for one target profile.
    fn effective_libs_for_target_profile(
        target: &Target,
        compiler_options: &CompilerOptions,
        profile_config: Option<&ProfileConfig>,
        tsconfig_options: Option<&TsConfigOptions>,
        runtime: Runtime,
        runtime_version: Option<String>,
        platform: Platform,
    ) -> Vec<String> {
        // derive fallback target settings for implicit lib resolution
        let derived_target = Target {
            runtime,
            runtime_version,
            platform,
            ..Target::default()
        };

        // pick one base lib list from config precedence
        let base_libs =
            if let Some(profile_lib) = profile_config.and_then(|profile| profile.lib.as_ref()) {
                profile_lib.clone()
            } else if let Some(target_lib) = target.lib.as_ref() {
                target_lib.clone()
            } else if !compiler_options.lib.is_empty() {
                compiler_options.lib.clone()
            } else if let Some(tsconfig_options) = tsconfig_options {
                typescript_default_libs(tsconfig_options)
            } else {
                derived_target.derived_lib()
            };

        // append additive type libs from config layers
        let mut libs = base_libs;
        let mut seen = HashSet::new();
        for lib_name in &libs {
            seen.insert(lib_name.clone());
        }

        let type_libs = Self::collect_types_for_target(target, compiler_options, profile_config);
        for type_lib in type_libs {
            if seen.insert(type_lib.clone()) {
                libs.push(type_lib);
            }
        }

        // lock native targets to native friendly libs
        if runtime.is_native() {
            libs.retain(|lib| {
                let Some(builtin) = destack_builtin::builtin_library(lib) else {
                    return true;
                };

                matches!(builtin.kind, destack_builtin::BuiltinLibraryKind::Language)
                    || matches!(builtin.name, "native" | "platform" | "destack")
            });
            if !libs.iter().any(|lib| lib == "native") {
                libs.push("native".to_string());
            }
        }

        libs
    }

    /// Build a profile key for a target.
    fn profile_key_for_target(
        target: &Target,
        compiler_options: &CompilerOptions,
        profile_config: Option<&ProfileConfig>,
        tsconfig_options: Option<&TsConfigOptions>,
    ) -> ProfileKey {
        let compiler_options = Self::compiler_options_for_target(target, compiler_options);
        let emit = target.emit;
        let runtime = profile_config
            .and_then(|profile| profile.runtime.as_ref().map(|runtime| runtime.host))
            .unwrap_or(target.runtime);
        let runtime_version = profile_config
            .and_then(|profile| {
                profile
                    .runtime
                    .as_ref()
                    .and_then(|runtime| runtime.version.clone())
            })
            .or_else(|| target.runtime_version.clone());
        let platform = profile_config
            .and_then(|profile| profile.platform)
            .unwrap_or(target.platform);
        let debug = profile_config
            .and_then(|profile| profile.debug)
            .unwrap_or(target.debug);

        // resolve effective libraries once using config precedence
        let libs = Self::effective_libs_for_target_profile(
            target,
            &compiler_options,
            profile_config,
            tsconfig_options,
            runtime,
            runtime_version,
            platform,
        );

        let env = profile_config
            .and_then(|profile| profile.comptime_env.as_ref())
            .or(compiler_options.comptime_env.as_ref())
            .map(|keys| EnvSnapshot::from_env_whitelist(keys))
            .unwrap_or_else(EnvSnapshot::from_env_all);

        let flags = profile_flags_for_compiler_options(&compiler_options);
        let (_, _, _, test) = ProfileEnv::mode_from_snapshot(&env, debug);

        ProfileKey::new(
            emit,
            runtime,
            platform,
            target.target_arch.clone(),
            target.target_vendor.clone(),
            target.target_env.clone(),
            libs,
            debug,
            test,
            compiler_options.skip_lib_check,
            env,
            flags,
        )
    }

    /// Build compiler options for a target, applying derived restrictions.
    pub fn compiler_options_for_target(
        target: &Target,
        compiler_options: &CompilerOptions,
    ) -> CompilerOptions {
        let mut options = compiler_options.clone();
        let is_native_output = target.emit.is_wasm() || target.emit.is_native();

        // require strict mode for native or wasm output
        if is_native_output {
            options.apply_native_restrictions();
        }

        options
    }

    /// Read package json module type for one package id.
    fn package_module_type_for_id(&self, package_id: PackageId) -> Option<String> {
        // skip missing package entries
        let package = self.packages.get_maybe(package_id)?;

        // read module type from package manifest
        let package = package.read();
        package
            .manifest
            .as_ref()
            .and_then(|manifest| manifest.content.module_type.clone())
    }

    /// Read module detection strategy from one tsconfig id.
    fn tsconfig_module_detection_for_id(&self, tsconfig_id: Option<TsConfigId>) -> ModuleDetection {
        // use defaults when no tsconfig is attached
        let Some(tsconfig_id) = tsconfig_id else {
            return ModuleDetection::default();
        };

        // use defaults when the tsconfig is missing
        let Some(tsconfig) = self.tsconfigs.get_maybe(tsconfig_id) else {
            return ModuleDetection::default();
        };

        // read module detection from tsconfig compiler options
        tsconfig.read().options.compiler.module_detection
    }

    /// Read module format override from one tsconfig id.
    fn tsconfig_module_format_for_id(
        &self,
        tsconfig_id: Option<TsConfigId>,
    ) -> Option<ModuleFormat> {
        // skip when no tsconfig is attached
        let tsconfig_id = tsconfig_id?;

        // skip when the tsconfig is missing
        let tsconfig = self.tsconfigs.get_maybe(tsconfig_id)?;
        let module_target = tsconfig.read().options.compiler.module;

        ModuleFormat::from_tsconfig_target(module_target)
    }

    /// Return true when a module AST contains top-level module syntax.
    fn module_has_import_export_syntax(
        &self,
        artifacts: &ArtifactStore,
        module_id: ModuleId,
    ) -> bool {
        let Some(ast) = artifacts.ast(module_id) else {
            return false;
        };

        // scan top-level expressions for import or export syntax
        for root_id in &ast.roots {
            if Self::expression_has_module_syntax_in_tree(&ast.tree, *root_id) {
                return true;
            }
        }

        false
    }

    /// Return true when one top-level expression contains module syntax.
    fn expression_has_module_syntax_in_tree(
        tree: &ast::NodeTree,
        expression_id: ast::LocalNodeId<ast::Expression>,
    ) -> bool {
        // unwrap statement wrappers at the top level
        let expression_id = Self::top_level_expression_without_statement(tree, expression_id);
        let expression = tree.get(expression_id);

        // treat module import statements as module syntax
        if let ast::Expression::Import { source, .. } = expression {
            return matches!(
                source,
                ast::ImportSource::ImportStatement | ast::ImportSource::ImportEquals
            );
        }

        // treat explicit export statements as module syntax
        if matches!(
            expression,
            ast::Expression::Export { .. } | ast::Expression::ExportNamespace { .. }
        ) {
            return true;
        }

        // treat declaration-style export modifiers as module syntax
        match expression {
            ast::Expression::Declaration(declaration_id) => {
                let declaration = tree.get(*declaration_id);
                declaration.descriptor().export.is_some()
            }
            ast::Expression::Let { descriptor, .. } | ast::Expression::Using { descriptor, .. } => {
                descriptor.export.is_some()
            }
            _ => false,
        }
    }

    /// Unwrap one top-level statement expression.
    fn top_level_expression_without_statement(
        tree: &ast::NodeTree,
        expression_id: ast::LocalNodeId<ast::Expression>,
    ) -> ast::LocalNodeId<ast::Expression> {
        let _ = tree;
        expression_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use destack_artifact::{EmitFormat, Platform, Runtime};

    use crate::{BorrowMode, DiagnosticPolicy, EsTarget};

    #[test]
    fn test_profile_key_for_target_appends_types() {
        // merges types into the normalized lib list
        let target = Target {
            runtime: Runtime::Node,
            platform: Platform::Web,
            lib: Some(vec!["es2020".to_string()]),
            types: Some(vec!["dom".to_string()]),
            ..Target::default()
        };

        let compiler_options = CompilerOptions {
            types: vec![
                "node".to_string(),
                "dom".to_string(),
                "vitest/globals".to_string(),
            ],
            ..CompilerOptions::default()
        };

        let profile_config = ProfileConfig {
            types: Some(vec!["deno".to_string()]),
            ..ProfileConfig::default()
        };

        let key = Program::profile_key_for_target(
            &target,
            &compiler_options,
            Some(&profile_config),
            None,
        );

        assert_eq!(
            key.lib,
            vec![
                "deno".to_string(),
                "dom".to_string(),
                "es2020".to_string(),
                "node".to_string()
            ]
        );
    }

    #[test]
    fn test_profile_key_for_target_uses_tsconfig_default_libs() {
        let target = Target::js("default");
        let compiler_options = CompilerOptions::default();

        let mut tsconfig_options = TsConfigOptions::default();
        tsconfig_options.compiler.es_target = EsTarget::Es2022;

        let key = Program::profile_key_for_target(
            &target,
            &compiler_options,
            None,
            Some(&tsconfig_options),
        );

        assert!(key.lib.iter().any(|lib| lib == "dom"));
        assert!(key.lib.iter().any(|lib| lib == "dom.iterable"));
        assert!(key.lib.iter().any(|lib| lib == "scripthost"));
        assert!(key.lib.iter().any(|lib| lib == "js"));
        assert!(key.lib.iter().any(|lib| lib == "es2022"));
        assert!(!key.lib.iter().any(|lib| lib == "node"));
    }

    #[test]
    fn test_profile_key_for_target_honors_tsconfig_no_lib() {
        let target = Target::js("default");
        let compiler_options = CompilerOptions::default();

        let mut tsconfig_options = TsConfigOptions::default();
        tsconfig_options.compiler.no_lib = true;

        let key = Program::profile_key_for_target(
            &target,
            &compiler_options,
            None,
            Some(&tsconfig_options),
        );

        assert!(key.lib.is_empty());
    }

    #[test]
    fn test_apply_tsconfig_profile_overrides_keeps_normalized_type_entries() {
        // configure tsconfig types with duplicates and non-builtin entries
        let mut tsconfig_options = TsConfigOptions::default();
        tsconfig_options.compiler.types = vec![
            "@types/node".to_string(),
            "NODE".to_string(),
            "vitest/globals".to_string(),
        ];

        // apply tsconfig profile overrides
        let mut compiler_options = CompilerOptions::default();
        Program::apply_tsconfig_profile_overrides(&mut compiler_options, &tsconfig_options);

        // keep normalized type entries for downstream resolution
        assert_eq!(
            compiler_options.types,
            vec!["node".to_string(), "vitest/globals".to_string()]
        );
    }

    #[test]
    fn test_profile_key_for_target_normalizes_tsconfig_lib_names() {
        let target = Target::js("default");
        let compiler_options = CompilerOptions::default();

        let mut tsconfig_options = TsConfigOptions::default();
        tsconfig_options.compiler.lib = vec!["DOM".to_string(), "lib.ES2022.d.ts".to_string()];

        let key = Program::profile_key_for_target(
            &target,
            &compiler_options,
            None,
            Some(&tsconfig_options),
        );

        assert!(key.lib.iter().any(|lib| lib == "dom"));
        assert!(key.lib.iter().any(|lib| lib == "es2022"));
        assert!(!key.lib.iter().any(|lib| lib == "DOM"));
        assert!(!key.lib.iter().any(|lib| lib == "lib.ES2022.d.ts"));
    }

    #[test]
    fn test_profile_key_for_target_uses_only_explicit_tsconfig_libs() {
        let target = Target::js("default");
        let compiler_options = CompilerOptions::default();

        let mut tsconfig_options = TsConfigOptions::default();
        tsconfig_options.compiler.lib = vec!["ESNext".to_string(), "DOM".to_string()];

        let key = Program::profile_key_for_target(
            &target,
            &compiler_options,
            None,
            Some(&tsconfig_options),
        );

        assert!(!key.lib.iter().any(|lib| lib == "node"));
        assert!(key.lib.iter().any(|lib| lib == "esnext"));
        assert!(key.lib.iter().any(|lib| lib == "dom"));
    }

    #[test]
    fn test_native_target_forces_strict_mode() {
        // set non-strict options to false to verify enforcement
        let compiler_options = CompilerOptions {
            strict: false,
            always_strict: false,
            no_implicit_any: DiagnosticPolicy::Allow,
            no_implicit_this: DiagnosticPolicy::Allow,
            strict_null_checks: false,
            strict_function_types: false,
            strict_bind_call_apply: false,
            strict_builtin_iterator_return: false,
            strict_property_initialization: false,
            use_unknown_in_catch_variables: false,
            ..CompilerOptions::default()
        };

        // enforce soundness defaults for native output
        let target = Target {
            emit: EmitFormat::Native,
            ..Target::default()
        };
        let options = Program::compiler_options_for_target(&target, &compiler_options);

        // verify strict options are set
        assert!(options.strict);
        assert!(options.always_strict);
        assert!(options.no_implicit_any.is_deny());
        assert!(options.no_implicit_this.is_deny());
        assert!(options.strict_null_checks);
        assert!(options.strict_function_types);
        assert!(options.strict_bind_call_apply);
        assert!(options.strict_builtin_iterator_return);
        assert!(options.strict_property_initialization);
        assert!(options.use_unknown_in_catch_variables);
        assert!(options.no_implicit_returns.is_deny());
        assert!(options.no_implicit_override.is_deny());
        assert!(options.exact_optional_property_types);
        assert!(options.no_unchecked_indexed_access.is_deny());
        assert!(options.no_property_access_from_index_signature.is_deny());
        assert!(options.no_unused_locals.is_allow());
        assert!(options.no_unused_parameters.is_allow());
        assert!(options.no_fallthrough_cases_in_switch.is_allow());
        assert!(options.allow_unreachable_code.is_allow());
        assert!(options.allow_unused_labels.is_allow());

        // verify soundness defaults are set
        assert!(options.no_any.is_deny());
        assert!(options.no_imprecise_primitives.is_deny());
        assert!(options.no_implicit_conversions.is_deny());
        assert!(options.no_unsafe_type_assertions.is_deny());
        assert!(options.no_must_assertions.is_deny());
        assert!(options.no_definite_assignment_assertions.is_deny());
        assert!(options.no_custom_type_guards.is_deny());
        assert!(options.no_unsound_variance.is_deny());
        assert!(options.no_unsound_narrowing.is_deny());
        assert!(options.deep_readonly.is_deny());
        assert!(options.no_untrusted_declarations.is_deny());
        assert!(options.no_implicit_managed.is_deny());
        assert!(options.no_managed.is_deny());
        assert!(options.no_dynamic_evaluation.is_deny());
        assert!(options.no_dynamic_import.is_deny());
        assert!(options.no_proxy.is_deny());
        assert!(options.no_dynamic_shapes.is_deny());
        assert!(options.no_exceptions.is_deny());
        assert!(options.no_global_this.is_deny());
        assert_eq!(options.borrow_mode, BorrowMode::Strict);
    }
}
