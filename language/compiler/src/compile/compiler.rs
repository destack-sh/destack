use dyst_dir::Session;
use dyst_source::FileId;

use crate::{
    BuildOptions, CompilerQueue, CompilerTask, ResolveOptions, ExecuteOptions, ImportOptions,
    ImportTask, OptimizeOptions, ValidateOptions,
};

/// The options for compiling a Workspace.
#[derive(Debug, Clone, Default)]
pub struct CompilerOptions {
    /// The options for importing.
    pub import: ImportOptions,
    /// The options for evaluating.
    pub resolve: ResolveOptions,
    /// The options for validating.
    pub validate: ValidateOptions,
    /// The options for executing.
    pub execute: ExecuteOptions,
    /// The options for optimizing.
    pub optimize: OptimizeOptions,
    /// The options for building.
    pub build: BuildOptions,
}

/// Compile files and sources into something (via DIR).
/// Includes module importing, parsing, evaluation, validation, execution, and building.
#[derive(Debug)]
pub struct Compiler<'s> {
    /// The session.
    pub session: &'s Session<'s>,
    /// The options for compiling.
    pub options: CompilerOptions,
    /// The queue of compiler tasks.
    pub(super) queue: CompilerQueue,
}

#[allow(clippy::too_many_arguments)]
impl<'s> Compiler<'s> {
    /// Create a new Compiler.
    pub fn new(session: &'s Session<'s>) -> Self {
        Self {
            session,
            options: CompilerOptions::default(),
            queue: CompilerQueue::new(),
        }
    }

    /// Create a new Compiler from a single file.
    pub fn from_file(session: &'s Session<'s>, file_id: FileId, options: CompilerOptions) -> Self {
        let mut compiler = Self {
            session,
            options,
            queue: CompilerQueue::new(),
        };
        compiler
            .queue
            .push_back(CompilerTask::Import(ImportTask::ImportFileFromId { file_id }));
        compiler
    }
}
