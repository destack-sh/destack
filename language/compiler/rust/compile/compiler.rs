use dyst_ast as ast;
use dyst_ast::StringPool;
use dyst_package::{DocumentBody, Workspace};
use dyst_session::Session;

use dyst_dir::{Dumper, DumperOptions, NodeTree};

use crate::AstNodeId;

/// The options for compiling a Workspace.
#[derive(Debug, Clone, Default)]
pub struct CompilerOptions {}

/// A compiler for a Workspace of Dyst sources on Dyst DIR.
#[derive(Debug, Clone)]
pub struct Compiler<'s> {
    /// The node tree of the compiled DIR.
    pub tree: NodeTree,
    /// The string pool.
    pub strings: StringPool,
    /// The options for compiling the Workspace.
    pub options: CompilerOptions,

    /// The workspace to compile.
    pub workspace: &'s Workspace,
    /// The session to compile with.
    pub session: &'s Session,
}

impl<'s> Compiler<'s> {
    /// Create a new compiler.
    pub fn new(workspace: &'s Workspace, options: CompilerOptions) -> Self {
        Self {
            tree: NodeTree::new(),
            strings: StringPool::new(),
            options,
            workspace,
            session: &workspace.session,
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
        // get the document
        let document = self
            .workspace
            .get_document_by_id(node.source_id)
            .unwrap_or_else(|| panic!("document not found"));
        // get the AST
        let DocumentBody::Text { ast, .. } = &document.body else {
            panic!("document is not a text document");
        };
        let node = ast.get(node.id);
        (ast, node)
    }
}
