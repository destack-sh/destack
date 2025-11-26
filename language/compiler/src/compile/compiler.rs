use std::sync::Arc;

use dyst_dir::Program;
use dyst_source::DiagnosticOptions;
use parking_lot::RwLock;

use crate::{
    BuildOptions, CompileDiagnostic, CompileError, CompileWarning, CompilerQueue, ExecuteOptions,
    ImportOptions, LinkOptions, LowerOptions, OptimizeOptions, ResolveOptions, ValidateOptions,
};

/// The options for compiling a Workspace.
#[derive(Debug, Clone, Default)]
pub struct CompileOptions {
    /// The diagnostic options.
    pub diagnostic: DiagnosticOptions,
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

/// Compile files and sources into something (via DIR).
/// Includes module importing, parsing, evaluation, validation, execution, and building.
#[derive(Debug)]
pub struct Compiler {
    /// The program.
    pub program: Arc<Program>,
    /// The options for compiling.
    pub options: CompileOptions,
    /// The pending compiler diagnostics.
    pub pending_diagnostics: RwLock<Vec<CompileDiagnostic>>,
    /// The queue of compiler tasks.
    pub(super) queue: CompilerQueue,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Create a new Compiler.
    pub fn new(program: Arc<Program>, options: CompileOptions) -> Self {
        Self {
            program,
            options,
            pending_diagnostics: RwLock::new(Vec::new()),
            queue: CompilerQueue::new(),
        }
    }

    /// Add an error to the compiler.
    pub fn error<T: Into<CompileError>>(&self, error: T) {
        let error: CompileError = error.into();
        let diagnostic: CompileDiagnostic = error.into();
        self.pending_diagnostics.write().push(diagnostic);
    }

    /// Add a warning to the compiler.
    pub fn warning<T: Into<CompileWarning>>(&self, warning: T) {
        let warning: CompileWarning = warning.into();
        let diagnostic: CompileDiagnostic = warning.into();
        self.pending_diagnostics.write().push(diagnostic);
    }

    /// Add a diagnostic to the compiler.
    pub fn diagnostic<T: Into<CompileDiagnostic>>(&self, diagnostic: T) {
        let diagnostic: CompileDiagnostic = diagnostic.into();
        self.pending_diagnostics.write().push(diagnostic);
    }

    /// Flush pending diagnostics into the program.
    pub fn flush_diagnostics(&self) {
        let mut diagnostics = self.pending_diagnostics.write();
        for diagnostic in diagnostics.drain(..) {
            let diagnostic = diagnostic.to_diagnostic(&self.program);
            self.program.diagnostics.insert(diagnostic);
        }
    }
}
