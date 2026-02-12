use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_ast as ast;
use destack_base::StringPool;
use destack_source::{
    DiagnosticCollector, DiagnosticStore, File, FileContent, FileId, FileRegistry, FileSystem,
    FileType, FileVersion, LanguageType, ModuleId, ModuleVersion, PackageId, PackageVersion, Uri,
};
use indexmap::IndexMap;
use parking_lot::RwLock;

use crate::{
    ArtifactRegistry, Builtins, DsConfigCompilerOptions, DsConfigOptions, EnvSnapshot,
    FormatterOptions, LinterOptions, Loader, Module, ModuleAst, ModuleDetection, ModuleFormat,
    ModuleGraphKey, ModuleGraphStamp, ModuleGraphVersion, ModuleRegistry, ModuleSource, Package,
    PackageKind, PackageRegistry, Profile, ProfileConfig, ProfileEnv, ProfileFlags, ProfileId,
    ProfileKey, ProfileRegistry, ProgramIndex, SourceType, Target, TargetId, TsConfigId,
    TsConfigOptions, TsConfigRegistry, WorkspaceFileEntry, WorkspaceIndexSnapshot,
    WorkspaceModuleEntry, payload_hash_from_bytes,
};

const TYPESCRIPT_LIB_PREFIX: &str = "lib.";
const TYPESCRIPT_LIB_SUFFIX: &str = ".d.ts";
const TYPESCRIPT_IMPLICIT_HOST_LIBS: &[&str] = &["dom", "dom.iterable", "scripthost"];

/// Unique identifier for Programs.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProgramId(pub u32);

impl std::fmt::Debug for ProgramId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl std::fmt::Display for ProgramId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl ProgramId {
    /// Wrap an id as a ProgramId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Digest of package versions for program-scoped tasks.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProgramStamp(pub u64);

impl std::fmt::Debug for ProgramStamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "p{:016x}", self.0)
    }
}

impl std::fmt::Display for ProgramStamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "p{:016x}", self.0)
    }
}

impl ProgramStamp {
    /// Create a new ProgramStamp.
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Get the raw stamp value.
    pub fn raw(self) -> u64 {
        self.0
    }
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
    /// Generated artifacts (from codegen).
    pub artifacts: Arc<ArtifactRegistry>,
    /// The profiles in this program.
    pub profiles: Arc<ProfileRegistry>,
    /// Derived tables and indexes for the program.
    pub index: Arc<ProgramIndex>,
    /// Loaded workspace index snapshot for cache rehydration.
    pub workspace_index: RwLock<Option<Arc<WorkspaceIndexSnapshot>>>,
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
        let artifacts = Arc::new(ArtifactRegistry::new());
        let profiles = Arc::new(ProfileRegistry::new());
        let index = Arc::new(ProgramIndex::new());
        let strings = Arc::new(StringPool::new());
        let diagnostics = DiagnosticCollector::new();
        let workspace_index = RwLock::new(None);
        let diagnostic_store = DiagnosticStore::new();

        // create and insert the root package and module (for global caching)
        let (root_module_id, fallback_file_id) =
            Self::make_root(&modules, &packages, files.clone());

        Self {
            formatter,
            linter,
            cwd,
            fs,
            files,

            modules,
            packages,
            tsconfigs,
            artifacts,
            profiles,
            index,
            strings,
            workspace_index,
            diagnostics,
            diagnostic_store,
            builtins: None,

            root_module_id,
            fallback_file_id,
        }
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
        artifacts: Arc<ArtifactRegistry>,
        builtins: Option<Arc<Builtins>>,
    ) -> Self {
        let diagnostics = DiagnosticCollector::new();
        let profiles = Arc::new(ProfileRegistry::new());
        let index = Arc::new(ProgramIndex::new());
        let workspace_index = RwLock::new(None);
        let diagnostic_store = DiagnosticStore::new();

        // create and insert the root package and module
        let (root_module_id, fallback_file_id) =
            Self::make_root(&modules, &packages, files.clone());

        Self {
            formatter,
            linter,
            cwd,
            fs,
            files,

            modules,
            packages,
            tsconfigs,
            artifacts,
            profiles,
            index,
            strings,
            workspace_index,
            diagnostics,
            diagnostic_store,
            builtins,

            root_module_id,
            fallback_file_id,
        }
    }

    /// Create and insert the root file, AST, module, and package for caching.
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
            dsconfig: None,
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

        // root AST (empty)
        let root_ast = ast::NodeTree::new();

        // root module (uses ephemeral module id)
        let root_module_id = ModuleId::EPHEMERAL;
        let mut root_module_ast = ModuleAst::from_tree(
            root_module_id,
            ModuleVersion::INITIAL,
            root_ast,
            Vec::new(),
            StringPool::new(),
            Vec::new(),
            Vec::new(),
        );
        root_module_ast.ensure_anchor_expression(root_file_id);
        let root_file = files.get(root_file_id);
        let root_module = Module::from_ast(
            root_module_id,
            root_file_id,
            root_file.version,
            root_uri,
            None,
            root_package_id,
            None, // no tsconfig for root module
            SourceType::Script,
            ModuleFormat::Esm,
            LanguageType::Destack,
            Loader::Destack,
            ModuleSource::User,
            root_module_ast,
        );

        modules.insert(root_module);
        (root_module_id, root_file_id)
    }

    /// Load a workspace index snapshot into program state.
    pub fn apply_workspace_index(&self, snapshot: WorkspaceIndexSnapshot) {
        // update the program-visible snapshot
        let snapshot = Arc::new(snapshot);
        *self.workspace_index.write() = Some(snapshot.clone());

        // reset cached graphs and signatures
        self.index.module_graphs.clear();
        self.index.module_graph_versions.clear();
        self.index.module_signatures.clear();
        self.index.module_signature_digests.clear();

        // seed module graph versions from the snapshot
        for (profile_id, version) in &snapshot.module_graph_versions {
            self.index
                .module_graph_versions
                .insert(*profile_id, *version);
        }

        // seed module graphs from the snapshot
        for (key, graph) in &snapshot.module_graphs {
            self.index.module_graphs.insert(*key, graph.clone());
            self.index
                .module_graph_versions
                .entry(key.profile_id)
                .or_insert(ModuleGraphVersion::INITIAL);
        }

        // seed signature digests from the snapshot
        for (key, digest) in &snapshot.module_signature_digests {
            self.index.module_signature_digests.insert(*key, *digest);
        }
    }

    /// Get the current module graph version for a profile.
    pub fn module_graph_version(&self, profile_id: ProfileId) -> ModuleGraphVersion {
        self.index
            .module_graph_versions
            .entry(profile_id)
            .or_insert(ModuleGraphVersion::INITIAL)
            .value()
            .to_owned()
    }

    /// Get the current module graph stamp for a profile.
    pub fn module_graph_stamp(&self, profile_id: ProfileId) -> ModuleGraphStamp {
        ModuleGraphStamp::new(profile_id, self.module_graph_version(profile_id))
    }

    /// Bump the module graph version for a profile.
    pub fn bump_module_graph_version(&self, profile_id: ProfileId) -> ModuleGraphVersion {
        let next = self.module_graph_version(profile_id).next();
        self.index.module_graph_versions.insert(profile_id, next);
        next
    }

    /// Drop cached module graphs for a profile and bump the graph version.
    pub fn drop_module_graph(&self, profile_id: ProfileId) {
        let graph_key = ModuleGraphKey::new(profile_id);
        self.index.module_graphs.remove(&graph_key);
        self.bump_module_graph_version(profile_id);
    }

    /// Get the workspace index file entry for a path.
    pub fn workspace_file_entry(&self, path: &Path) -> Option<WorkspaceFileEntry> {
        // read the workspace index snapshot when available
        let snapshot = self.workspace_index.read();
        let snapshot = snapshot.as_ref()?;
        snapshot.files.get(path).copied()
    }

    /// Get the workspace index module entry for a module id.
    pub fn workspace_module_entry(&self, module_id: ModuleId) -> Option<WorkspaceModuleEntry> {
        // read the workspace index snapshot when available
        let snapshot = self.workspace_index.read();
        let snapshot = snapshot.as_ref()?;
        snapshot.modules.get(&module_id).copied()
    }

    /// Compute a hash for the file contents at a path.
    fn file_content_hash_for_path(&self, path: &Path) -> Option<u64> {
        // reuse loaded file contents when available
        if let Some(file) = self.files.get_by_path(path) {
            match &file.content {
                FileContent::Text { content } => {
                    return Some(payload_hash_from_bytes(content.as_bytes()));
                }
                FileContent::Json { content, .. } => {
                    return Some(payload_hash_from_bytes(content.as_bytes()));
                }
                FileContent::Binary { content } => {
                    return Some(payload_hash_from_bytes(content));
                }
                FileContent::Missing => return None,
                FileContent::Unloaded => {}
            }
        }

        // fall back to reading from the file system
        let bytes = self.fs.read(path).ok()?;
        Some(payload_hash_from_bytes(&bytes))
    }

    /// Resolve a cached file version for a path using the workspace index.
    pub fn workspace_file_version_for_path(&self, path: &Path) -> FileVersion {
        // return the initial version when no snapshot exists
        let Some(entry) = self.workspace_file_entry(path) else {
            return FileVersion::INITIAL;
        };

        // compare against filesystem metadata when available
        let metadata = self.fs.metadata(path).ok();
        let Some(metadata) = metadata else {
            return entry.version_for_missing();
        };

        // return early when metadata changes
        let version = entry.version_for_metadata(&metadata);
        if version != entry.version {
            return version;
        }

        // skip content hashing when no hash is recorded
        let Some(expected_hash) = entry.content_hash else {
            return entry.version;
        };

        // compare against the current content hash
        let Some(current_hash) = self.file_content_hash_for_path(path) else {
            return entry.version.next();
        };

        if current_hash == expected_hash {
            entry.version
        } else {
            entry.version.next()
        }
    }

    /// Resolve a cached module version using the workspace index and file version.
    pub fn workspace_module_version_for_id(
        &self,
        module_id: ModuleId,
        file_version: FileVersion,
    ) -> ModuleVersion {
        // return the initial version when no snapshot exists
        let Some(entry) = self.workspace_module_entry(module_id) else {
            return ModuleVersion::INITIAL;
        };

        // use the stored version when the source version matches
        if entry.source_version == file_version {
            return entry.version;
        }

        entry.version.next()
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
            file_version,
            uri,
            None,
            package_id,
            None,
            source_type,
            module_format,
            language_type,
            loader,
            ModuleSource::User,
        );
        self.modules.insert(module);

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
    pub fn refresh_module_semantics(&self, module_id: ModuleId) {
        // capture current module state and detection inputs
        let module = self.modules.get(module_id);
        let (
            path,
            package_id,
            tsconfig_id,
            language_type,
            previous_source_type,
            previous_module_format,
            has_import_export,
        ) = {
            let module = module.read();
            (
                module.path.clone(),
                module.package_id,
                module.tsconfig_id,
                module.language_type,
                module.source_type,
                module.module_format,
                Self::module_has_import_export_syntax(&module),
            )
        };

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

        let mut module = module.write();
        module.source_type = source_type;
        module.module_format = module_format;
    }

    /// Increment a module version and return the updated value.
    pub fn bump_module_version(&self, module_id: ModuleId) -> ModuleVersion {
        // load module for update
        let module = self.modules.get(module_id);
        let mut module = module.write();

        // bump module version
        let next = module.version.next();
        module.version = next;

        next
    }

    /// Access tsconfig options for a module via closure.
    pub fn with_tsconfig_options<T>(
        &self,
        module: &Module,
        f: impl FnOnce(&TsConfigOptions) -> T,
    ) -> Option<T> {
        let tsconfig_id = module.tsconfig_id?;
        let tsconfig = self.tsconfigs.get(tsconfig_id);
        Some(f(&tsconfig.read().options))
    }

    /// Access dsconfig options for a module via closure.
    pub fn with_dsconfig_options<T>(
        &self,
        module: &Module,
        f: impl FnOnce(&DsConfigOptions) -> T,
    ) -> Option<T> {
        let package = self.packages.get(module.package_id);
        let package_guard = package.read();
        let dsconfig = package_guard.dsconfig.as_ref()?;
        Some(f(&dsconfig.options))
    }

    /// Get effective linter options for a module (package dsconfig > program defaults).
    pub fn get_linter_options(&self, module_id: ModuleId) -> LinterOptions {
        let module = self.modules.get(module_id);
        let module = module.read();

        // try package dsconfig first
        if let Some(options) = self.with_dsconfig_options(&module, |ds| ds.linter.clone()) {
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
    ) -> (DsConfigCompilerOptions, Option<TsConfigOptions>) {
        // start from package compiler options when dsconfig exists
        let mut compiler_options = package
            .dsconfig
            .as_ref()
            .map(|dsconfig| dsconfig.options.compiler.clone())
            .unwrap_or_default();

        // only use tsconfig when no package dsconfig exists
        let tsconfig_options = if package.dsconfig.is_none() {
            self.with_tsconfig_options(module, |options| options.clone())
        } else {
            None
        };

        // use tsconfig defaults when dsconfig is absent
        if let Some(tsconfig_options) = tsconfig_options.as_ref() {
            Self::apply_tsconfig_profile_overrides(&mut compiler_options, tsconfig_options);
        }

        (compiler_options, tsconfig_options)
    }

    /// Apply tsconfig settings that affect profile and resolution behavior.
    fn apply_tsconfig_profile_overrides(
        compiler_options: &mut DsConfigCompilerOptions,
        tsconfig_options: &TsConfigOptions,
    ) {
        let ts_compiler_options = &tsconfig_options.compiler;

        compiler_options.base_url = ts_compiler_options.base_url.clone();
        compiler_options.paths = ts_compiler_options.paths.as_ref().map(|paths| {
            let mut mapped_paths = crate::DsPathAliases::default();
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
            compiler_options.lib = Self::normalize_tsconfig_lib_names(&ts_compiler_options.lib);
        }

        if !ts_compiler_options.types.is_empty() {
            compiler_options.types = ts_compiler_options.types.clone();
        }
    }

    /// Get the profile id for a module with the default profile selection.
    pub fn default_profile_id_for_module(&self, module_id: ModuleId) -> ProfileId {
        // get package for module
        let module = self.modules.get(module_id);
        let module = module.read();
        let package = self.packages.get(module.package_id);
        let package = package.read();

        // derive profile compiler options from dsconfig or tsconfig
        let (compiler_options, tsconfig_options) =
            self.profile_compiler_options_for_module(&package, &module);

        // get target and profile config from package dsconfig
        let (target, profile_config) = if let Some(dsconfig) = package.dsconfig.as_ref() {
            // resolve the configured or fallback target
            let target = dsconfig
                .options
                .default_target
                .as_ref()
                .and_then(|name| {
                    let target_id = TargetId::new(package.id, name);
                    package.targets.get(&target_id)
                })
                .or_else(|| package.targets.values().find(|target| !target.synthetic))
                .cloned()
                .unwrap_or_else(|| self.fallback_target_for_module(&module));
            let profile_config = compiler_options
                .profile
                .as_ref()
                .and_then(|name| dsconfig.options.profiles.get(name));
            (target, profile_config)
        } else {
            // pick a target based on the module language when no dsconfig exists
            (self.fallback_target_for_module(&module), None)
        };

        let key = Self::profile_key_for_target(
            &target,
            &compiler_options,
            profile_config,
            tsconfig_options.as_ref(),
        );

        self.profiles.get_or_create(key)
    }

    /// Ensure a target exists for a module and return its id.
    pub fn ensure_target_for_module(&self, module_id: ModuleId) -> TargetId {
        let module = self.modules.get(module_id);
        let module = module.read();
        let package_id = module.package_id;
        let diagnostic_id = TargetId::new(package_id, Self::DIAGNOSTIC_TARGET_NAME);

        let package = self.packages.get(package_id);
        let mut package = package.write();

        if package.targets.contains_key(&diagnostic_id) {
            return diagnostic_id;
        }

        if let Some((existing_id, _)) = package.targets.iter().find(|(_, target)| {
            !target.synthetic && (target.output.is_native() || target.output.is_wasm())
        }) {
            return existing_id.clone();
        }

        // resolve base target for diagnostics
        let base_target = if let Some(dsconfig) = package.dsconfig.as_ref() {
            dsconfig
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
                .unwrap_or_else(|| self.fallback_target_for_module(&module))
        } else {
            self.fallback_target_for_module(&module)
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
    /// FUGU #Cleanup: should we have fallback profiles at all? or only explicitly (sometimes?)?
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
        let module = module.read();
        let package = self.packages.get(module.package_id);
        let package = package.read();
        let target = package
            .targets
            .get(target_id)
            .cloned()
            .or_else(|| Target::implicit_for_name(&target_id.name))?;

        let (compiler_options, tsconfig_options) =
            self.profile_compiler_options_for_module(&package, &module);
        let profile_config = package.dsconfig.as_ref().and_then(|dsconfig| {
            target
                .profile
                .as_ref()
                .or(compiler_options.profile.as_ref())
                .and_then(|name| dsconfig.options.profiles.get(name))
        });

        let key = Self::profile_key_for_target(
            &target,
            &compiler_options,
            profile_config,
            tsconfig_options.as_ref(),
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
        compiler_options: &DsConfigCompilerOptions,
        profile_config: Option<&ProfileConfig>,
    ) -> Vec<String> {
        let mut types = Vec::new();

        // add compiler level types first
        if !compiler_options.types.is_empty() {
            types.extend(compiler_options.types.clone());
        }

        // add target level types
        if let Some(target_types) = &target.types {
            types.extend(target_types.clone());
        }

        // add profile level types
        if let Some(profile_types) = profile_config.and_then(|profile| profile.types.as_ref()) {
            types.extend(profile_types.clone());
        }

        types
    }

    /// Normalize one tsconfig lib name to builtin lookup form.
    fn normalize_tsconfig_lib_name(lib: &str) -> String {
        // normalize casing and whitespace
        let lower = lib.trim().to_ascii_lowercase();
        let without_prefix = lower
            .strip_prefix(TYPESCRIPT_LIB_PREFIX)
            .unwrap_or(lower.as_str());
        let normalized = without_prefix
            .strip_suffix(TYPESCRIPT_LIB_SUFFIX)
            .unwrap_or(without_prefix);

        normalized.to_string()
    }

    /// Normalize tsconfig lib names to builtin lookup form.
    fn normalize_tsconfig_lib_names(libs: &[String]) -> Vec<String> {
        libs.iter()
            .map(|lib| Self::normalize_tsconfig_lib_name(lib))
            .collect()
    }

    /// Build implicit tsconfig libs when no explicit profile libs are configured.
    fn default_libs_for_tsconfig(tsconfig_options: &TsConfigOptions) -> Vec<String> {
        let ts_compiler_options = &tsconfig_options.compiler;

        // tsconfig noLib disables implicit libs
        if ts_compiler_options.no_lib {
            return Vec::new();
        }

        // explicit tsconfig lib entries fully define the base lib set
        if !ts_compiler_options.lib.is_empty() {
            return Self::normalize_tsconfig_lib_names(&ts_compiler_options.lib);
        }

        // use TypeScript default ambient libs first
        let mut libs = vec![
            "js".to_string(),
            ts_compiler_options.es_target.default_lib_name().to_string(),
        ];

        // add typescript host libs
        for lib in TYPESCRIPT_IMPLICIT_HOST_LIBS {
            libs.push((*lib).to_string());
        }

        libs
    }

    /// Build a profile key for a target.
    fn profile_key_for_target(
        target: &Target,
        compiler_options: &DsConfigCompilerOptions,
        profile_config: Option<&ProfileConfig>,
        tsconfig_options: Option<&TsConfigOptions>,
    ) -> ProfileKey {
        let compiler_options = Self::compiler_options_for_target(target, compiler_options);
        let output = target.output;
        let runtime = profile_config
            .and_then(|profile| profile.runtime)
            .unwrap_or(target.runtime);
        let runtime_version = profile_config
            .and_then(|profile| profile.runtime_version.clone())
            .or_else(|| target.runtime_version.clone());
        let platform = profile_config
            .and_then(|profile| profile.platform)
            .unwrap_or(target.platform);
        let debug = profile_config
            .and_then(|profile| profile.debug)
            .unwrap_or(target.debug);

        // derive fallback target settings once for implicit lib resolution
        let derived_target = Target {
            runtime,
            runtime_version,
            platform,
            ..Target::default()
        };

        // pick one base lib list from config precedence
        let base_lib =
            if let Some(profile_lib) = profile_config.and_then(|profile| profile.lib.as_ref()) {
                profile_lib.clone()
            } else if let Some(target_lib) = target.lib.as_ref() {
                target_lib.clone()
            } else if !compiler_options.lib.is_empty() {
                compiler_options.lib.clone()
            } else if let Some(tsconfig_options) = tsconfig_options {
                if tsconfig_options.compiler.no_lib {
                    Vec::new()
                } else if !tsconfig_options.compiler.lib.is_empty() {
                    Self::normalize_tsconfig_lib_names(&tsconfig_options.compiler.lib)
                } else {
                    Self::default_libs_for_tsconfig(tsconfig_options)
                }
            } else {
                derived_target.derived_lib()
            };

        // append additive library types from config layers
        let mut libs = base_lib;
        let mut seen = HashSet::new();
        for lib_name in &libs {
            seen.insert(lib_name.clone());
        }

        // regular additive "types" libs
        let types = Self::collect_types_for_target(target, &compiler_options, profile_config);
        for type_name in types {
            if seen.insert(type_name.clone()) {
                libs.push(type_name);
            }
        }

        // lock native targets to native-friendly libs (#Cleanup)
        if runtime.is_native() {
            libs.retain(|lib| {
                let Some(builtin) = destack_builtin::builtin_lib(lib) else {
                    return true;
                };
                matches!(builtin.kind, destack_builtin::BuiltinLibKind::Std)
                    || matches!(builtin.name, "native" | "platform" | "destack")
            });
            if !libs.iter().any(|lib| lib == "native") {
                libs.push("native".to_string());
            }
        }

        let env = profile_config
            .and_then(|profile| profile.comptime_env.as_ref())
            .or(compiler_options.comptime_env.as_ref())
            .map(|keys| EnvSnapshot::from_env_whitelist(keys))
            .unwrap_or_else(EnvSnapshot::from_env_all);

        let flags = ProfileFlags::from(&compiler_options);
        let (_, _, _, test) = ProfileEnv::mode_from_snapshot(&env, debug);

        ProfileKey::new(
            output,
            runtime,
            platform,
            target.target_arch.clone(),
            target.target_vendor.clone(),
            target.target_env.clone(),
            libs,
            debug,
            test,
            env,
            flags,
        )
    }

    /// Build compiler options for a target, applying derived restrictions.
    pub fn compiler_options_for_target(
        target: &Target,
        compiler_options: &DsConfigCompilerOptions,
    ) -> DsConfigCompilerOptions {
        let mut options = compiler_options.clone();
        let is_native_output = target.output.is_wasm() || target.output.is_native();

        // require strict mode for native or wasm output
        if is_native_output {
            options.apply_native_restrictions();
        }

        options
    }

    /// Read package json module type for one package id.
    fn package_module_type_for_id(&self, package_id: PackageId) -> Option<String> {
        // skip missing package entries
        let Some(package) = self.packages.get_maybe(package_id) else {
            return None;
        };

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
    fn module_has_import_export_syntax(module: &Module) -> bool {
        // skip modules without parsed ASTs
        let Some(ast) = module.ast_maybe() else {
            return false;
        };

        // scan top-level expressions for import or export syntax
        for root_id in &ast.roots {
            if Self::expression_has_module_syntax(ast, *root_id) {
                return true;
            }
        }

        false
    }

    /// Return true when one top-level expression contains module syntax.
    fn expression_has_module_syntax(
        ast: &ModuleAst,
        expression_id: ast::LocalNodeId<ast::Expression>,
    ) -> bool {
        // unwrap statement wrappers at the top level
        let expression_id = Self::top_level_expression_without_statement(&ast.tree, expression_id);
        let expression = ast.tree.get(expression_id);

        // treat explicit imports as module syntax
        if let ast::Expression::Import { source, .. } = expression {
            return *source != ast::ImportSource::ImportCall;
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
                let declaration = ast.tree.get(*declaration_id);
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
        let expression = tree.get(expression_id);
        if let ast::Expression::Statement(statement_id) = expression {
            *statement_id
        } else {
            expression_id
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BorrowMode, DiagnosticPolicy, OutputFormat, Platform, Runtime};

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

        let compiler_options = DsConfigCompilerOptions {
            types: vec!["node".to_string(), "dom".to_string()],
            ..DsConfigCompilerOptions::default()
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
        let compiler_options = DsConfigCompilerOptions::default();

        let mut tsconfig_options = TsConfigOptions::default();
        tsconfig_options.compiler.es_target = crate::EsTarget::Es2022;

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
        let compiler_options = DsConfigCompilerOptions::default();

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
    fn test_profile_key_for_target_normalizes_tsconfig_lib_names() {
        let target = Target::js("default");
        let compiler_options = DsConfigCompilerOptions::default();

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
        let compiler_options = DsConfigCompilerOptions::default();

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
        let compiler_options = DsConfigCompilerOptions {
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
            ..DsConfigCompilerOptions::default()
        };

        // enforce soundness defaults for native output
        let target = Target {
            output: OutputFormat::Native,
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
        assert!(options.no_unused_locals.is_deny());
        assert!(options.no_unused_parameters.is_deny());
        assert!(options.no_fallthrough_cases_in_switch.is_deny());
        assert!(options.allow_unreachable_code.is_deny());
        assert!(options.allow_unused_labels.is_deny());

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
