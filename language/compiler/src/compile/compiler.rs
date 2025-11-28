use std::num::NonZero;
use std::sync::Arc;
use std::thread;

use dyst_dir::Program;
use dyst_source::{DiagnosticCollector, DiagnosticOptions};

use crate::{
    BuildOptions, CompileDiagnostic, CompileWarning, ExecuteOptions, ImportOptions, LinkOptions,
    LowerOptions, OptimizeOptions, ResolveOptions, TaskError, TaskQueue, ValidateOptions,
};

/// Get the default number of worker threads (available parallelism, or 1 if unknown).
pub fn default_workers() -> u16 {
    thread::available_parallelism()
        .unwrap_or(NonZero::new(1).unwrap())
        .get() as u16
}

/// The options for compiling a Workspace.
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// The diagnostic options.
    pub diagnostic: DiagnosticOptions,
    /// The number of worker threads to use.
    pub workers: u16,
    /// The options for importing.
    pub import: ImportOptions,
    /// The options for evaluating.
    pub resolve: ResolveOptions,
    /// The options for validating.
    pub validate: ValidateOptions,
    /// The options for lowering.
    pub lower: LowerOptions,
    /// The options for executing.
    pub execute: ExecuteOptions,
    /// The options for optimizing.
    pub optimize: OptimizeOptions,
    /// The options for building.
    pub build: BuildOptions,
    /// The options for linking.
    pub link: LinkOptions,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            diagnostic: DiagnosticOptions::default(),
            workers: default_workers(),
            import: ImportOptions::default(),
            resolve: ResolveOptions::default(),
            validate: ValidateOptions::default(),
            lower: LowerOptions::default(),
            execute: ExecuteOptions::default(),
            optimize: OptimizeOptions::default(),
            build: BuildOptions::default(),
            link: LinkOptions::default(),
        }
    }
}

/// Compile files and sources into something (via DIR).
#[derive(Debug)]
pub struct Compiler {
    /// The program.
    pub program: Arc<Program>,
    /// The options for compiling.
    pub options: CompileOptions,
    /// The pending compiler diagnostics.
    pub pending_diagnostics: DiagnosticCollector,
    /// The queue of compiler tasks.
    pub(super) queue: TaskQueue,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Create a new Compiler.
    pub fn new(program: Arc<Program>, options: CompileOptions) -> Self {
        Self {
            program,
            options,
            pending_diagnostics: DiagnosticCollector::new(),
            queue: TaskQueue::new(),
        }
    }

    /// Add an error to the compiler.
    pub fn error<T: Into<TaskError>>(&self, error: T) {
        let error: TaskError = error.into();
        let diagnostic: CompileDiagnostic = error.into();
        let diagnostic = diagnostic.to_diagnostic(&self.program);
        self.pending_diagnostics.insert(diagnostic);
    }

    /// Add a warning to the compiler.
    pub fn warning<T: Into<CompileWarning>>(&self, warning: T) {
        let warning: CompileWarning = warning.into();
        let diagnostic: CompileDiagnostic = warning.into();
        let diagnostic = diagnostic.to_diagnostic(&self.program);
        self.pending_diagnostics.insert(diagnostic);
    }

    /// Add a diagnostic to the compiler.
    pub fn diagnostic<T: Into<CompileDiagnostic>>(&self, diagnostic: T) {
        let diagnostic: CompileDiagnostic = diagnostic.into();
        let diagnostic = diagnostic.to_diagnostic(&self.program);
        self.pending_diagnostics.insert(diagnostic);
    }

    /// Flush pending diagnostics into the program.
    pub fn flush_diagnostics(&self) {
        self.program
            .diagnostics
            .take_from(&self.pending_diagnostics);
    }
}
