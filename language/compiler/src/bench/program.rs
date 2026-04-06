use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use destack_artifact::{ArtifactKey, CacheStore, MemoryCacheStore};
use destack_source::{
    DiagnosticCollection, DiagnosticSeverity, File, FileSystem, MemoryFileSystem, ModuleId,
    PrintOptions, ProfileId,
};
use destack_workspace::{Module, Ref, Repository, Workspace};

use crate::{Compiler, CompilerOptions, TaskPhase};

/// One repository-backed bench workspace view.
#[derive(Debug)]
pub(super) struct BenchRepository {
    /// The wrapped repository.
    repository: Arc<Repository>,
    /// The workspace root used for revision reads.
    root_directory: PathBuf,
}

impl BenchRepository {
    /// Create one bench repository view over one workspace root.
    fn new(repository: Arc<Repository>, root_directory: PathBuf) -> Self {
        Self {
            repository,
            root_directory,
        }
    }

    /// Return the current workspace revision.
    fn current_revision(&self) -> destack_workspace::Revision {
        let reference = Ref::for_workspace_root(&self.root_directory);
        self.repository
            .current(&reference)
            .unwrap_or_else(|error| panic!("missing current workspace revision: {error}"))
    }

    /// Return the default profile id for one module in the current revision.
    fn default_profile_id_for_module(&self, module_id: ModuleId) -> ProfileId {
        self.repository
            .default_profile_id_for_module(self.current_revision(), module_id)
            .unwrap_or_else(|error| {
                panic!("failed to compute default profile for module {module_id:?}: {error}")
            })
    }

    /// Return one revision-scoped module snapshot.
    fn module_snapshot(&self, module_id: ModuleId) -> Arc<Module> {
        self.repository
            .module(self.current_revision(), module_id)
            .unwrap_or_else(|error| panic!("failed to read module {module_id:?}: {error}"))
            .unwrap_or_else(|| panic!("missing module {module_id:?}"))
    }

    /// Return one revision-scoped file snapshot when present.
    fn source_file_maybe(&self, file_id: destack_source::FileId) -> Option<Arc<File>> {
        self.repository
            .file(self.current_revision(), file_id)
            .unwrap_or_else(|error| panic!("failed to read file {file_id:?}: {error}"))
    }
}

impl Deref for BenchRepository {
    type Target = Repository;

    fn deref(&self) -> &Self::Target {
        self.repository.as_ref()
    }
}

/// Repository-backed wrapper for compiler bench runs.
pub(super) struct BenchWorkspace {
    /// The repository view.
    pub(super) workspace: Arc<BenchRepository>,
    /// The compiler.
    pub(super) compiler: Arc<Compiler>,
    /// The latest diagnostics from one bench compiler operation.
    latest_diagnostics: Mutex<DiagnosticCollection>,
    /// Optional override for the default profile.
    pub(super) default_profile_override: Option<ProfileId>,
}

impl BenchWorkspace {
    /// Create a new in-memory bench workspace.
    pub(super) fn new(workers: u16, inject_prelude: bool, load_libraries: bool) -> Self {
        let root_directory = PathBuf::new();

        // seed a default package name for in memory runs
        let fs = Arc::new(MemoryFileSystem::new());
        fs.create_dir(PathBuf::new().as_path())
            .unwrap_or_else(|_| panic!("failed to initialize memory file system root"));
        fs.add_file("package.json", br#"{ \"name\": \"bench\" }"#)
            .unwrap_or_else(|_| panic!("failed to add package.json to memory file system"));

        let cache_store: Arc<dyn CacheStore> = Arc::new(MemoryCacheStore::new());
        let repository = Arc::new(
            Repository::open_root_from_fs(root_directory.clone(), fs)
                .expect("failed to import repository from compiler bench file system")
                .with_cache_store(cache_store),
        );
        let workspace = Arc::new(BenchRepository::new(repository.clone(), root_directory));

        let compiler_options = CompilerOptions {
            workers,
            inject_prelude,
            load_libraries,
            ..CompilerOptions::default()
        };
        let compiler = Arc::new(Compiler::new(repository, compiler_options));

        Self {
            workspace,
            compiler,
            latest_diagnostics: Mutex::new(DiagnosticCollection::new()),
            default_profile_override: None,
        }
    }

    /// Override the default profile with an explicit lib set.
    pub(super) fn with_profile_libs(mut self, libs: &[&str]) -> Self {
        let default_profile = self.workspace.profile(self.default_profile_id_for_root());
        let mut key = default_profile.key.clone();
        key.lib = libs.iter().map(|lib| (*lib).to_string()).collect();
        let profile_id = self.compiler.remember_profile_key(key);
        self.default_profile_override = Some(profile_id);
        self
    }

    /// Mutate compiler options for this run.
    pub(super) fn with_options_mut<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut CompilerOptions),
    {
        let compiler = Arc::get_mut(&mut self.compiler)
            .unwrap_or_else(|| panic!("compiler options are already shared"));
        f(&mut compiler.options);
        self
    }

    /// Get the default profile id for the root module.
    pub(super) fn default_profile_id_for_root(&self) -> ProfileId {
        let revision = self.workspace.current_revision();
        let root_module_id = self.workspace.root_module_id();

        self.default_profile_override.unwrap_or_else(|| {
            self.compiler
                .context(revision)
                .unwrap_or_else(|message| panic!("{message}"))
                .default_profile_id_for_module(root_module_id)
        })
    }

    /// Get the default profile id for a module.
    pub(super) fn default_profile_id(&self, module_id: ModuleId) -> ProfileId {
        let revision = self.workspace.current_revision();

        self.default_profile_override.unwrap_or_else(|| {
            self.compiler
                .context(revision)
                .unwrap_or_else(|message| panic!("{message}"))
                .default_profile_id_for_module(module_id)
        })
    }

    /// Enqueue Import task for a module.
    pub(super) fn import_module(&self, module: ModuleId) {
        self.enqueue(ArtifactKey::dir_base(module));
    }

    /// Enqueue Analyze task for a module.
    pub(super) fn analyze_module(&self, module: ModuleId) {
        let profile = self.default_profile_id(module);
        self.enqueue(ArtifactKey::dir_analyzed(module, profile));
    }

    /// Resolve the language environment for the default root profile.
    pub(super) fn resolve_language_environment(&self) {
        let profile = self.default_profile_id_for_root();
        let revision = self.workspace.current_revision();
        self.compiler
            .run_to_completion(revision, |compiler, _context| {
                compiler.require_language_environment(profile)
            })
            .unwrap_or_else(|error| panic!("failed to resolve language environment: {error:?}"));

        // publish diagnostics from this compiler operation into the bench harness
        self.replace_latest_diagnostics(self.current_workspace_diagnostics());
    }

    /// Resolve builtin libs for the default root profile.
    pub(super) fn resolve_libs(&self) {
        let profile = self.default_profile_id_for_root();
        let revision = self.workspace.current_revision();
        self.compiler
            .run_to_completion(revision, |compiler, _context| {
                compiler.require_library_environment(profile)
            })
            .unwrap_or_else(|error| panic!("failed to resolve libs: {error:?}"));

        // publish diagnostics from this compiler operation into the bench harness
        self.replace_latest_diagnostics(self.current_workspace_diagnostics());
    }

    /// Enqueue one artifact key.
    pub(super) fn enqueue<T: Into<ArtifactKey>>(&self, artifact_key: T) {
        self.compiler.enqueue(artifact_key);
    }

    /// Run all queued tasks to completion with a custom timeout.
    pub(super) fn compile_with_timeout(&self, timeout: Duration) {
        // spawn the compile thread
        let compiler = self.compiler.clone();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            compiler.compile();
            let _ = tx.send(());
        });

        match rx.recv_timeout(timeout) {
            Ok(()) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => {
                panic!("compile timed out after {timeout:?}");
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                panic!("compile thread panicked");
            }
        }

        // publish diagnostics from this compiler run into the bench harness
        self.replace_latest_diagnostics(self.current_workspace_diagnostics());
    }

    /// Run all queued tasks to completion.
    pub(super) fn compile(&self) {
        let timeout = Duration::from_secs(10);
        self.compile_with_timeout(timeout);
    }

    /// Replace the latest compiler diagnostics for this bench harness.
    fn replace_latest_diagnostics(&self, diagnostics: DiagnosticCollection) {
        let mut latest_diagnostics = self
            .latest_diagnostics
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        *latest_diagnostics = diagnostics;
    }

    /// Return the latest compiler diagnostics for this bench harness.
    fn diagnostics(&self) -> DiagnosticCollection {
        self.latest_diagnostics
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    /// Collect the current workspace diagnostics from module artifact families.
    fn current_workspace_diagnostics(&self) -> DiagnosticCollection {
        let revision = self.workspace.current_revision();
        let module_ids = self
            .workspace
            .workspace_module_ids(revision)
            .unwrap_or_else(|error| panic!("failed to read workspace modules: {error}"));
        let mut diagnostics = DiagnosticCollection::new();

        // current workspace families
        for module_id in module_ids {
            let profile_id = self.workspace.default_profile_id_for_module(module_id);
            diagnostics.merge_from(
                &self
                    .workspace
                    .module_artifact_diagnostics(revision, module_id, profile_id),
            );
        }

        diagnostics
    }

    /// Check that no diagnostics up to and including the given phase are present.
    pub(super) fn check_no_diagnostics_up_to_including_phase(&self, phase: TaskPhase) {
        let phases: Vec<TaskPhase> = TaskPhase::all()
            .take_while(|p| p.code() <= phase.code())
            .collect();
        self.check_no_diagnostics_for_phases(&phases);
    }

    /// Check no diagnostics of at least the given severity.
    pub(super) fn check_no_diagnostic(&self, min_severity: DiagnosticSeverity) {
        let diagnostics = self.diagnostics();
        let highest = diagnostics.highest_severity();
        if let Some(highest) = highest
            && highest >= min_severity
        {
            let options = PrintOptions::new()
                .with_line_width(self.workspace.formatter.line_width as u32)
                .with_module_count(self.workspace.tracked_module_count());
            destack_source::print_diagnostics(&self.workspace.files, &diagnostics, options);
            let severity_name = min_severity.family_name().to_ascii_lowercase();
            panic!(
                "program has {} unexpected {severity_name}s",
                diagnostics.len()
            );
        }
    }

    /// Check that no diagnostics for the given phases are present.
    pub(super) fn check_no_diagnostics_for_phases(&self, phases: &[TaskPhase]) {
        let prefixes: Vec<String> = phases
            .iter()
            .flat_map(|phase| {
                let letter = phase.letter();
                [format!("E{letter}"), format!("W{letter}")]
            })
            .collect();

        let diagnostics = self.diagnostics();
        let diagnostic_vec = diagnostics.iter();
        let has_matching = diagnostic_vec
            .iter()
            .any(|d| prefixes.iter().any(|prefix| d.code.starts_with(prefix)));

        if has_matching {
            let matching_diagnostics = destack_source::DiagnosticCollection::from_diagnostics(
                diagnostic_vec
                    .iter()
                    .filter(|d| prefixes.iter().any(|prefix| d.code.starts_with(prefix)))
                    .cloned()
                    .collect(),
            );
            let options = PrintOptions::new()
                .with_line_width(self.workspace.formatter.line_width as u32)
                .with_module_count(self.workspace.tracked_module_count());
            destack_source::print_diagnostics(
                &self.workspace.files,
                &matching_diagnostics,
                options,
            );
            let phase_names: Vec<&str> = phases.iter().map(|p| p.name()).collect();
            panic!("unexpected diagnostics for phases: {phase_names:?}");
        }
    }
}
