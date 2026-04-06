use std::cell::RefCell;
use std::fmt;
use std::io::Read;
use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_compiler::{Compiler, CompilerEventHandler, CompilerOptions, StatsSnapshot};
use destack_source::{DiagnosticCollection, DiagnosticOptions, FileType, ModuleId};
use destack_workspace::{Change, Edit, Ref, Repository, RepositorySnapshot, Revision};

use crate::common::{DiagnosticArgs, InputArgs, InputSource, ProgramArgs, print_diagnostics};
use crate::console;
use crate::error::{CliError, CliResult};

/// Print helpful message when no input is provided.
pub fn print_no_input_help(command: &str) {
    console::error("error: no input files provided");
    eprintln!();
    eprintln!("{}:", console::bold("Usage"));
    eprintln!("  destack {command} <FILES>...       Process files or directories");
    eprintln!("  destack {command} -e '<CODE>'      Process inline code");
    eprintln!("  destack {command} --stdin          Process code from stdin");
    eprintln!();
    eprintln!("{}:", console::bold("Examples"));
    eprintln!("  destack {command} src/             Process all files in src/");
    eprintln!("  destack {command} *.ts             Process all TypeScript files");
    eprintln!("  destack {command} -e 'const x = 1' Process inline expression");
    eprintln!("  echo 'const x = 1' | destack {command} --stdin");
    eprintln!();
    eprintln!(
        "For more information, try '{}'",
        console::dim(&format!("destack {command} --help"))
    );
}

/// How deeply to compile modules.
#[derive(Debug, Clone, Default)]
pub enum CompilerMode {
    /// Type check only (parse, bind, resolve, analyze).
    #[default]
    Check,
    /// Lower DIR to MIR for a target.
    Lower {
        /// The target name to lower for.
        target: String,
    },
    /// Full build for a target (type check + codegen).
    Build {
        /// The target name to build for.
        target: String,
    },
}

/// Common compilation context shared by check/build/lint commands.
pub struct CompilerContext {
    /// The repository.
    pub repository: Arc<Repository>,
    /// The compiler.
    pub compiler: Compiler,
    /// The diagnostic options.
    pub diagnostic_options: DiagnosticOptions,
    /// The compile mode.
    pub mode: CompilerMode,
    /// The pinned command local snapshot when inputs materialize extra source.
    active_snapshot: RefCell<Option<RepositorySnapshot>>,
    /// The modules explicitly queued by the CLI command.
    queued_modules: RefCell<Vec<ModuleId>>,
}

impl fmt::Debug for CompilerContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompileContext")
            .field("repository", &self.repository)
            .field("compiler", &"Compiler { ... }")
            .field("diagnostic_options", &self.diagnostic_options)
            .field("mode", &self.mode)
            .finish()
    }
}

impl CompilerContext {
    /// Create a new compilation context for type checking.
    pub fn for_check(program_args: &ProgramArgs, diagnostic_args: &DiagnosticArgs) -> Self {
        Self::new(program_args, diagnostic_args, CompilerMode::Check, None)
    }

    /// Create a new compilation context for building a target.
    pub fn for_build(
        program_args: &ProgramArgs,
        diagnostic_args: &DiagnosticArgs,
        target: String,
    ) -> Self {
        Self::new(
            program_args,
            diagnostic_args,
            CompilerMode::Build { target },
            None,
        )
    }

    /// Create a new compilation context for lowering to MIR.
    pub fn for_run(
        program_args: &ProgramArgs,
        diagnostic_args: &DiagnosticArgs,
        target: String,
    ) -> Self {
        Self::new(
            program_args,
            diagnostic_args,
            CompilerMode::Lower { target },
            None,
        )
    }

    /// Create a new compilation context with progress reporting.
    pub fn with_progress(
        program_args: &ProgramArgs,
        diagnostic_args: &DiagnosticArgs,
        mode: CompilerMode,
        event_handler: CompilerEventHandler,
    ) -> Self {
        Self::new(program_args, diagnostic_args, mode, Some(event_handler))
    }

    /// Create a new compilation context with the given mode.
    pub fn new(
        program_args: &ProgramArgs,
        diagnostic_args: &DiagnosticArgs,
        mode: CompilerMode,
        event_handler: Option<CompilerEventHandler>,
    ) -> Self {
        let diagnostic_options: DiagnosticOptions = diagnostic_args.clone().into();
        let repository = program_args.setup();

        let compiler = Compiler::new(
            repository.clone(),
            CompilerOptions {
                diagnostic: diagnostic_options.clone(),
                workers: program_args.workers,
                load_libraries: !program_args.no_libs,
                inject_prelude: !program_args.no_prelude,
                follow_imports: !program_args.no_follow_imports,
                timings: program_args.timings,
                event_handler,
                ..Default::default()
            },
        );
        Self {
            repository,
            compiler,
            diagnostic_options,
            mode,
            active_snapshot: RefCell::new(None),
            queued_modules: RefCell::new(Vec::new()),
        }
    }

    /// Load sources from input args, returning an error message on failure.
    pub fn load_sources(&self, input: &InputArgs) -> Result<Vec<InputSource>, i32> {
        self.load_sources_for(input, "check")
    }

    /// Load sources from input args with a specific command name for error messages.
    pub fn load_sources_for(
        &self,
        input: &InputArgs,
        command: &str,
    ) -> Result<Vec<InputSource>, i32> {
        match input.to_sources() {
            Ok(sources) => Ok(sources),
            Err(e) => {
                if !input.has_input() {
                    print_no_input_help(command);
                } else {
                    console::error(&format!("error: {e}"));
                }
                Err(1)
            }
        }
    }

    /// Resolve an InputSource to a ModuleId, registering it with the compiler.
    pub fn resolve_source(&self, source: &InputSource) -> CliResult<ModuleId> {
        match source {
            InputSource::File(path) => {
                let revision = self.current_revision()?;

                self.compiler
                    .resolve_path_to_module(revision, &path.to_path_buf())
                    .map_err(|e| CliError::message(format!("{e:?}")))
            }
            InputSource::Inline { code, name } => {
                self.materialize_inline_module("inline", name, code)
            }
            InputSource::Stdin { name } => {
                let mut content = String::new();
                std::io::stdin()
                    .read_to_string(&mut content)
                    .map_err(|e| CliError::message(format!("failed to read stdin: {e}")))?;
                self.materialize_inline_module("stdin", name, &content)
            }
        }
    }

    /// Resolve a source and enqueue it for compilation.
    pub fn enqueue_source(&self, source: &InputSource) -> CliResult<ModuleId> {
        let module_id = self.resolve_source(source)?;
        self.enqueue_module(module_id)?;
        Ok(module_id)
    }

    /// Enqueue a module for compilation based on the compile mode.
    pub fn enqueue_module(&self, module: ModuleId) -> CliResult<()> {
        let revision = self.current_revision()?;

        match &self.mode {
            CompilerMode::Check => {
                let profile = self
                    .repository
                    .default_profile_id_for_module(revision, module)
                    .map_err(|error| CliError::message(error.to_string()))?;
                self.compiler
                    .enqueue(revision, ArtifactKey::dir_analyzed(module, profile));

                // diagnostics
                let diagnostic_target = self
                    .repository
                    .diagnostic_target_for_module(revision, module)
                    .map_err(|error| CliError::message(error.to_string()))?;
                let diagnostic_profile = self
                    .repository
                    .profile_id_for_target_or_default(revision, module, &diagnostic_target)
                    .map_err(|error| CliError::message(error.to_string()))?;
                self.compiler.enqueue(
                    revision,
                    ArtifactKey::mir_optimized(module, diagnostic_profile, diagnostic_target),
                );
            }
            CompilerMode::Lower { target } => {
                let module_ref = self
                    .repository
                    .module(revision, module)
                    .map_err(|error| {
                        CliError::message(format!("failed to read module snapshot: {error}"))
                    })?
                    .ok_or_else(|| {
                        CliError::message(format!("missing module snapshot for {module:?}"))
                    })?;
                let package_id = module_ref.package_id;
                let target_id = self.repository.intern_target_id(package_id, target);
                let profile = self
                    .repository
                    .profile_id_for_target_or_default(revision, module, &target_id)
                    .map_err(|error| CliError::message(error.to_string()))?;
                self.compiler
                    .enqueue(revision, ArtifactKey::mir_base(module, profile, target_id));
            }
            CompilerMode::Build { target } => {
                let module_ref = self
                    .repository
                    .module(revision, module)
                    .map_err(|error| {
                        CliError::message(format!("failed to read module snapshot: {error}"))
                    })?
                    .ok_or_else(|| {
                        CliError::message(format!("missing module snapshot for {module:?}"))
                    })?;
                let package_id = module_ref.package_id;
                let target_id = self.repository.intern_target_id(package_id, target);
                self.compiler
                    .enqueue(revision, ArtifactKey::module_output(module, target_id));
            }
        }

        // keep one stable module list for post-compile diagnostics
        let mut queued_modules = self.queued_modules.borrow_mut();
        if !queued_modules.contains(&module) {
            queued_modules.push(module);
        }

        Ok(())
    }

    /// Resolve and enqueue all sources for compilation.
    pub fn enqueue_sources(&self, sources: &[InputSource]) -> Result<Vec<ModuleId>, i32> {
        let mut modules = Vec::new();
        for source in sources {
            match self.enqueue_source(source) {
                Ok(module_id) => modules.push(module_id),
                Err(e) => {
                    console::error(&format!("error: {e}"));
                    return Err(1);
                }
            }
        }
        Ok(modules)
    }

    /// Shorthand: load sources from input args and enqueue them.
    pub fn enqueue(&self, sources: &[InputSource]) -> Result<Vec<ModuleId>, i32> {
        self.enqueue_sources(sources)
    }

    /// Run the compiler.
    pub fn compile(self) -> CliResult<CompileResult> {
        // compile all queued work
        self.compiler.compile();
        let revision = self.current_revision()?;
        let diagnostics = self.collect_module_diagnostics(revision)?;
        let stats = self
            .compiler
            .stats
            .snapshot_with_repository(self.module_count_for_stats(), Some(&self.repository));
        drop(self.compiler);

        Ok(CompileResult {
            repository: self.repository,
            revision,
            diagnostic_options: self.diagnostic_options,
            diagnostics,
            stats,
        })
    }

    /// Run the compiler without consuming self.
    /// Useful when you need to access program/modules after compilation.
    pub fn run_compile(&self) {
        self.compiler.compile();
    }

    /// Create a compile result (for diagnostics).
    /// Get stats snapshot (before consuming the compiler).
    pub fn stats(&self) -> StatsSnapshot {
        self.compiler
            .stats
            .snapshot_with_repository(self.module_count_for_stats(), Some(&self.repository))
    }

    /// Convert to a CompileResult, consuming self.
    pub fn into_result(self) -> CliResult<CompileResult> {
        let revision = self.current_revision()?;
        let diagnostics = self.collect_module_diagnostics(revision)?;
        let stats = self.stats();
        drop(self.compiler);

        Ok(CompileResult {
            repository: self.repository,
            revision,
            diagnostic_options: self.diagnostic_options,
            diagnostics,
            stats,
        })
    }

    /// Return the current workspace revision for the compiler context.
    fn current_revision(&self) -> CliResult<Revision> {
        if let Some(snapshot) = self.active_snapshot.borrow().as_ref() {
            return Ok(snapshot.revision());
        }

        let reference = Ref::for_workspace_root(self.repository.workspace_root());
        let revision = self.repository.current(&reference).map_err(|error| {
            CliError::message(format!("failed to resolve current revision: {error}"))
        })?;
        let snapshot = self.repository.snapshot(revision).map_err(|error| {
            CliError::message(format!("failed to pin current revision: {error}"))
        })?;

        *self.active_snapshot.borrow_mut() = Some(snapshot.clone());

        Ok(snapshot.revision())
    }

    /// Materialize one inline CLI input into one command local revision.
    fn materialize_inline_module(
        &self,
        kind: &str,
        name: &str,
        content: &str,
    ) -> CliResult<ModuleId> {
        let base_revision = self.current_revision()?;
        let extension = name.rsplit('.').next().unwrap_or("ds");
        let file_type = FileType::from_extension_or_unknown(extension);
        let logical_path = cli_input_logical_path(kind, name, file_type);
        let change = Change::single(Edit::set_text(&logical_path, content));
        let revision = self
            .repository
            .apply_to_revision(base_revision, change)
            .map_err(|error| CliError::message(error.to_string()))?;
        let path = self.repository.workspace_root().join(&logical_path);
        let module_id = self
            .repository
            .module_id_for_path(revision, &path)
            .map_err(|error| CliError::message(error.to_string()))?
            .ok_or_else(|| CliError::message(format!("missing module for input {name}")))?;

        let snapshot = self
            .repository
            .snapshot(revision)
            .map_err(|error| CliError::message(format!("failed to pin input revision: {error}")))?;
        *self.active_snapshot.borrow_mut() = Some(snapshot);

        Ok(module_id)
    }

    /// Return the module count used for stats reporting.
    fn module_count_for_stats(&self) -> usize {
        let Ok(revision) = self.current_revision() else {
            return 0;
        };

        self.repository
            .workspace_module_ids(revision)
            .map(|modules| modules.len())
            .unwrap_or(0)
    }

    /// Collect diagnostics for the queued modules in the current revision.
    fn collect_module_diagnostics(&self, revision: Revision) -> CliResult<DiagnosticCollection> {
        let mut diagnostics = DiagnosticCollection::new();

        for module_id in self.queued_modules.borrow().iter().copied() {
            let profile_id = self
                .repository
                .default_profile_id_for_module(revision, module_id)
                .map_err(|error| CliError::message(error.to_string()))?;
            diagnostics.merge_from(
                &self
                    .repository
                    .module_artifact_diagnostics(revision, module_id, profile_id),
            );
        }

        Ok(diagnostics)
    }
}

/// Build one stable logical path for one CLI source input.
fn cli_input_logical_path(kind: &str, name: &str, file_type: FileType) -> String {
    let extension = file_type.extension().unwrap_or("txt");
    let sanitized_name = sanitize_cli_input_name(name);

    format!(".destack/cli/{kind}/{sanitized_name}.{extension}")
}

/// Sanitize one CLI input label for use in a logical path.
fn sanitize_cli_input_name(name: &str) -> String {
    let mut sanitized = String::new();

    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            sanitized.push(character.to_ascii_lowercase());
        } else if matches!(character, '/' | '\\' | '-' | '_' | '.') {
            sanitized.push('_');
        }
    }

    if sanitized.is_empty() {
        sanitized.push_str("input");
    }

    sanitized
}

/// Result of compilation, ready for diagnostics and output.
pub struct CompileResult {
    /// The repository.
    pub repository: Arc<Repository>,
    /// The revision used for compilation and diagnostics.
    pub revision: Revision,
    /// The diagnostic options.
    pub diagnostic_options: DiagnosticOptions,
    /// The collected diagnostics for this compile result.
    pub diagnostics: DiagnosticCollection,
    /// Compilation statistics.
    pub stats: StatsSnapshot,
}

impl fmt::Debug for CompileResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompileResult")
            .field("repository", &self.repository)
            .field("revision", &self.revision)
            .field("diagnostic_options", &self.diagnostic_options)
            .field("diagnostics", &self.diagnostics)
            .field("stats", &self.stats)
            .finish()
    }
}

impl CompileResult {
    /// Print diagnostics and return exit code.
    pub fn finish(self) -> i32 {
        let diagnostics = self.diagnostics.map(&self.diagnostic_options);
        print_diagnostics(&self.repository, self.revision, &diagnostics);
        diagnostics.get_status_code()
    }
}
