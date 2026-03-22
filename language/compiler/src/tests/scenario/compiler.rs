use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{
    File, FileId, FileType, FileVersion, ModuleId, TemporaryPhysicalFileSystem, Uri,
};
use destack_workspace::{
    ArtifactKey, Ast, Destack, DirPrepared, DirResolved, FileUpdate, MemoryCacheStore, Program,
    Session, Workspace,
};

use crate::{Compiler, CompilerOptions};

use super::{CompilerEdit, CompilerEditScript};

const DEFAULT_DISK_CACHE_CONFIG: &str = r#"{ "cache": { "mode": "disk" } }"#;

/// The cache mode used by one scenario workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScenarioCacheMode {
    /// Disable persisted artifact image reuse.
    Off,
    /// Enable persisted artifact image reuse.
    Disk,
}

/// The ambient language surface used by one scenario workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScenarioLanguageSurface {
    /// Use the normal prelude and profile libraries.
    Full,
    /// Use a minimal surface without prelude or profile libraries.
    Minimal,
}

/// One UTF-8 source file in a compiler scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ScenarioFile {
    /// The relative workspace path.
    path: PathBuf,
    /// The file contents.
    content: String,
}

/// A reusable physical workspace scenario for compiler state tests.
#[derive(Debug, Clone)]
pub(crate) struct CompilerScenario {
    /// The temp-root prefix used when materializing the scenario.
    prefix: String,
    /// The initial workspace files.
    files: Vec<ScenarioFile>,
    /// The root module paths.
    roots: Vec<PathBuf>,
    /// The persisted cache mode.
    cache_mode: ScenarioCacheMode,
    /// The ambient language surface.
    language_surface: ScenarioLanguageSurface,
    /// The compiler worker count for each run.
    workers: u16,
}

impl CompilerScenario {
    /// Create an empty compiler scenario.
    pub(crate) fn new() -> Self {
        Self {
            prefix: "compiler_scenario".to_string(),
            files: Vec::new(),
            roots: Vec::new(),
            cache_mode: ScenarioCacheMode::Off,
            language_surface: ScenarioLanguageSurface::Full,
            workers: 1,
        }
    }

    /// Set the temp-root prefix.
    pub(crate) fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Enable disk-backed persisted artifact images.
    pub(crate) fn disk_cache(mut self) -> Self {
        self.cache_mode = ScenarioCacheMode::Disk;
        self
    }

    /// Use a minimal ambient language surface for this scenario.
    pub(crate) fn minimal_language_surface(mut self) -> Self {
        self.language_surface = ScenarioLanguageSurface::Minimal;
        self
    }

    /// Set the compiler worker count for each run.
    pub(crate) fn workers(mut self, workers: u16) -> Self {
        self.workers = workers.max(1);
        self
    }

    /// Add one source file to the scenario.
    pub(crate) fn file(mut self, path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        self.files.push(ScenarioFile {
            path: path.into(),
            content: content.into(),
        });
        self
    }

    /// Add one root module path to the scenario.
    pub(crate) fn root(mut self, path: impl Into<PathBuf>) -> Self {
        self.roots.push(path.into());
        self
    }

    /// Materialize the scenario into one physical workspace.
    pub(crate) fn materialize(&self) -> CompilerScenarioWorkspace {
        let root = TemporaryPhysicalFileSystem::new_with_prefix(&self.prefix);

        // workspace manifests
        if !root.path_for("package.json").exists() {
            root.write_bytes_or_error("package.json", br#"{ "name": "compiler-scenario" }"#);
        }

        let workspace_config_text = self.effective_workspace_config_text();
        if let Some(text) = workspace_config_text.as_deref() {
            root.write_text_or_error("destack.json", text);
        }

        // initial files
        for file in &self.files {
            root.write_text_or_error(&file.path, &file.content);
        }

        CompilerScenarioWorkspace {
            scenario: self.clone(),
            root: Arc::new(root),
        }
    }

    /// Resolve the effective workspace config text for the scenario.
    fn effective_workspace_config_text(&self) -> Option<String> {
        match self.cache_mode {
            ScenarioCacheMode::Disk => Some(DEFAULT_DISK_CACHE_CONFIG.to_string()),
            ScenarioCacheMode::Off => None,
        }
    }
}

impl Default for CompilerScenario {
    fn default() -> Self {
        Self::new()
    }
}

/// One materialized compiler scenario workspace.
#[derive(Debug, Clone)]
pub(crate) struct CompilerScenarioWorkspace {
    /// The scenario definition.
    scenario: CompilerScenario,
    /// The physical workspace root.
    root: Arc<TemporaryPhysicalFileSystem>,
}

impl CompilerScenarioWorkspace {
    /// Return the physical workspace root path.
    pub(crate) fn root_path(&self) -> &Path {
        self.root.root()
    }

    /// Resolve one relative path under the workspace root.
    pub(crate) fn path_for(&self, path: impl AsRef<Path>) -> PathBuf {
        self.root.path_for(path)
    }

    /// Open one live compiler session for the current workspace contents.
    pub(crate) fn open(&self) -> CompilerScenarioRun {
        self.open_with_options(|_| {})
    }

    /// Open one live compiler session with customized compiler options.
    pub(crate) fn open_with_options<F>(&self, configure: F) -> CompilerScenarioRun
    where
        F: FnOnce(&mut CompilerOptions),
    {
        // build the session and workspace view
        let root_path = self.root_path().to_path_buf();
        let workspace = self.workspace();
        let mut session = Session::workspace(root_path.clone(), Arc::new(workspace));

        // cache backend
        if self.scenario.cache_mode == ScenarioCacheMode::Off {
            session = session.with_cache_store(Arc::new(MemoryCacheStore::new()));
        }

        // build the live program and compiler
        let session = Arc::new(session);
        let program = session.add_root(root_path.clone());
        let (inject_prelude, load_libraries) = match self.scenario.language_surface {
            ScenarioLanguageSurface::Full => (true, true),
            ScenarioLanguageSurface::Minimal => (false, false),
        };
        let mut options = CompilerOptions {
            workers: self.scenario.workers,
            inject_prelude,
            load_libraries,
            ..CompilerOptions::default()
        };
        configure(&mut options);
        let compiler = Arc::new(Compiler::new(session.clone(), program.clone(), options));

        CompilerScenarioRun {
            workspace: self.clone(),
            program,
            compiler,
        }
    }

    /// Apply one edit directly to the physical workspace.
    pub(crate) fn apply_edit(&self, edit: &CompilerEdit) {
        match edit {
            CompilerEdit::ReplaceFile { path, content } => {
                self.root.write_text_or_error(path, content);
            }
        }
    }

    /// Build the workspace snapshot for one fresh session.
    fn workspace(&self) -> Workspace {
        let root_path = self.root_path().to_path_buf();
        let Some(config_text) = self.scenario.effective_workspace_config_text() else {
            return Workspace::single_package(root_path);
        };

        let config_path = self.path_for("destack.json");
        let config = parse_workspace_config(&config_path, &config_text);
        Workspace::single_package(root_path).with_config(config)
    }
}

/// One live scenario session and compiler.
#[derive(Debug, Clone)]
pub(crate) struct CompilerScenarioRun {
    /// The materialized workspace.
    workspace: CompilerScenarioWorkspace,
    /// The live program.
    program: Arc<Program>,
    /// The live compiler.
    compiler: Arc<Compiler>,
}

impl CompilerScenarioRun {
    /// Return the live program.
    pub(crate) fn program(&self) -> &Arc<Program> {
        &self.program
    }

    /// Return the live compiler.
    pub(crate) fn compiler(&self) -> &Arc<Compiler> {
        &self.compiler
    }

    /// Resolve one relative path under the workspace root.
    pub(crate) fn path_for(&self, path: impl AsRef<Path>) -> PathBuf {
        self.workspace.path_for(path)
    }

    /// Resolve one scenario module handle from a workspace-relative path.
    pub(crate) fn module(&self, path: impl Into<PathBuf>) -> ScenarioModule {
        let path = path.into();
        let module_id = self.module_id(&path);

        ScenarioModule {
            run: self.clone(),
            path,
            module_id,
        }
    }

    /// Resolve one module id from a workspace-relative path.
    pub(crate) fn module_id(&self, path: impl AsRef<Path>) -> ModuleId {
        let path = self.path_for(path);
        self.compiler
            .resolve_path_to_module(&path)
            .unwrap_or_else(|error| panic!("failed to resolve scenario module: {error:?}"))
    }

    /// Apply one edit to the live program and physical workspace.
    pub(crate) fn apply_edit(&self, edit: &CompilerEdit) {
        // resolve the absolute workspace path first
        let absolute_path = match edit {
            CompilerEdit::ReplaceFile { path, .. } => self.path_for(path),
        };

        // physical filesystem
        self.workspace.apply_edit(edit);

        // live invalidation
        let Some(file_id) = self.program.files.get_id_by_path(&absolute_path) else {
            return;
        };

        // propagate the updated file contents into the live program
        match edit {
            CompilerEdit::ReplaceFile { content, .. } => {
                self.program
                    .invalidate_file(
                        file_id,
                        FileUpdate::Text {
                            content: content.clone(),
                        },
                    )
                    .unwrap_or_else(|error| {
                        panic!("failed to invalidate replaced scenario file: {error}")
                    });
            }
        }
    }

    /// Apply one edit script to the live program and physical workspace.
    pub(crate) fn apply_script(&self, script: &CompilerEditScript) {
        // replay edits in order
        for edit in script.iter() {
            self.apply_edit(edit);
        }
    }

    /// Reopen the same physical workspace as a fresh compiler session.
    pub(crate) fn reopen_fresh(&self) -> Self {
        self.workspace.open()
    }
}

/// One scenario module handle.
#[derive(Debug, Clone)]
pub(crate) struct ScenarioModule {
    /// The owning scenario run.
    run: CompilerScenarioRun,
    /// The workspace-relative module path.
    path: PathBuf,
    /// The resolved module id in this run.
    module_id: ModuleId,
}

impl ScenarioModule {
    /// Return the live module id.
    pub(crate) fn module_id(&self) -> ModuleId {
        self.module_id
    }

    /// Return the default profile id for this module.
    pub(crate) fn profile_id(&self) -> destack_workspace::ProfileId {
        self.run
            .program
            .default_profile_id_for_module(self.module_id)
    }

    /// Return the AST artifact key for this module.
    pub(crate) fn ast_key(&self) -> ArtifactKey {
        ArtifactKey::ast(self.module_id)
    }

    /// Return the resolved DIR artifact key for this module.
    pub(crate) fn dir_resolved_key(&self) -> ArtifactKey {
        ArtifactKey::dir_resolved(self.module_id, self.profile_id())
    }

    /// Return whether the AST is currently available.
    pub(crate) fn ast_is_available(&self) -> bool {
        self.run.compiler.artifact_key_is_available(&self.ast_key())
    }

    /// Return whether the resolved DIR is currently available.
    pub(crate) fn dir_resolved_is_available(&self) -> bool {
        self.run
            .compiler
            .artifact_key_is_available(&self.dir_resolved_key())
    }

    /// Require the AST for this module.
    pub(crate) fn require_ast(&self) {
        self.run
            .compiler
            .run_to_completion(|compiler| compiler.require_ast(self.module_id))
            .unwrap_or_else(|error| panic!("failed to require scenario ast: {error:?}"));
    }

    /// Require the prepared DIR for this module.
    pub(crate) fn require_dir_prepared(&self) {
        // resolve the ambient profile once
        let profile_id = self.profile_id();

        // build the prepared dir to completion
        self.run
            .compiler
            .run_to_completion(|compiler| compiler.require_dir_prepared(self.module_id, profile_id))
            .unwrap_or_else(|error| panic!("failed to require scenario prepared dir: {error:?}"));
    }

    /// Require the resolved DIR for this module.
    pub(crate) fn require_dir_resolved(&self) {
        // resolve the ambient profile once
        let profile_id = self.profile_id();

        // build the resolved dir to completion
        self.run
            .compiler
            .run_to_completion(|compiler| compiler.require_dir_resolved(self.module_id, profile_id))
            .unwrap_or_else(|error| panic!("failed to require scenario resolved dir: {error:?}"));
    }

    /// Return the published AST for this module.
    pub(crate) fn ast(&self) -> std::sync::Arc<destack_workspace::Ast> {
        self.run
            .program
            .artifacts
            .ast(self.module_id)
            .unwrap_or_else(|| panic!("expected published scenario ast"))
    }

    /// Return the published prepared DIR for this module.
    pub(crate) fn dir_prepared(&self) -> std::sync::Arc<DirPrepared> {
        self.run
            .program
            .artifacts
            .dir_prepared(self.module_id, self.profile_id())
            .unwrap_or_else(|| panic!("expected published scenario prepared dir"))
    }

    /// Return the published resolved DIR for this module.
    pub(crate) fn dir_resolved(&self) -> std::sync::Arc<DirResolved> {
        self.run
            .program
            .artifacts
            .dir_resolved(self.module_id, self.profile_id())
            .unwrap_or_else(|| panic!("expected published scenario resolved dir"))
    }

    /// Load the persisted AST image for this module when available.
    pub(crate) fn load_ast_image(&self) -> Option<Ast> {
        // derive the persisted ast identity inputs
        let module = self.run.program.modules.get(self.module_id);
        let language_type = match module.loader {
            destack_workspace::Loader::Destack
            | destack_workspace::Loader::TypeScript
            | destack_workspace::Loader::JavaScript => Some(module.language_type),
            _ => None,
        };
        let file_id = module.file_id;

        // load the current file view before reading a persisted ast image
        let file = self.run.program.files.get(file_id);
        if !file.is_loaded() {
            let path = self.run.path_for(&self.path);
            let content = self
                .run
                .program
                .fs
                .read_to_string(&path)
                .unwrap_or_else(|error| {
                    panic!("failed to load scenario source file for ast image: {error}")
                });
            let loaded_file = File::from_text(
                file_id,
                file.name.clone(),
                file.uri.clone(),
                file.path.clone(),
                file.ty,
                content,
            );
            self.run.program.files.replace(loaded_file);
        }
        let file = self.run.program.files.get(file_id);

        // probe the persisted image through the compiler cache path
        self.run
            .compiler
            .load_ast_image(
                self.module_id,
                self.run.compiler.module_version(self.module_id),
                file.as_ref(),
                language_type,
            )
            .unwrap_or_else(|error| panic!("failed to load persisted scenario ast image: {error}"))
    }

    /// Replace the module contents and invalidate live state.
    pub(crate) fn replace(&self, content: impl Into<String>) {
        self.run
            .apply_edit(&CompilerEdit::replace_file(self.path.clone(), content));
    }
}

/// Parse one workspace config file.
pub(crate) fn parse_workspace_config(config_path: &Path, config_content: &str) -> Destack {
    // parse the config file through the normal source pipeline
    let file = File::from_text_as_jsonc(
        FileId::new(1),
        "destack.json".to_string(),
        Uri::from_path(config_path),
        Some(config_path.to_path_buf()),
        FileType::Json,
        config_content.to_string(),
    )
    .unwrap_or_else(|error| panic!("failed to parse destack.json: {error}"));
    let file = Arc::new(file.with_version(FileVersion::INITIAL));

    Destack::parse(&file).unwrap_or_else(|error| panic!("failed to build destack.json: {error}"))
}

/// Build one compiler over a disk-cache-enabled temporary workspace.
pub(crate) fn build_disk_cache_compiler(
    root: &TemporaryPhysicalFileSystem,
) -> (Arc<Session>, Arc<Program>, Compiler, PathBuf) {
    // resolve the fixed workspace paths
    let root_path = root.root().to_path_buf();
    let config_path = root.path_for("destack.json");
    let config_content = DEFAULT_DISK_CACHE_CONFIG;
    let package_manifest_path = root.path_for("package.json");
    let source_path = root.path_for("main.ts");

    // workspace files
    if !package_manifest_path.exists() {
        root.write_bytes_or_error("package.json", br#"{ "name": "artifact-image-test" }"#);
    }
    if !config_path.exists() {
        root.write_text_or_error("destack.json", config_content);
    }
    if !source_path.exists() {
        root.write_text_or_error("main.ts", "export const value: number = 1;");
    }

    // workspace config
    let config = parse_workspace_config(&config_path, config_content);
    let workspace = Workspace::single_package(root_path.clone()).with_config(config);
    let session = Arc::new(Session::workspace(root_path.clone(), Arc::new(workspace)));
    let program = session.add_root(root_path.clone());

    // build the compiler with normal disk cache behavior
    let compiler = Compiler::new(
        session.clone(),
        program.clone(),
        CompilerOptions {
            workers: 1,
            ..CompilerOptions::default()
        },
    );

    (session, program, compiler, root.path_for("main.ts"))
}

/// Build one compiler over a temporary workspace without persisted artifact caching.
pub(crate) fn build_memory_cache_compiler(
    root: &TemporaryPhysicalFileSystem,
) -> (Arc<Session>, Arc<Program>, Compiler, PathBuf) {
    // resolve the fixed workspace paths
    let root_path = root.root().to_path_buf();
    let package_manifest_path = root.path_for("package.json");
    let source_path = root.path_for("main.ts");

    // workspace files
    if !package_manifest_path.exists() {
        root.write_bytes_or_error("package.json", br#"{ "name": "artifact-image-test" }"#);
    }
    if !source_path.exists() {
        root.write_text_or_error("main.ts", "export const value: number = 1;");
    }

    // workspace config
    let workspace = Workspace::single_package(root_path.clone());
    let session = Arc::new(
        Session::workspace(root_path.clone(), Arc::new(workspace))
            .with_cache_store(Arc::new(MemoryCacheStore::new())),
    );
    let program = session.add_root(root_path.clone());

    // build the compiler over the in memory cache backend
    let compiler = Compiler::new(
        session.clone(),
        program.clone(),
        CompilerOptions {
            workers: 1,
            ..CompilerOptions::default()
        },
    );

    (session, program, compiler, root.path_for("main.ts"))
}
