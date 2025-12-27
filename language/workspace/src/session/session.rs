use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use destack_base::StringPool;
use destack_source::{FileRegistry, FileSystem, ModuleId, PhysicalFileSystem};

use crate::{
    ArtifactRegistry, FormatterOptions, LanguageBuiltins, LinterOptions, ModuleRegistry,
    PackageRegistry, ProfileId, Program, TsConfigRegistry, Workspace,
};

/// A session is the persistent state for a workspace.
#[derive(Debug)]
pub struct Session {
    /// The workspace this session is for.
    pub workspace: Arc<Workspace>,
    /// The current working directory.
    pub cwd: PathBuf,
    /// The file system.
    pub fs: Arc<dyn FileSystem>,
    /// The files in the session.
    pub files: Arc<FileRegistry>,

    /// Default formatter options.
    pub formatter: FormatterOptions,
    /// Default linter options.
    pub linter: LinterOptions,

    /// The programs in the session, keyed by root path.
    pub programs: DashMap<PathBuf, Arc<Program>>,
    /// The package registry (for builtins and cross-program sharing).
    pub packages: Arc<PackageRegistry>,
    /// The module registry (for builtins).
    pub modules: Arc<ModuleRegistry>,
    /// The tsconfig registry (separate registry as tsconfigs can be nested within packages).
    pub tsconfigs: Arc<TsConfigRegistry>,
    /// The combined string pool.
    pub strings: Arc<StringPool>,
    /// The artifact registry (for generated artifacts from codegen).
    pub artifacts: Arc<ArtifactRegistry>,
    /// Compiled builtins (always loaded).
    pub builtins: Arc<LanguageBuiltins>,
}

impl Session {
    /// Create a new session with builtins loaded.
    pub fn new(cwd: PathBuf) -> Self {
        let files = Arc::new(FileRegistry::new());
        let modules = Arc::new(ModuleRegistry::new());
        let packages = Arc::new(PackageRegistry::new());
        let builtins = Arc::new(LanguageBuiltins::embedded(
            files.clone(),
            modules.clone(),
            packages.clone(),
        ));

        Self {
            workspace: Arc::new(Workspace::single_package(cwd.clone())),
            cwd,
            fs: Arc::new(PhysicalFileSystem::new()),
            files,

            formatter: FormatterOptions::default(),
            linter: LinterOptions::default(),

            programs: DashMap::new(),
            packages,
            modules,
            builtins,
            strings: Arc::new(StringPool::new()),
            artifacts: Arc::new(ArtifactRegistry::new()),
            tsconfigs: Arc::new(TsConfigRegistry::new()),
        }
    }

    /// Create a session for a workspace with builtins loaded.
    pub fn workspace(cwd: PathBuf, workspace: Arc<Workspace>) -> Self {
        let files = Arc::new(FileRegistry::new());
        let modules = Arc::new(ModuleRegistry::new());
        let packages = Arc::new(PackageRegistry::new());
        let builtins = Arc::new(LanguageBuiltins::embedded(
            files.clone(),
            modules.clone(),
            packages.clone(),
        ));

        Self {
            workspace,
            cwd,
            fs: Arc::new(PhysicalFileSystem::new()),
            files,

            formatter: FormatterOptions::default(),
            linter: LinterOptions::default(),

            programs: DashMap::new(),
            packages,
            modules,
            tsconfigs: Arc::new(TsConfigRegistry::new()),
            strings: Arc::new(StringPool::new()),
            artifacts: Arc::new(ArtifactRegistry::new()),
            builtins,
        }
    }

    /// Set the file system (builder pattern).
    pub fn with_fs(mut self, fs: Arc<dyn FileSystem>) -> Self {
        self.fs = fs;
        self
    }

    /// Set formatter options (builder pattern).
    pub fn with_formatter(mut self, formatter: FormatterOptions) -> Self {
        self.formatter = formatter;
        self
    }

    /// Set linter options (builder pattern).
    pub fn with_linter(mut self, linter: LinterOptions) -> Self {
        self.linter = linter;
        self
    }

    /// Load a lib module set (e.g., "dom", "es2024").
    /// Returns None if the lib name is not registered.
    pub fn load_lib(&self, name: &str) -> Option<Vec<ModuleId>> {
        self.builtins
            .load_lib(name, self.files.clone(), self.modules.clone())
    }

    /// Add a root to the session.
    /// Creates a new Program for the given root path.
    pub fn add_root(&self, root: PathBuf) -> Arc<Program> {
        let program = Arc::new(Program::new(
            self.formatter,
            self.linter.clone(),
            root.clone(),
            self.fs.clone(),
            self.files.clone(),
            self.modules.clone(),
            self.packages.clone(),
            self.tsconfigs.clone(),
            self.strings.clone(),
            self.artifacts.clone(),
            Some(self.builtins.clone()),
        ));
        self.programs.insert(root.clone(), program.clone());
        program
    }

    /// Get a program by root path.
    pub fn get_program(&self, root: &Path) -> Option<Arc<Program>> {
        self.programs.get(root).map(|p| p.clone())
    }

    /// Get or create a program for the given root path.
    pub fn get_or_create_program(&self, root: PathBuf) -> Arc<Program> {
        if let Some(program) = self.programs.get(&root) {
            return program.clone();
        }
        self.add_root(root)
    }

    /// Find the program that contains the given path.
    /// Falls back to the cwd-based program if no match is found.
    pub fn find_program_for_path(&self, path: &Path) -> Arc<Program> {
        // look for a program whose root is a prefix of the path
        for entry in self.programs.iter() {
            if path.starts_with(entry.key()) {
                return entry.value().clone();
            }
        }

        // fallback: create/get a program for the cwd
        self.get_or_create_program(self.cwd.clone())
    }

    /// Get the default profile for a module.
    pub fn default_profile_for_module(&self, module_id: ModuleId) -> ProfileId {
        let module = self.modules.get(module_id);
        let module = module.read();
        let program = module
            .path
            .as_ref()
            .map(|path| self.find_program_for_path(path))
            .unwrap_or_else(|| self.get_or_create_program(self.cwd.clone()));
        program.default_profile_id_for_module(module_id)
    }
}
