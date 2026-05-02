use std::cell::RefCell;
use std::fmt;
use std::io::Read;
use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_compiler::Compiler;
use destack_linter::Linter;
use destack_session::{FileChange, Session, SessionEventHandler};
use destack_source::{DiagnosticCollection, FileType, ModuleId, ProfileId, TargetId};
use destack_workspace::{Ref, Repository, Revision};

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
    /// Private command session for this compilation.
    pub session: Session,
    /// The compile mode.
    pub mode: CompilerMode,
    /// The requested root artifacts.
    root_artifact_keys: RefCell<Vec<ArtifactKey>>,
    /// The modules explicitly queued by the CLI command.
    queued_modules: RefCell<Vec<ModuleId>>,
}

impl fmt::Debug for CompilerContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompileContext")
            .field("repository", &self.repository)
            .field("session", &"Session { ... }")
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
        event_handler: SessionEventHandler,
    ) -> Self {
        Self::new(program_args, diagnostic_args, mode, Some(event_handler))
    }

    /// Create a new compilation context with the given mode.
    pub fn new(
        program_args: &ProgramArgs,
        _diagnostic_args: &DiagnosticArgs,
        mode: CompilerMode,
        event_handler: Option<SessionEventHandler>,
    ) -> Self {
        let repository = program_args.setup();

        let compiler = Arc::new(Compiler::new(repository.clone()));
        let reference = destack_workspace::Ref::for_workspace_root(repository.workspace_root());
        let revision = repository
            .current(&reference)
            .expect("cli workspace revision should exist");
        let session = Session::fork(
            repository.workspace_root().to_path_buf(),
            program_args.effective_cwd(),
            repository.clone(),
            Ref::new(format!("cli:{}", repository.workspace_root().display())),
            revision,
            compiler.clone(),
            Arc::new(Linter::new(repository.clone())),
            program_args.workers as usize,
            event_handler,
        )
        .expect("cli session should initialize");

        Self {
            repository,
            session,
            mode,
            root_artifact_keys: RefCell::new(Vec::new()),
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
            InputSource::File(path) => self
                .session
                .load_module_from_fs(self.session.head(), path)
                .map_err(|e| CliError::message(format!("{e:?}"))),
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
        let mut root_artifact_keys = self.root_artifact_keys.borrow_mut();

        match &self.mode {
            CompilerMode::Check => {
                let profile = self.module_profile_id(revision, module)?;
                root_artifact_keys.push(ArtifactKey::dir_checked(module, profile));
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
                let target_id = TargetId::new(package_id, target);
                let profile = self.target_profile_id(revision, module, target_id)?;
                root_artifact_keys.push(ArtifactKey::mir_lowered(module, profile, target_id));
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
                let target_id = TargetId::new(package_id, target);
                root_artifact_keys.push(ArtifactKey::module_output(module, target_id));
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
        // provide all requested roots
        let artifact_keys = self.root_artifact_keys.borrow().clone();
        let revision = self.current_revision()?;
        self.session
            .provide(revision, &artifact_keys)
            .map_err(|error| CliError::message(error.to_string()))?;
        let diagnostics = self.collect_module_diagnostics(revision)?;

        Ok(CompileResult {
            repository: self.repository,
            revision,
            diagnostics,
        })
    }

    /// Run the compiler without consuming self.
    /// Useful when you need to access program/modules after compilation.
    pub fn run_compile(&self) -> CliResult<()> {
        let artifact_keys = self.root_artifact_keys.borrow().clone();
        let revision = self.current_revision()?;
        self.session
            .provide(revision, &artifact_keys)
            .map_err(|error| CliError::message(error.to_string()))?;

        Ok(())
    }

    /// Convert to a CompileResult, consuming self.
    pub fn into_result(self) -> CliResult<CompileResult> {
        let revision = self.current_revision()?;
        let diagnostics = self.collect_module_diagnostics(revision)?;

        Ok(CompileResult {
            repository: self.repository,
            revision,
            diagnostics,
        })
    }

    /// Return the current workspace revision for the compiler context.
    fn current_revision(&self) -> CliResult<Revision> {
        self.session
            .revision(self.session.head())
            .map_err(|error| CliError::message(error.to_string()))
    }

    /// Materialize one inline CLI input into one command local revision.
    fn materialize_inline_module(
        &self,
        kind: &str,
        name: &str,
        content: &str,
    ) -> CliResult<ModuleId> {
        let extension = name.rsplit('.').next().unwrap_or("ds");
        let file_type = FileType::from_extension_or_unknown(extension);
        let logical_path = cli_input_logical_path(kind, name, file_type);
        let path = self.repository.workspace_root().join(&logical_path);
        self.session
            .apply_file(
                self.session.head(),
                path.as_path(),
                FileChange::Text {
                    content: content.to_string(),
                },
            )
            .map_err(|error| CliError::message(error.to_string()))?;
        let module_id = self
            .session
            .load_module_from_fs(self.session.head(), path.as_path())
            .map_err(|error| CliError::message(format!("{error}")))?;

        Ok(module_id)
    }

    /// Collect diagnostics for the queued modules in the current revision.
    fn collect_module_diagnostics(&self, revision: Revision) -> CliResult<DiagnosticCollection> {
        let diagnostics = self
            .repository
            .diagnostics(revision)
            .map_err(|error| CliError::message(error.to_string()))?;

        Ok(diagnostics)
    }

    /// Return the default profile id for one module.
    fn module_profile_id(&self, revision: Revision, module_id: ModuleId) -> CliResult<ProfileId> {
        let profile = self
            .repository
            .module_profile(revision, module_id)
            .map_err(|error| CliError::message(error.to_string()))?;

        Ok(profile.id())
    }

    /// Return the profile id selected for one module target.
    fn target_profile_id(
        &self,
        revision: Revision,
        module_id: ModuleId,
        target_id: TargetId,
    ) -> CliResult<ProfileId> {
        let profile = self
            .repository
            .module_target_profile(revision, module_id, target_id)
            .map_err(|error| CliError::message(error.to_string()))?;
        let profile = if let Some(profile) = profile {
            profile
        } else {
            self.repository
                .module_profile(revision, module_id)
                .map_err(|error| CliError::message(error.to_string()))?
        };

        Ok(profile.id())
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
    /// The collected diagnostics for this compile result.
    pub diagnostics: DiagnosticCollection,
}

impl fmt::Debug for CompileResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompileResult")
            .field("repository", &self.repository)
            .field("revision", &self.revision)
            .field("diagnostics", &self.diagnostics)
            .finish()
    }
}

impl CompileResult {
    /// Print diagnostics and return exit code.
    pub fn finish(self) -> i32 {
        if let Err(error) = print_diagnostics(&self.repository, self.revision, &self.diagnostics) {
            eprintln!("failed to render diagnostics: {error}");

            return 1;
        }

        self.diagnostics.get_status_code()
    }
}
