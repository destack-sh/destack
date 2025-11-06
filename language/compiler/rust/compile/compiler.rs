use dyst_ast::{StringId, StringPool};

use dyst_dir::NodeTree;
use dyst_source::{DiagnosticCollector, FileId, LanguageOptions};

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
#[derive(Debug, Clone)]
pub struct Compiler<'s> {
    /// The language options.
    pub language: LanguageOptions,
    /// The options for compiling the Package.
    pub options: CompilerOptions,
    /// The diagnostic collector.
    pub diagnostics: &'s DiagnosticCollector,

    /// The source files.
    // pub files: FileCache,
    /// The compiled DIR node tree.
    pub tree: NodeTree,
    /// The combined string pool.
    pub strings: StringPool,
    /// The queue of compiler messages.
    pub queue: CompilerQueue,
}

#[allow(clippy::too_many_arguments)]
impl<'s> Compiler<'s> {
    // Intern an AST string for a certain source.
    pub fn intern_string(&mut self, file_id: FileId, string_id: StringId) -> StringId {
        // let document = self
        //     .workspace
        //     .get_file_by_file_id(file_id)
        //     .unwrap_or_else(|| panic!("document not found: {file_id:?}"));
        // match &document.content {
        //     FileContent::File(FileFile { strings, .. }) => {
        //         let string = strings.get(string_id);
        //         self.strings.intern(string)
        //     }
        //     FileContent::Binary { .. } => panic!("binary document not supported"),
        // }
        todo!("intern_string({file_id:?}, {string_id:?})")
    }

    /// Get an interned string.
    pub fn get_string(&self, string_id: StringId) -> &str {
        self.strings.get(string_id)
    }
}
