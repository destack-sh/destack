use dyst_ast::{self as ast, StringId, StringPool};
use dyst_package::{FileContent, Package, SourceFile, Workspace};
use dyst_session::Session;

use dyst_dir::{Dumper, DumperOptions, NodeTree};
use dyst_source::SourceId;

use crate::AstNodeId;

/// The options for compiling a Workspace.
#[derive(Debug, Clone, Default)]
pub struct CompilerOptions {
    /// Default integer width.
    pub default_int_width: u16 = 32,
    /// Default float width.
    pub default_float_width: u16 = 32,
}

/// A compiler for a Dyst package containing related Dyst sources.
#[derive(Debug, Clone)]
pub struct Compiler<'s> {
    /// The workspace we're in.
    pub workspace: &'s Workspace,
    /// The session we're in.
    pub session: &'s Session,
    /// The package we're compiling.
    pub package: &'s Package,

    /// The node tree of the compiled DIR.
    pub tree: NodeTree,
    /// The string pool.
    pub strings: StringPool,
    /// The options for compiling the Package.
    pub options: CompilerOptions,
    /// Whether the compiler has been finalized.
    pub is_finalized: bool,
}

#[allow(clippy::too_many_arguments)]
impl<'s> Compiler<'s> {
    /// Create a new compiler.
    pub fn new(workspace: &'s Workspace, package: &'s Package, options: CompilerOptions) -> Self {
        Self {
            workspace,
            session: &workspace.session,
            package,

            tree: NodeTree::new(),
            strings: StringPool::new(),
            options,
            is_finalized: false,
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
        ast::NodeTree: ast::NodeTreeStore<T>,
    {
        let document = self
            .workspace
            .get_file_by_source_id(node.source_id)
            .unwrap_or_else(|| panic!("document not found: {node:?}"));
        if let FileContent::Source(SourceFile { ast, .. }) = &document.content {
            let node = ast.get(node.id);
            (ast, node)
        } else {
            panic!("node is not in a text document: {node:?}");
        }
    }

    // Intern an AST string for a certain source.
    pub fn intern_string(&mut self, source_id: SourceId, string_id: StringId) -> StringId {
        let document = self
            .workspace
            .get_file_by_source_id(source_id)
            .unwrap_or_else(|| panic!("document not found: {source_id:?}"));
        match &document.content {
            FileContent::Source(SourceFile { strings, .. }) => {
                let string = strings.get(string_id);
                self.strings.intern(string)
            }
            FileContent::Binary { .. } => panic!("binary document not supported"),
        }
    }

    /// Finalize the compiler.
    pub fn finalize(&mut self) {
        assert!(!self.is_finalized, "already finalized");
        self.attach_all_annotations();
        self.is_finalized = true;
    }
}
