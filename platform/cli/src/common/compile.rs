use std::fmt;
use std::io::Read;
use std::sync::Arc;

use destack_compiler::{AnalyzeTask, CompileOptions, Compiler, GenerateTask, LintTask};
use destack_source::{DiagnosticOptions, FileType, ModuleId, Uri};
use destack_workspace::{Program, Session};

use crate::common::{DiagnosticArgs, InputArgs, InputSource, ProgramArgs, print_diagnostics};
use crate::console;

/// How deeply to compile modules.
#[derive(Debug, Clone, Default)]
pub enum CompileMode {
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
pub struct CompileContext {
    pub session: Arc<Session>,
    pub program: Arc<Program>,
    pub compiler: Compiler,
    pub diagnostic_options: DiagnosticOptions,
    pub mode: CompileMode,
}

impl fmt::Debug for CompileContext {
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

impl CompileContext {
    /// Create a new compilation context for type checking.
    pub fn for_check(program_args: &ProgramArgs, diagnostic_args: &DiagnosticArgs) -> Self {
        Self::new(program_args, diagnostic_args, CompileMode::Check)
    }

    /// Create a new compilation context for linting.
    pub fn for_lint(program_args: &ProgramArgs, diagnostic_args: &DiagnosticArgs) -> Self {
        Self::new(program_args, diagnostic_args, CompileMode::Lint)
    }

    /// Create a new compilation context for building a target.
    pub fn for_build(
        program_args: &ProgramArgs,
        diagnostic_args: &DiagnosticArgs,
        target: String,
    ) -> Self {
        Self::new(program_args, diagnostic_args, CompileMode::Build { target })
    }

    /// Create a new compilation context with the given mode.
    pub fn new(
        program_args: &ProgramArgs,
        diagnostic_args: &DiagnosticArgs,
        mode: CompileMode,
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
            program.clone(),
            CompileOptions {
                diagnostic: diagnostic_options.clone(),
                workers: program_args.workers,
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
        match input.to_sources() {
            Ok(sources) => Ok(sources),
            Err(e) => {
                if input.files.is_empty() && input.eval.is_empty() && !input.stdin {
                    console::error("error: no input provided");
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
            CompileMode::Check => {
                self.compiler.enqueue(AnalyzeTask::AnalyzeModule { module });
            }
            CompileMode::Lint => {
                self.compiler.enqueue(LintTask::LintModule { module });
            }
            CompileMode::Build { target } => {
                self.compiler.enqueue(GenerateTask::GenerateModule {
                    module,
                    target: target.clone(),
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
        drop(self.compiler);
        CompileResult {
            program: self.program,
            diagnostic_options: self.diagnostic_options,
        }
    }
}

/// Result of compilation, ready for diagnostics and output.
pub struct CompileResult {
    /// The program.
    pub program: Arc<Program>,
    /// The diagnostic options.
    pub diagnostic_options: DiagnosticOptions,
}

impl fmt::Debug for CompileResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompileResult")
            .field("program", &self.program)
            .field("diagnostic_options", &self.diagnostic_options)
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
