use std::fmt;
use std::io::Read;
use std::sync::Arc;

use destack_compiler::{
    AnalyzeTask, Compiler, CompilerEventHandler, CompilerOptions, GenerateTask, LintTask,
    StatsSnapshot,
};
use destack_source::{DiagnosticOptions, FileType, ModuleId, Uri};
use destack_workspace::{Program, Session, TargetId};

use crate::common::{DiagnosticArgs, InputArgs, InputSource, ProgramArgs, print_diagnostics};
use crate::console;

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
    /// Type check + lint rules.
    Lint,
    /// Full build for a target (type check + codegen).
    Build {
        /// The target name to build for.
        target: String,
    },
}

/// Common compilation context shared by check/build/lint commands.
pub struct CompilerContext {
    /// The session.
    pub session: Arc<Session>,
    /// The program.
    pub program: Arc<Program>,
    /// The compiler.
    pub compiler: Compiler,
    /// The diagnostic options.
    pub diagnostic_options: DiagnosticOptions,
    /// The compile mode.
    pub mode: CompilerMode,
}

impl fmt::Debug for CompilerContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompileContext")
            .field("session", &self.session)
            .field("program", &self.program)
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

    /// Create a new compilation context for linting.
    pub fn for_lint(program_args: &ProgramArgs, diagnostic_args: &DiagnosticArgs) -> Self {
        Self::new(program_args, diagnostic_args, CompilerMode::Lint, None)
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
        let session = program_args.setup();

        // get the program from the session (created during setup)
        let program = session
            .programs
            .iter()
            .next()
            .map(|entry| entry.value().clone())
            .expect("session should have a program after setup");

        let compiler = Compiler::new(
            session.clone(),
            program.clone(),
            CompilerOptions {
                diagnostic: diagnostic_options.clone(),
                workers: program_args.workers,
                load_libs: !program_args.no_libs,
                inject_prelude: !program_args.no_prelude,
                follow_imports: !program_args.no_follow_imports,
                event_handler,
                ..Default::default()
            },
        );
        Self {
            session,
            program,
            compiler,
            diagnostic_options,
            mode,
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
    pub fn resolve_source(&self, source: &InputSource) -> Result<ModuleId, String> {
        match source {
            InputSource::File(path) => self
                .compiler
                .resolve_path_to_module(&path.to_path_buf())
                .map_err(|e| format!("{e:?}")),
            InputSource::Inline { code, name } => {
                let uri = Uri::from_string(name);
                let extension = name.rsplit('.').next().unwrap_or("ds");
                let file_type = FileType::from_extension_or_unknown(extension);
                Ok(self
                    .program
                    .register_inline_module(uri, code.clone(), file_type))
            }
            InputSource::Stdin { name } => {
                let mut content = String::new();
                std::io::stdin()
                    .read_to_string(&mut content)
                    .map_err(|e| format!("failed to read stdin: {e}"))?;
                let uri = Uri::from_string(name);
                let extension = name.rsplit('.').next().unwrap_or("ds");
                let file_type = FileType::from_extension_or_unknown(extension);
                Ok(self.program.register_inline_module(uri, content, file_type))
            }
        }
    }

    /// Resolve a source and enqueue it for compilation.
    pub fn enqueue_source(&self, source: &InputSource) -> Result<ModuleId, String> {
        let module_id = self.resolve_source(source)?;
        self.enqueue_module(module_id);
        Ok(module_id)
    }

    /// Enqueue a module for compilation based on the compile mode.
    pub fn enqueue_module(&self, module: ModuleId) {
        match &self.mode {
            CompilerMode::Check => {
                let profile = self.program.default_profile_id_for_module(module);
                self.compiler
                    .enqueue(AnalyzeTask::AnalyzeModule { module, profile });
            }
            CompilerMode::Lint => {
                let profile = self.program.default_profile_id_for_module(module);
                self.compiler
                    .enqueue(LintTask::LintModule { module, profile });
            }
            CompilerMode::Build { target } => {
                let module_ref = self.program.modules.get(module);
                let package_id = module_ref.read().package_id;
                let target_id = TargetId::new(package_id, target);
                self.compiler.enqueue(GenerateTask::GenerateModule {
                    module,
                    target: target_id,
                });
            }
        }
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
    pub fn compile(self) -> CompileResult {
        self.compiler.compile();
        let stats = self
            .compiler
            .stats
            .snapshot_with_program(self.program.modules.len(), Some(&self.program));
        drop(self.compiler);
        CompileResult {
            program: self.program,
            diagnostic_options: self.diagnostic_options,
            stats,
        }
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
            .snapshot_with_program(self.program.modules.len(), Some(&self.program))
    }

    /// Convert to a CompileResult, consuming self.
    pub fn into_result(self) -> CompileResult {
        let stats = self.stats();
        drop(self.compiler);
        CompileResult {
            program: self.program,
            diagnostic_options: self.diagnostic_options,
            stats,
        }
    }
}

/// Result of compilation, ready for diagnostics and output.
pub struct CompileResult {
    /// The program.
    pub program: Arc<Program>,
    /// The diagnostic options.
    pub diagnostic_options: DiagnosticOptions,
    /// Compilation statistics.
    pub stats: StatsSnapshot,
}

impl fmt::Debug for CompileResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompileResult")
            .field("program", &self.program)
            .field("diagnostic_options", &self.diagnostic_options)
            .field("stats", &self.stats)
            .finish()
    }
}

impl CompileResult {
    /// Print diagnostics and return exit code.
    pub fn finish(self) -> i32 {
        let diagnostics = self
            .program
            .diagnostics
            .collect()
            .map(&self.diagnostic_options);
        print_diagnostics(&self.program, &diagnostics);
        diagnostics.get_status_code()
    }
}
