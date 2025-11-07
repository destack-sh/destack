use dyst_ast::{StringId, StringPool};
use dyst_dir::{Module, ModuleGraph, NodeTree};
use dyst_source::{DiagnosticCollector, File, FileId, LanguageOptions};

use crate::CompilerQueue;

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
    /// The options for compiling the Package.
    pub options: CompilerOptions,

    /// The diagnostic collector.
    pub diagnostics: &'s mut DiagnosticCollector,
    /// The modules.
    pub modules: ModuleGraph,
    /// The compiled DIR node tree.
    pub tree: NodeTree,
    /// The combined string pool.
    pub strings: StringPool,
    /// The queue of compiler messages.
    pub queue: CompilerQueue,
}

#[allow(clippy::too_many_arguments)]
impl<'s> Compiler<'s> {
    pub fn from_file(
        file: File,
        language: LanguageOptions,
        diagnostics: &'s mut DiagnosticCollector,
    ) -> Self {
        todo!("from_file({file:?})")
    }

    pub fn from_module(
        module: Module,
        language: LanguageOptions,
        diagnostics: &'s mut DiagnosticCollector,
    ) -> Self {
        todo!("from_module({module:?})")
    }
}
