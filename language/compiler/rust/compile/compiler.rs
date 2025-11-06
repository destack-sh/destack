use dyst_ast::{self as ast, StringId, StringPool};
use dyst_workspace::{FileContent, Package, FileFile, Workspace};

use dyst_dir::{Dumper, DumperOptions, NodeTree};
use dyst_source::{LanguageOptions, FileId};

use crate::{AstNodeId, CompilerQueue};

/// The options for compiling a Workspace.
#[derive(Debug, Clone, Default)]
pub struct CompilerOptions {
    /// Default integer width.
    pub default_int_width: u16 = 64,
    /// Default float width.
    pub default_float_width: u16 = 64,
}

/// The status the compiler is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompilerStatus {
    Parsed,
    Compiling,
    Compiled,
    Finalized,
}

/// A compiler for a Dyst package containing related Dyst sources.
#[derive(Debug, Clone)]
pub struct Compiler<'s> {
    /// The workspace we're in.
    pub workspace: &'s Workspace,
    /// The diagnostic collector.
    pub diagnostics: &'s DiagnosticCollector,
    /// The package we're compiling.
    pub package: &'s Package,
    /// The language options.
    pub language: LanguageOptions,

    /// The node tree of the compiled DIR.
    pub tree: NodeTree,
    /// The string pool.
    pub strings: StringPool,
    /// The options for compiling the Package.
    pub options: CompilerOptions,
    /// The status the compiler is in.
    pub status: CompilerStatus,
    /// The queue of compiler messages.
    pub queue: CompilerQueue,
}

#[allow(clippy::too_many_arguments)]
impl<'s> Compiler<'s> {
    /// Create a new compiler.
    pub fn new(
        workspace: &'s Workspace,
        package: &'s Package,
        language: LanguageOptions,
        options: CompilerOptions,
    ) -> Self {
        Self {
            workspace,
            diagnostics: &workspace.diagnostics,
            package,
            language,
            tree: NodeTree::new(),
            strings: StringPool::new(),
            options,
            status: CompilerStatus::Parsed,
            queue: CompilerQueue::new(),
        }
    }

    /// Create a new Dumper.
    pub fn dumper(&self, options: DumperOptions) -> Dumper<'_> {
        Dumper::new(&self.strings, &self.tree, options)
    }

    /// Resolve an AST node.
    pub fn get_ast_node<T>(&self, node: AstNodeId<T>) -> (&ast::NodeTree, &T)
    where
        T: ast::Node,
        ast::NodeTree: ast::NodeTreeImpl<T>,
    {
        let document = self
            .workspace
            .get_file_by_source_id(node.source_id)
            .unwrap_or_else(|| panic!("document not found: {node:?}"));
        if let FileContent::File(FileFile { ast, .. }) = &document.content {
            let node = ast.get(node.id);
            (ast, node)
        } else {
            panic!("node is not in a text document: {node:?}");
        }
    }

    // Intern an AST string for a certain source.
    pub fn intern_string(&mut self, source_id: FileId, string_id: StringId) -> StringId {
        let document = self
            .workspace
            .get_file_by_source_id(source_id)
            .unwrap_or_else(|| panic!("document not found: {source_id:?}"));
        match &document.content {
            FileContent::File(FileFile { strings, .. }) => {
                let string = strings.get(string_id);
                self.strings.intern(string)
            }
            FileContent::Binary { .. } => panic!("binary document not supported"),
        }
    }

    /// Get an interned string.
    pub fn get_string(&self, string_id: StringId) -> &str {
        self.strings.get(string_id)
    }

    /// Finalize the compiler.
    pub fn finalize(&mut self) {
        assert!(self.status == CompilerStatus::Compiled);
        self.status = CompilerStatus::Finalized;
    }
}
