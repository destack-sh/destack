use std::marker::PhantomData;

use dyst_ast::StringPool;
use dyst_dir::{ModuleGraph, NodeTree};
use dyst_source::{DiagnosticCollector, File, LanguageOptions};

use crate::{CompilerQueue, CompilerTask, LoadTask};

/// The options for compiling a Workspace.
#[derive(Debug, Clone, Default)]
pub struct CompilerOptions {
    /// Default integer width.
    pub default_int_width: u16 = 64,
    /// Default float width.
    pub default_float_width: u16 = 64,
}

/// Compile files and sources into something (via DIR).
/// Includes module loading, parsing, evaluation, validation, execution, and building.
#[derive(Debug)]
pub struct Compiler<'s> {
    /// The language options.
    pub language: LanguageOptions,
    /// The options for compiling.
    pub options: CompilerOptions,

    /// The diagnostic collector.
    pub diagnostics: DiagnosticCollector,
    /// The modules.
    pub modules: ModuleGraph,
    /// The compiled DIR node tree.
    pub tree: NodeTree,
    /// The combined string pool.
    pub strings: StringPool,
    /// The queue of compiler tasks.
    pub(super) queue: CompilerQueue,

    // (will probably use lifetime parameter later)
    _marker: PhantomData<&'s ()>,
}

#[allow(clippy::too_many_arguments)]
impl<'s> Compiler<'s> {
    /// Create a new Compiler from a single file.
    pub fn from_file(file: File, language: LanguageOptions, options: CompilerOptions) -> Self {
        let mut compiler = Self {
            language,
            options,
            diagnostics: DiagnosticCollector::new(),
            modules: ModuleGraph::new(),
            tree: NodeTree::new(),
            strings: StringPool::new(),
            queue: CompilerQueue::new(),
            _marker: PhantomData,
        };
        compiler
            .queue
            .push_back(CompilerTask::Load(LoadTask::LoadFileFromMemory { file }));
        compiler
    }
}
