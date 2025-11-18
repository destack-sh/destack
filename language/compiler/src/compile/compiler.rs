use dyst_dir::Session;
use dyst_source::{Diagnostic, DiagnosticOptions, FileId};

use crate::{
    BuildOptions, CompileDiagnostic, CompileError, CompileWarning, CompilerQueue, ExecuteOptions,
    ImportOptions, ImportTask, LinkOptions, LowerOptions, OptimizeOptions, ResolveOptions,
    ValidateOptions,
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
pub struct Compiler<'s> {
    /// The session.
    pub session: &'s Session<'s>,
    /// The options for compiling.
    pub options: CompileOptions,
    /// The queue of compiler tasks.
    pub(super) queue: CompilerQueue,
}

#[allow(clippy::too_many_arguments)]
impl<'s> Compiler<'s> {
    /// Create a new Compiler.
    pub fn new(session: &'s Session<'s>) -> Self {
        Self {
            session,
            options: CompileOptions::default(),
            queue: CompilerQueue::new(),
        }
    }

    /// Create a new Compiler from a single module/file.
    pub fn from_file(session: &'s Session<'s>, file_id: FileId, options: CompileOptions) -> Self {
        let mut compiler = Self {
            session,
            options,
            queue: CompilerQueue::new(),
        };
        compiler.enqueue(ImportTask::ImportFileFromId { file_id }.into());
        compiler
    }

    /// Add an error to the compiler.
    pub fn error<T: Into<CompileError>>(&mut self, error: T) {
        let error: CompileError = error.into();
        let diagnostic: CompileDiagnostic = error.into();
        let diagnostic: Diagnostic = diagnostic.to_diagnostic(self.session);
        if let Some(diagnostic) = self.options.diagnostic.map(diagnostic) {
            self.session.diagnostics.insert(diagnostic);
        }
    }

    /// Add a warning to the compiler.
    pub fn warning<T: Into<CompileWarning>>(&mut self, warning: T) {
        let warning: CompileWarning = warning.into();
        let diagnostic: CompileDiagnostic = warning.into();
        let diagnostic: Diagnostic = diagnostic.to_diagnostic(self.session);
        if let Some(diagnostic) = self.options.diagnostic.map(diagnostic) {
            self.session.diagnostics.insert(diagnostic);
        }
    }
}
