use dyst_dir::Session;
use dyst_source::FileId;

use crate::{CompilerQueue, CompilerResult, CompilerTask, LoadTask};

/// The options for compiling a Workspace.
#[derive(Debug, Clone, Default)]
pub struct CompilerOptions {
    /// Default integer width.
    pub default_int_width: u16 = 32,
    /// Default float width.
    pub default_float_width: u16 = 32,
}

/// Compile files and sources into something (via DIR).
/// Includes module loading, parsing, evaluation, validation, execution, and building.
#[derive(Debug)]
pub struct Compiler<'s> {
    /// The session.
    pub session: &'s mut Session<'s>,
    /// The options for compiling.
    pub options: CompilerOptions,
    /// The queue of compiler tasks.
    pub(super) queue: CompilerQueue,
}

#[allow(clippy::too_many_arguments)]
impl<'s> Compiler<'s> {
    /// Create a new Compiler.
    pub fn new(session: &'s mut Session<'s>) -> Self {
        Self {
            session,
            options: CompilerOptions::default(),
            queue: CompilerQueue::new(),
        }
    }

    /// Create a new Compiler from a single file.
    pub fn from_file(
        session: &'s mut Session<'s>,
        file_id: FileId,
        options: CompilerOptions,
    ) -> Self {
        let mut compiler = Self {
            session,
            options,
            queue: CompilerQueue::new(),
        };
        compiler
            .queue
            .push_back(CompilerTask::Load(LoadTask::LoadFileFromMemory { file_id }));
        compiler
    }
}
