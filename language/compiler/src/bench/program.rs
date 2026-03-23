use std::env::current_dir;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use destack_artifact::{ArtifactKey, CacheStore, MemoryCacheStore};
use destack_source::{DiagnosticSeverity, ModuleId, ProfileStamp, ProfileVersion};
use destack_workspace::{ProfileId, Program, Session};

use crate::{Compiler, CompilerOptions, TaskPhase};

/// Program wrapper for compiler bench runs.
pub(super) struct BenchProgram {
    /// The program.
    pub(super) program: Arc<Program>,
    /// The compiler.
    pub(super) compiler: Arc<Compiler>,
    /// Optional override for the default profile.
    pub(super) default_profile_override: Option<ProfileId>,
}

impl BenchProgram {
    /// Create a new in-memory bench program.
    pub(super) fn new(workers: u16, inject_prelude: bool, load_libraries: bool) -> Self {
        let root_directory = current_dir().unwrap_or_else(|error| {
            panic!("failed to read current directory: {error}");
        });

        // seed a default package name for in memory runs
        let fs = Arc::new(destack_source::MemoryFileSystem::new());
        fs.add_file("package.json", br#"{ \"name\": \"bench\" }"#)
            .unwrap_or_else(|_| panic!("failed to add package.json to memory file system"));

        let cache_store: Arc<dyn CacheStore> = Arc::new(MemoryCacheStore::new());
        let session = Arc::new(
            Session::new(root_directory.clone())
                .with_fs(fs)
                .with_cache_store(cache_store),
        );
        let program = session.add_root(root_directory);

        let compiler_options = CompilerOptions {
            workers,
            inject_prelude,
            load_libraries,
            ..CompilerOptions::default()
        };
        let compiler = Arc::new(Compiler::new(
            session.clone(),
            program.clone(),
            compiler_options,
        ));

        Self {
            program,
            compiler,
            default_profile_override: None,
        }
    }

    /// Override the default profile with an explicit lib set.
    pub(super) fn with_profile_libs(mut self, libs: &[&str]) -> Self {
        let default_profile = self.program.profile(self.default_profile_id_for_root());
        let mut key = default_profile.key.clone();
        key.lib = libs.iter().map(|lib| (*lib).to_string()).collect();
        let profile_id = self.program.profiles.get_or_create(key);
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
        self.default_profile_override.unwrap_or_else(|| {
            self.program
                .default_profile_id_for_module(self.program.root_module_id)
        })
    }

    /// Get the default profile id for a module.
    pub(super) fn default_profile_id(&self, module_id: ModuleId) -> ProfileId {
        self.default_profile_override
            .unwrap_or_else(|| self.program.default_profile_id_for_module(module_id))
    }

    /// Get the profile version.
    pub(super) fn profile_version(&self, profile_id: ProfileId) -> ProfileVersion {
        self.program
            .profiles
            .get(profile_id)
            .unwrap_or_else(|| panic!("missing profile data for {profile_id:?}"))
            .version
    }

    /// Get the profile stamp.
    pub(super) fn profile_stamp(&self, profile_id: ProfileId) -> ProfileStamp {
        ProfileStamp::new(profile_id, self.profile_version(profile_id))
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
        self.compiler
            .run_to_completion(|compiler| compiler.require_language_environment(profile))
            .unwrap_or_else(|error| panic!("failed to resolve language environment: {error:?}"));
    }

    /// Resolve builtin libs for the default root profile.
    pub(super) fn resolve_libs(&self) {
        let profile = self.default_profile_id_for_root();
        self.compiler
            .run_to_completion(|compiler| compiler.require_library_environment(profile))
            .unwrap_or_else(|error| panic!("failed to resolve libs: {error:?}"));
    }

    /// Enqueue one artifact key (does not run it).
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
    }

    /// Run all queued tasks to completion.
    pub(super) fn compile(&self) {
        let timeout = Duration::from_secs(10);
        self.compile_with_timeout(timeout);
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
        let diagnostics = self.program.diagnostics.collect();
        let highest = diagnostics.highest_severity();
        if let Some(highest) = highest
            && highest >= min_severity
        {
            let options = destack_source::PrintOptions::new()
                .with_line_width(self.program.formatter.line_width as u32)
                .with_module_count(self.program.modules.len());
            destack_source::print_diagnostics(&self.program.files, &diagnostics, options);
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

        let diagnostics = self.program.diagnostics.collect();
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
            let options = destack_source::PrintOptions::new()
                .with_line_width(self.program.formatter.line_width as u32)
                .with_module_count(self.program.modules.len());
            destack_source::print_diagnostics(&self.program.files, &matching_diagnostics, options);
            let phase_names: Vec<&str> = phases.iter().map(|p| p.name()).collect();
            panic!("unexpected diagnostics for phases: {phase_names:?}");
        }
    }
}
